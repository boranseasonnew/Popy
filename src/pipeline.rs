use crate::{
    media::{Media, probe},
    process::{Stage, gpu_lease, run},
    tools::{Model, Tools, external_path, sha256},
};
use anyhow::{Context, Result, ensure};
use serde_json::json;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const LIMIT: Duration = Duration::from_secs(1800);
pub(crate) const AUTO_COLOR: &str =
    "normalize=blackpt=black:whitept=white:smoothing=24:independence=0:strength=0.35";
pub(crate) const TO_RGB: &str = "zscale=matrixin=709:matrix=gbr:transferin=709:transfer=iec61966-2-1:primariesin=709:primaries=709:rangein=limited:range=full,format=gbrpf32le,format=rgb24";
pub(crate) const TO_709: &str = "format=gbrpf32le,zscale=matrixin=gbr:matrix=709:transferin=iec61966-2-1:transfer=709:primariesin=709:primaries=709:rangein=full:range=limited,format=yuv420p,setsar=1";

pub struct Options {
    pub input: PathBuf,
    pub output: PathBuf,
    pub model: Model,
    pub seconds: f64,
    pub gpu: u32,
    pub tile: u32,
    pub auto_color: bool,
    pub source_rights: String,
}

pub fn benchmark(tools: &Tools, options: Options) -> Result<PathBuf> {
    ensure!(
        options.seconds.is_finite() && (0.1..=10.0).contains(&options.seconds),
        "Deneme süresi 0.1–10 saniye olmalı"
    );
    ensure!(
        (32..=256).contains(&options.tile),
        "Tile 32–256 aralığında olmalı"
    );
    ensure!(
        options.gpu <= 15,
        "GPU indeksi 0–15 aralığında olmalı; CPU'ya sessiz dönüş yapılmaz"
    );
    ensure!(
        !options.source_rights.trim().is_empty(),
        "Kaynak kullanım hakkını --rights ile belirtin"
    );
    let input = external_path(&options.input).context("Girdi dosyası bulunamadı")?;
    let info = probe(tools, &input)?;
    let frames = ((options.seconds * info.rate.fps()).floor() as usize).min(info.frame_count);
    ensure!((2..=300).contains(&frames), "Deneme 2–300 kare içermeli");
    let duration = frames as f64 / info.rate.fps();
    let native = options.model.native_scale();
    let native_width = info.width.checked_mul(native).context("Boyut taşması")?;
    let native_height = info.height.checked_mul(native).context("Boyut taşması")?;
    ensure!(
        native_width <= 3840
            && native_height <= 2160
            && info.width % 2 == 0
            && info.height % 2 == 0,
        "Yerel model çıktısı en fazla 3840×2160 ve girdi boyutları çift sayı olmalı"
    );
    let estimated_bytes = info.width as u64
        * info.height as u64
        * 3
        * (1 + native as u64 * native as u64)
        * frames as u64;
    ensure!(
        estimated_bytes <= 2_000_000_000,
        "Kare dosyası tahmini 2 GB prototip sınırını aşıyor; süreyi kısaltın"
    );
    let model_hashes = options.model.hashes(tools)?;
    let _gpu_lease = gpu_lease(options.gpu)?;
    // create_dir is exclusive: no preexisting directory or user output is overwritten.
    fs::create_dir_all(
        options
            .output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    fs::create_dir(&options.output)
        .context("Çıktı klasörü zaten var; yeni bir --out klasörü seçin")?;
    let output = external_path(&options.output)?;
    fs::create_dir(output.join("logs"))?;
    fs::create_dir(output.join("input-frames"))?;
    fs::create_dir(output.join("ai-frames"))?;
    let started = Instant::now();
    write_json(
        &output.join("job.json"),
        &json!({
            "state":"running", "schema_version":1, "input":input, "source_sha256":sha256(&input)?,
            "source_rights":options.source_rights, "model":options.model.name(), "native_scale":native,
            "output_scale":2, "auto_color":options.auto_color, "frames":frames, "input_media":info,
            "warning":"Araştırma prototipi: kısa kare dizileri diske yazılır; uzun video alfa motoru değildir."
        }),
    )?;
    let result = execute(tools, &options, &input, &info, &output, frames, duration).and_then(|stages| {
            let total_seconds = started.elapsed().as_secs_f64();
            let ai = probe(tools, &output.join("enhanced.mp4"))?;
            let baseline = probe(tools, &output.join("lanczos.mp4"))?;
            verify_output(&info, &ai, frames, duration)?;
            verify_output(&info, &baseline, frames, duration)?;
            let ai_stage_seconds = stages
                .iter()
                .find(|s| s.name == "ai")
                .context("AI ölçümü eksik")?
                .seconds;
            let report = json!({
                "schema_version":1, "state":"completed", "app_version":env!("CARGO_PKG_VERSION"), "app_sha256":sha256(&std::env::current_exe()?)?,
                "unix_time":SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                "source":input, "source_sha256":sha256(&input)?, "source_rights":options.source_rights,
                "input_media":info, "output_media":ai,
                "model":{"name":options.model.name(),"native_scale":native,"output_scale":2,"hashes":model_hashes},
                "runtime":{"ncnn_path":tools.ncnn,"ncnn_sha256":sha256(&tools.ncnn)?,"ffmpeg_sha256":sha256(&tools.ffmpeg)?,
                    "gpu_index":options.gpu,"gpu_identity":"see logs/ai.stderr.log; device-specific support is not certified",
                    "tile":options.tile,"threads":"1:1:1","vram_peak_bytes":null},
                "auto_color":{"enabled":options.auto_color,"method":"temporally smoothed levels plus mild saturation; not grayscale colorization", "filter":if options.auto_color {Some(AUTO_COLOR)}else{None}},
                "measurement":{"frames":frames,"clip_seconds":duration,"ai_seconds":ai_stage_seconds,
                    "ai_input_fps":frames as f64/ai_stage_seconds,"comparison_workflow_seconds":total_seconds,
                    "comparison_workflow_input_fps":frames as f64/total_seconds,
                    "note":"Workflow time includes Lanczos, contact sheet, probes and hashes. AI time includes PNG I/O. Peak RAM is per child process, not summed. GPU VRAM is not measured."},
                "stages":stages,
                "validation":{"resolution":true,"frame_count":true,"rational_fps":true,"duration":true,"audio_presence":true,
                    "aac_payload_matches_source":if info.has_audio {Some(true)}else{None},
                    "audio_timing":"metadata and AAC payload checked; perceptual sync and waveform alignment not certified",
                    "visual_quality":"not scored; inspect comparison.png and moving video"},
                "outputs":{"enhanced":"enhanced.mp4","baseline":"lanczos.mp4","comparison":"comparison.png"}
            });
            write_json(&output.join("report.json"), &report)?;
            write_json(
                &output.join("job.json"),
                &json!({"state":"completed","report":"report.json"}),
            )?;
            fs::write(
                output.join("report.md"),
                format!(
                    "# Popy — yerel teknik deneme\n\nModel: {} (yerel {}× → çıktı 2×).\n\n{} kare / {:.3} saniye kaynak. AI aşaması: {:.2} sn ({:.2} giriş karesi/sn). Tüm karşılaştırma: {:.2} sn.\n\nOtomatik renk: {}. Kaynak hakkı: {}.\n\nKare sayısı, çözünürlük, rasyonel FPS, süre ve ses varlığı doğrulandı. Algılanan kalite, renk doğruluğu ve sesin algısal senkronu henüz onaylanmadı. VRAM ölçülmedi. Süreler soğuk model yüklemesini ve PNG işlemlerini içerir.\n\n![Kaynak, Lanczos, AI](comparison.png)\n\nSütunlar: kaynak (nearest ile aynı boyuta büyütülmüş), Lanczos, AI. Ayrıntılar ve dosya hash'leri report.json içinde.\n",
                    options.model.name(),
                    native,
                    frames,
                    duration,
                    ai_stage_seconds,
                    frames as f64 / ai_stage_seconds,
                    total_seconds,
                    options.auto_color,
                    options.source_rights
                ),
            )?;
            Ok(())
    });
    match result {
        Ok(()) => Ok(output),
        Err(error) => {
            write_json(
                &output.join("job.json"),
                &json!({"state":"failed_or_cancelled","error":format!("{error:#}")}),
            )?;
            Err(error.context(format!("Günlükler korundu: {}", output.display())))
        }
    }
}

fn execute(
    tools: &Tools,
    options: &Options,
    input: &Path,
    info: &Media,
    out: &Path,
    frames: usize,
    duration: f64,
) -> Result<Vec<Stage>> {
    let logs = out.join("logs");
    let mut stages = Vec::new();
    let input_pattern = out.join("input-frames/%08d.png");
    let ai_pattern = out.join("ai-frames/%08d.png");
    let mut extract = ffmpeg(tools);
    extract
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
            "-start_number",
            "1",
        ])
        .arg(&input_pattern);
    stages.push(run(&mut extract, "decode", &logs, LIMIT)?);
    validate_sequence(&out.join("input-frames"), frames, info.width, info.height)?;
    let mut ai = Command::new(&tools.ncnn);
    ai.arg("-i")
        .arg(out.join("input-frames"))
        .arg("-o")
        .arg(out.join("ai-frames"))
        .arg("-m")
        .arg(&tools.models)
        .args([
            "-n",
            options.model.name(),
            "-s",
            &options.model.native_scale().to_string(),
            "-t",
            &options.tile.to_string(),
            "-g",
            &options.gpu.to_string(),
            "-j",
            "1:1:1",
            "-f",
            "png",
        ]);
    stages.push(run(&mut ai, "ai", &logs, LIMIT)?);
    validate_sequence(
        &out.join("ai-frames"),
        frames,
        info.width * options.model.native_scale(),
        info.height * options.model.native_scale(),
    )?;

    let width = info.width * 2;
    let height = info.height * 2;
    let size = enhancement_filter(width, height, true, options.auto_color);
    let baseline_size = enhancement_filter(width, height, false, options.auto_color);

    let mut baseline = ffmpeg(tools);
    baseline
        .arg("-i")
        .arg(input)
        .args(["-map", "0:v:0", "-map", "0:a:0?", "-vf", &baseline_size]);
    encode_options(&mut baseline, frames, duration);
    baseline.arg(out.join("lanczos.partial.mp4"));
    stages.push(run(&mut baseline, "baseline", &logs, LIMIT)?);

    let mut encode = ffmpeg(tools);
    encode
        .args(["-framerate", &info.rate.ffmpeg(), "-start_number", "1"])
        .arg("-i")
        .arg(&ai_pattern)
        .arg("-i")
        .arg(input)
        .args(["-map", "0:v:0", "-map", "1:a:0?", "-vf", &size]);
    encode_options(&mut encode, frames, duration);
    encode.arg(out.join("enhanced.partial.mp4"));
    stages.push(run(&mut encode, "encode", &logs, LIMIT)?);
    // Validate partials before committing either result to its final filename.
    for name in ["lanczos", "enhanced"] {
        let media = probe(tools, &out.join(format!("{name}.partial.mp4")))?;
        verify_output(info, &media, frames, duration)?;
    }
    if info.has_audio {
        let reference_hashes = audio_packet_hashes(tools, input)?;
        for name in ["lanczos", "enhanced"] {
            let actual = audio_packet_hashes(tools, &out.join(format!("{name}.partial.mp4")))?;
            ensure!(
                actual.len() <= reference_hashes.len()
                    && reference_hashes[..actual.len()] == actual,
                "{name}: kopyalanan AAC paketleri kaynakla eşleşmedi"
            );
        }
    }
    for name in ["lanczos", "enhanced"] {
        fs::rename(
            out.join(format!("{name}.partial.mp4")),
            out.join(format!("{name}.mp4")),
        )?;
    }

    let mut comparison = ffmpeg(tools);
    comparison.arg("-i").arg(input).arg("-i").arg(out.join("lanczos.mp4")).arg("-i").arg(out.join("enhanced.mp4"))
        .args(["-filter_complex", &format!("[0:v]{TO_RGB},scale={width}:{height}:flags=neighbor,setsar=1[a];[1:v]{TO_RGB},setsar=1[b];[2:v]{TO_RGB},setsar=1[c];[a][b][c]hstack=inputs=3"),
            "-frames:v", "1", "-update", "1"]).arg(out.join("comparison.png"));
    stages.push(run(&mut comparison, "comparison", &logs, LIMIT)?);
    Ok(stages)
}

pub(crate) fn ffmpeg(tools: &Tools) -> Command {
    let mut command = Command::new(&tools.ffmpeg);
    command.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-n"]);
    command
}

fn audio_packet_hashes(tools: &Tools, input: &Path) -> Result<Vec<String>> {
    let output = Command::new(&tools.ffprobe)
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
            "json",
        ])
        .arg(input)
        .output()?;
    ensure!(output.status.success(), "Ses paketleri hash'lenemedi");
    let data: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let packets = data["packets"]
        .as_array()
        .context("Ses paketleri bulunamadı")?;
    ensure!(!packets.is_empty(), "Ses paketleri boş");
    packets
        .iter()
        .map(|p| {
            p["data_hash"]
                .as_str()
                .map(str::to_owned)
                .context("Ses paketi hash'i eksik")
        })
        .collect()
}

pub(crate) fn enhancement_filter(
    width: u32,
    height: u32,
    from_rgb: bool,
    auto_color: bool,
) -> String {
    let decode = if from_rgb { "format=rgb24" } else { TO_RGB };
    let levels = if auto_color {
        format!(",{AUTO_COLOR}")
    } else {
        String::new()
    };
    let saturation = if auto_color {
        ",eq=saturation=1.08"
    } else {
        ""
    };
    // Actual transfer/matrix conversion, rather than relabeling PNG sRGB as BT.709.
    format!("{decode}{levels},scale={width}:{height}:flags=lanczos,{TO_709}{saturation}")
}

pub(crate) fn encode_options(command: &mut Command, frames: usize, duration: f64) {
    command.args([
        "-frames:v",
        &frames.to_string(),
        "-t",
        &format!("{duration:.9}"),
        "-fps_mode",
        "passthrough",
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
        "-movflags",
        "+faststart",
    ]);
}

pub(crate) fn verify_output(
    source: &Media,
    output: &Media,
    frames: usize,
    duration: f64,
) -> Result<()> {
    ensure!(
        output.width == source.width * 2 && output.height == source.height * 2,
        "Çıktı boyutu hatalı"
    );
    ensure!(
        output.frame_count == frames,
        "Çıktı kare sayısı hatalı: {} != {}",
        output.frame_count,
        frames
    );
    ensure!(output.rate == source.rate, "Çıktı kare hızı değişti");
    ensure!(
        (output.duration - duration).abs() <= 0.002,
        "Video süresi değişti"
    );
    ensure!(
        output.has_audio == source.has_audio,
        "Ses akışı kayboldu veya eklendi"
    );
    if let Some(length) = output.audio_duration {
        let expected = if frames == source.frame_count {
            source.audio_duration.unwrap_or(duration)
        } else {
            duration
        };
        ensure!((length - expected).abs() <= 0.025, "Ses uzunluğu hatalı");
        ensure!(
            output.audio_start.is_some_and(|t| t.abs() < 0.002),
            "Ses başlangıcı değişti"
        );
    }
    Ok(())
}

fn png_dimensions(path: &Path) -> Result<(u32, u32)> {
    use std::io::Read;
    let mut header = [0u8; 24];
    File::open(path)?.read_exact(&mut header)?;
    ensure!(
        &header[..8] == b"\x89PNG\r\n\x1a\n" && &header[12..16] == b"IHDR",
        "Geçersiz PNG: {}",
        path.display()
    );
    Ok((
        u32::from_be_bytes(header[16..20].try_into()?),
        u32::from_be_bytes(header[20..24].try_into()?),
    ))
}

pub(crate) fn validate_sequence(
    directory: &Path,
    frames: usize,
    width: u32,
    height: u32,
) -> Result<()> {
    let count = fs::read_dir(directory)?
        .collect::<std::io::Result<Vec<_>>>()?
        .len();
    ensure!(count == frames, "Kare dizisi eksik/fazla: {count}/{frames}");
    for index in 1..=frames {
        let file = directory.join(format!("{index:08}.png"));
        ensure!(
            png_dimensions(&file)? == (width, height),
            "Kare boyutu yanlış: {}",
            file.display()
        );
    }
    Ok(())
}

pub fn write_json(path: &Path, data: &serde_json::Value) -> Result<()> {
    let temporary = path.with_extension("json.tmp");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)?;
    serde_json::to_writer_pretty(&mut file, data)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, path)?;
    Ok(())
}

/// A reproducible generated fixture, with known rights and an untouched master.
pub fn fixtures(tools: &Tools, output: &Path) -> Result<()> {
    fs::create_dir_all(
        output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    fs::create_dir(output).context("Fixture klasörü zaten var; yeni klasör seçin")?;
    let output = external_path(output)?;
    fs::create_dir(output.join("logs"))?;
    let mut master = ffmpeg(tools);
    master
        .args([
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=1280x720:rate=12:duration=10",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:sample_rate=48000:duration=10",
            "-vf",
            "setparams=range=limited:color_primaries=bt709:color_trc=bt709:colorspace=bt709",
            "-c:v",
            "libx264",
            "-crf",
            "12",
            "-preset",
            "fast",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-color_primaries",
            "bt709",
            "-color_trc",
            "bt709",
            "-colorspace",
            "bt709",
            "-color_range",
            "tv",
            "-movflags",
            "+faststart",
        ])
        .arg(output.join("master-720p.mp4"));
    run(&mut master, "master", &output.join("logs"), LIMIT)?;
    for (name, filter) in [
        ("input-360p.mp4", "scale=640:360:flags=area"),
        ("input-144p.mp4", "scale=256:144:flags=area"),
        (
            "faded-144p.mp4",
            "scale=256:144:flags=area,eq=contrast=0.65:brightness=0.08:saturation=0.45",
        ),
    ] {
        let mut encode = ffmpeg(tools);
        encode
            .arg("-i")
            .arg(output.join("master-720p.mp4"))
            .args([
                "-vf",
                filter,
                "-c:v",
                "libx264",
                "-crf",
                "30",
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
            ])
            .arg(output.join(name));
        run(
            &mut encode,
            name.trim_end_matches(".mp4"),
            &output.join("logs"),
            LIMIT,
        )?;
        probe(tools, &output.join(name))?;
    }
    write_json(
        &output.join("manifest.json"),
        &json!({"generator":"FFmpeg testsrc2 and sine", "rights":"Procedurally generated technical test; no downloaded footage",
        "seconds":10,"fps":"12/1", "note":"Synthetic patterns do not establish quality on faces, real-world texture or natural movement.",
        "files":["master-720p.mp4","input-360p.mp4","input-144p.mp4","faded-144p.mp4"]}),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Media {
        Media {
            width: 256,
            height: 144,
            rate: crate::media::Rate::parse("12/1").unwrap(),
            frame_count: 120,
            duration: 10.0,
            has_audio: true,
            audio_duration: Some(10.0),
            audio_start: Some(0.0),
            color_space: "bt709".into(),
            color_transfer: "bt709".into(),
            first_pts: 0.0,
        }
    }
    #[test]
    fn output_validation_detects_loss_and_timing_errors() {
        let source = sample();
        let mut output = sample();
        output.width *= 2;
        output.height *= 2;
        assert!(verify_output(&source, &output, 120, 10.0).is_ok());
        output.frame_count = 119;
        assert!(verify_output(&source, &output, 120, 10.0).is_err());
        output.frame_count = 120;
        output.has_audio = false;
        assert!(verify_output(&source, &output, 120, 10.0).is_err());
        output.has_audio = true;
        output.audio_start = Some(0.05);
        assert!(verify_output(&source, &output, 120, 10.0).is_err());
    }
}
