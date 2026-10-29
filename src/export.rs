//! Bounded-frame export. PNGs are removed after each lossless RGB segment.
//! Segments still occupy disk proportional to video length; this is not FFI or
//! zero-copy inference. The ncnn executable reloads its model for each batch.
use crate::{
    media::{Media, probe},
    pipeline::{TO_RGB, enhancement_filter, ffmpeg, validate_sequence, verify_output, write_json},
    process::{ManagedChild, capture_lines, check_cancelled, gpu_lease, is_cancelled, run},
    tools::{Model, Tools, external_path, sha256},
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

const STAGE_LIMIT: Duration = Duration::from_secs(3600);
const FRAME_BUDGET: u64 = 192 * 1024 * 1024;

pub struct ExportOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub model: Model,
    pub gpu: u32,
    pub tile: u32,
    pub auto_color: bool,
    pub preview_seconds: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Progress {
    pub state: String,
    pub stage: String,
    pub completed_frames: usize,
    pub total_frames: usize,
    pub elapsed_seconds: f64,
    pub message: String,
}

fn batch_size(width: u32, height: u32, native_scale: u32) -> usize {
    let per_frame = u64::from(width)
        .saturating_mul(u64::from(height))
        .saturating_mul(3)
        .saturating_mul(1 + u64::from(native_scale).pow(2));
    (FRAME_BUDGET / per_frame.max(1)).clamp(1, 24) as usize
}

pub fn enhance(tools: &Tools, options: ExportOptions) -> Result<PathBuf> {
    ensure!(
        options.gpu <= 15 && (32..=256).contains(&options.tile),
        "GPU 0–15, tile 32–256 olmalı"
    );
    if let Some(seconds) = options.preview_seconds {
        ensure!(
            seconds.is_finite() && (0.1..=10.0).contains(&seconds),
            "Önizleme 0.1–10 saniye olmalı"
        );
    }
    ensure!(options.input.is_file(), "Girdi bulunamadı");
    fs::create_dir_all(
        options
            .output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    fs::create_dir(&options.output).context("Çıktı klasörü zaten var; yeni klasör seçin")?;
    let out = external_path(&options.output)?;
    let logs = out.join("logs");
    fs::create_dir(&logs)?;
    let start = Instant::now();
    let mut progress = Progress {
        state: "running".into(),
        stage: "inspect".into(),
        completed_frames: 0,
        total_frames: 0,
        elapsed_seconds: 0.0,
        message: "Video ve kare zamanları denetleniyor".into(),
    };
    let result = (|| -> Result<()> {
        write_progress(&out, &mut progress, start)?;
        let source = external_path(&options.input)?;
        let source_hash = sha256(&source)?;
        let work = out.join("work");
        fs::create_dir(&work)?;
        fs::create_dir(work.join("segments"))?;
        progress.stage = "prepare".into();
        progress.message = "Input format, color and timing are being prepared".into();
        write_progress(&out, &mut progress, start)?;
        let prepared =
            crate::import::prepare(tools, &source, &work, &logs, options.preview_seconds)?;
        let input = prepared.path;
        let info = prepared.media;
        let native = options.model.native_scale();
        ensure!(
            info.width > 0 && info.height > 0 && info.width % 2 == 0 && info.height % 2 == 0,
            "Girdi boyutları pozitif ve çift olmalı"
        );
        ensure!(
            info.width.checked_mul(native).is_some() && info.height.checked_mul(native).is_some(),
            "Native model dimensions exceed the image encoder's addressable size"
        );
        let frames = options
            .preview_seconds
            .map(|s| (s * info.rate.fps()).floor() as usize)
            .unwrap_or(info.frame_count)
            .min(info.frame_count);
        ensure!(frames >= 1, "No video frames in the selected preview");
        let batch = batch_size(info.width, info.height, native);
        let duration = frames as f64 / info.rate.fps();
        progress.total_frames = frames;
        progress.stage = "prepare".into();
        progress.message = format!(
            "{}×{} → {}×{} · {} kare · grup başına {batch} kare",
            info.width,
            info.height,
            info.width * 2,
            info.height * 2,
            frames
        );
        write_progress(&out, &mut progress, start)?;
        let model_hashes = options.model.hashes(tools)?;
        let _lease = gpu_lease(options.gpu)?;
        // Reserve conservatively for lossless 2× RGB video + the current native
        // batch. Compressed segments are usually smaller; user sees estimate.
        let pixels = u64::from(info.width)
            .checked_mul(u64::from(info.height))
            .context("Image size overflow")?;
        let current_batch = pixels
            .checked_mul(3 * (1 + u64::from(native).pow(2)))
            .context("Batch size overflow")?
            .max(FRAME_BUDGET);
        let estimate = pixels
            .checked_mul(12)
            .and_then(|n| n.checked_mul(frames as u64))
            .and_then(|n| n.checked_add(current_batch))
            .and_then(|n| n.checked_add(256 * 1024 * 1024))
            .context("Temporary disk estimate overflow")?;
        ensure_space(&out, estimate)?;
        let mut decoder = Decoder::start(tools, &input, &info, frames, &logs)?;
        let mut list = File::create(work.join("segments.txt"))?;
        let mut stages = Vec::new();
        let mut index = 0;
        while progress.completed_frames < frames {
            check_cancelled()?;
            let count = batch.min(frames - progress.completed_frames);
            let incoming = work.join("input");
            let outgoing = work.join("ai");
            fs::create_dir(&incoming)?;
            fs::create_dir(&outgoing)?;
            progress.stage = "decode".into();
            progress.message = format!("Grup {} · {count} kare hazırlanıyor", index + 1);
            write_progress(&out, &mut progress, start)?;
            for frame in 1..=count {
                let pixels = decoder.next()?;
                image::save_buffer_with_format(
                    incoming.join(format!("{frame:08}.png")),
                    &pixels,
                    info.width,
                    info.height,
                    image::ColorType::Rgb8,
                    image::ImageFormat::Png,
                )?;
            }
            progress.stage = "upscale".into();
            progress.message = format!(
                "Grup {} · GPU {} üzerinde iyileştiriliyor",
                index + 1,
                options.gpu
            );
            write_progress(&out, &mut progress, start)?;
            let mut ai = Command::new(&tools.ncnn);
            ai.arg("-i")
                .arg(&incoming)
                .arg("-o")
                .arg(&outgoing)
                .arg("-m")
                .arg(&tools.models)
                .args([
                    "-n",
                    options.model.name(),
                    "-s",
                    &native.to_string(),
                    "-t",
                    &options.tile.to_string(),
                    "-g",
                    &options.gpu.to_string(),
                    "-j",
                    "1:1:1",
                    "-f",
                    "png",
                ]);
            stages.push(run(&mut ai, &format!("ai-{index:06}"), &logs, STAGE_LIMIT)?);
            validate_sequence(&outgoing, count, info.width * native, info.height * native)?;
            progress.stage = "segment".into();
            write_progress(&out, &mut progress, start)?;
            let name = format!("segments/{index:06}.mkv");
            let mut encode = ffmpeg(tools);
            encode
                .args([
                    "-framerate",
                    &info.rate.ffmpeg(),
                    "-start_number",
                    "1",
                    "-i",
                ])
                .arg(outgoing.join("%08d.png"))
                .args([
                    "-vf",
                    &format!(
                        "format=rgb24,scale={}:{}:flags=lanczos,setsar=1",
                        info.width * 2,
                        info.height * 2
                    ),
                    "-frames:v",
                    &count.to_string(),
                    "-an",
                    "-c:v",
                    "libx264rgb",
                    "-crf",
                    "0",
                    "-preset",
                    "ultrafast",
                    "-fps_mode",
                    "passthrough",
                ])
                .arg(work.join(&name));
            stages.push(run(
                &mut encode,
                &format!("segment-{index:06}"),
                &logs,
                STAGE_LIMIT,
            )?);
            writeln!(
                list,
                "file '{name}'\nduration {:.12}",
                count as f64 / info.rate.fps()
            )?;
            list.flush()?;
            // Both paths are fixed direct children of this newly-created job's
            // work directory; no user-provided path is ever recursively deleted.
            fs::remove_dir_all(&incoming)?;
            fs::remove_dir_all(&outgoing)?;
            progress.completed_frames += count;
            index += 1;
            write_progress(&out, &mut progress, start)?;
        }
        decoder.finish()?;
        drop(decoder);
        drop(list);
        progress.stage = "encode".into();
        progress.message = "Gruplar birleştiriliyor; renk ayarı ve ses uygulanıyor".into();
        write_progress(&out, &mut progress, start)?;
        let partial = out.join("enhanced.partial.mp4");
        let mut final_encode = ffmpeg(tools);
        final_encode
            .args(["-f", "concat", "-safe", "1", "-i"])
            .arg(work.join("segments.txt"))
            .arg("-i")
            .arg(&input)
            .args([
                "-map",
                "0:v:0",
                "-map",
                "1:a:0?",
                "-vf",
                &format!(
                    "settb=expr={}/{},setpts=N,{}",
                    info.rate.denominator,
                    info.rate.numerator,
                    enhancement_filter(info.width * 2, info.height * 2, true, options.auto_color)
                ),
                "-frames:v",
                &frames.to_string(),
                "-r",
                &info.rate.ffmpeg(),
                "-fps_mode",
                "cfr",
                "-c:v",
                "libx264",
                "-crf",
                "16",
                "-preset",
                "fast",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "copy",
                "-color_primaries",
                "bt709",
                "-color_trc",
                "bt709",
                "-colorspace",
                "bt709",
                "-color_range",
                "tv",
                "-video_track_timescale",
                &info.rate.numerator.to_string(),
                "-movflags",
                "+faststart",
            ]);
        if frames < info.frame_count {
            final_encode.args(["-t", &format!("{duration:.9}")]);
        }
        final_encode.arg(&partial);
        stages.push(run(&mut final_encode, "final-encode", &logs, STAGE_LIMIT)?);
        progress.stage = "validate".into();
        progress.message = "Çözünürlük, kareler, süre ve ses denetleniyor".into();
        write_progress(&out, &mut progress, start)?;
        let actual = probe(tools, &partial)?;
        verify_output(&info, &actual, frames, duration)?;
        if info.has_audio {
            let (actual_count, actual_hash) = audio_signature(tools, &partial, None)?;
            let (source_count, source_signature) = audio_signature(
                tools,
                &input,
                if frames < info.frame_count {
                    Some(actual_count)
                } else {
                    None
                },
            )?;
            ensure!(
                source_count == actual_count && actual_hash == source_signature,
                "AAC paketleri kaynakla eşleşmedi"
            );
        }
        ensure!(
            sha256(&source)? == source_hash,
            "Kaynak dosya işlem sırasında değişti"
        );
        check_cancelled()?;
        // Keep the video as a partial until all core validations have passed.
        fs::rename(&partial, out.join("enhanced.mp4"))?;
        let mut compare = ffmpeg(tools);
        compare.arg("-i").arg(&input).arg("-i").arg(out.join("enhanced.mp4"))
            .args(["-filter_complex", &format!("[0:v]{TO_RGB},scale={}:{}:flags=neighbor,setsar=1[a];[1:v]{TO_RGB},setsar=1[b];[a][b]hstack=inputs=2", info.width * 2, info.height * 2),
                "-frames:v", "1", "-update", "1"]).arg(out.join("comparison.png"));
        stages.push(run(&mut compare, "comparison", &logs, STAGE_LIMIT)?);
        let report = json!({"schema_version":2,"state":"completed","app_version":env!("CARGO_PKG_VERSION"),
            "source":source,"source_sha256":source_hash,"input_media":info,"output_media":actual,
            "import":prepared.conversion,"audio_reference":if prepared.conversion.is_some() {"normalized_import"} else {"original"},
            "model":{"name":options.model.name(),"native_scale":native,"output_scale":2,"hashes":model_hashes},
            "gpu_index":options.gpu,"tile":options.tile,"auto_color":options.auto_color,
            "runtime":{"ncnn_sha256":sha256(&tools.ncnn)?,"ffmpeg_sha256":sha256(&tools.ffmpeg)?,"app_sha256":sha256(&std::env::current_exe()?)?},
            "color_method":"Temporally smoothed levels + mild saturation, applied once after concatenation; not grayscale colorization",
            "engine":{"batch_frames":batch,"batches":index,"decoded_rgb_batch_budget_bytes":FRAME_BUDGET,
                "temporary_disk_estimate_bytes":estimate,"note":"Bounded PNG batches; lossless RGB segments grow with duration; model reloads per batch"},
            "elapsed_seconds":start.elapsed().as_secs_f64(),"stages":stages,
            "validation":{"frames":true,"resolution":true,"rational_fps":true,"duration":true,"source_unchanged":true,
                "aac_payload_matches":if info.has_audio {Some(true)} else {None},"quality_gate_passed":false},
            "outputs":{"video":"enhanced.mp4","comparison":"comparison.png"}});
        write_json(&out.join("report.json"), &report)?;
        fs::write(
            out.join("report.md"),
            format!(
                "# Popy export\n\n{} kare · {:.3} sn · {} → 2×\n\nOtomatik renk: {}. İş: {:.2} sn.\n\nTeknik kontroller geçti. Algısal kalite ve tüm GPU aileleri onaylanmadı.\n\n![Kaynak / Popy](comparison.png)\n",
                frames,
                duration,
                options.model.name(),
                options.auto_color,
                start.elapsed().as_secs_f64()
            ),
        )?;
        fs::remove_dir_all(&work)?;
        progress.state = "completed".into();
        progress.stage = "done".into();
        progress.message = "Video hazır · enhanced.mp4".into();
        write_progress(&out, &mut progress, start)?;
        Ok(())
    })();
    if let Err(error) = result {
        progress.state = if is_cancelled() {
            "cancelled"
        } else {
            "failed"
        }
        .into();
        progress.message = format!("{error:#}");
        let _ = write_progress(&out, &mut progress, start);
        return Err(error.context(format!("Günlükler korundu: {}", out.display())));
    }
    Ok(out)
}

fn write_progress(out: &Path, progress: &mut Progress, start: Instant) -> Result<()> {
    progress.elapsed_seconds = start.elapsed().as_secs_f64();
    write_json(&out.join("job.json"), &serde_json::to_value(progress)?)
}

pub fn audio_signature(
    tools: &Tools,
    input: &Path,
    take: Option<usize>,
) -> Result<(usize, Vec<u8>)> {
    let mut hash = Sha256::new();
    let mut count = 0;
    capture_lines(
        Command::new(&tools.ffprobe)
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_packets",
                "-show_data_hash",
                "sha256",
                "-show_entries",
                "packet=data_hash",
                "-of",
                "csv=p=0",
            ])
            .arg(input),
        Duration::from_secs(3600),
        |line| {
            // AAC priming side-data can add extra CSV fields/lines. Only data_hash
            // fields are consumed, in packet order, with fixed-width hash strings.
            for field in line.split(',') {
                if let Some(digest) = field.strip_prefix("SHA256:") {
                    ensure!(
                        digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()),
                        "Geçersiz ses hash'i"
                    );
                    if take.is_none_or(|max| count < max) {
                        hash.update(digest.as_bytes());
                        count += 1;
                    }
                }
            }
            Ok(())
        },
    )?;
    ensure!(count > 0, "Ses paketleri bulunamadı");
    Ok((count, hash.finalize().to_vec()))
}

struct Decoder {
    managed: ManagedChild,
    frames: mpsc::Receiver<Result<Vec<u8>, String>>,
}
impl Decoder {
    fn start(
        tools: &Tools,
        input: &Path,
        info: &Media,
        frames: usize,
        logs: &Path,
    ) -> Result<Self> {
        let mut command = ffmpeg(tools);
        command
            .arg("-i")
            .arg(input)
            .args([
                "-map",
                "0:v:0",
                "-an",
                "-vf",
                TO_RGB,
                "-frames:v",
                &frames.to_string(),
                "-fps_mode",
                "passthrough",
                "-pix_fmt",
                "rgb24",
                "-f",
                "rawvideo",
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(File::create(logs.join("decoder.stderr.log"))?);
        let mut managed = ManagedChild::spawn(&mut command)?;
        let mut stdout = managed.child.stdout.take().context("Decoder stdout yok")?;
        let bytes = info.width as usize * info.height as usize * 3;
        let (send, receive) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            for _ in 0..frames {
                let mut frame = vec![0; bytes];
                if let Err(error) = stdout.read_exact(&mut frame) {
                    let _ = send.send(Err(error.to_string()));
                    break;
                }
                if send.send(Ok(frame)).is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            managed,
            frames: receive,
        })
    }
    fn next(&mut self) -> Result<Vec<u8>> {
        let start = Instant::now();
        loop {
            check_cancelled()?;
            ensure!(
                start.elapsed() < Duration::from_secs(120),
                "Decoder kare zaman sınırı"
            );
            match self.frames.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(frame)) => return Ok(frame),
                Ok(Err(error)) => bail!("Decoder eksik kare: {error}"),
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    bail!("Decoder beklenmedik şekilde kapandı")
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }
    fn finish(&mut self) -> Result<()> {
        let start = Instant::now();
        loop {
            check_cancelled()?;
            ensure!(
                start.elapsed() < Duration::from_secs(30),
                "Decoder kapanma zaman sınırı"
            );
            if let Some(status) = self.managed.child.try_wait()? {
                ensure!(status.success(), "Decoder başarısız");
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

#[cfg(windows)]
fn ensure_space(path: &Path, estimate: u64) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free = 0;
    let ok = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut free,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    ensure!(ok != 0, "Disk boşluğu sorgulanamadı");
    ensure!(
        free >= estimate,
        "Geçici dosyalar için en az {:.1} GiB boşluk gerekiyor; {:.1} GiB var",
        estimate as f64 / 1073741824.0,
        free as f64 / 1073741824.0
    );
    Ok(())
}
#[cfg(not(windows))]
fn ensure_space(_: &Path, _: u64) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn batches_respect_rgb_disk_budget_even_for_native_4x() {
        for (width, height, scale) in [(256, 144, 2), (960, 540, 4), (1920, 1080, 2)] {
            let batch = batch_size(width, height, scale);
            let bytes = u64::from(width)
                * u64::from(height)
                * 3
                * (1 + u64::from(scale).pow(2))
                * batch as u64;
            assert!((1..=24).contains(&batch));
            assert!(bytes <= FRAME_BUDGET);
        }
    }
}
