//! Decode broad input formats into the engine's lossless BT.709 SDR contract.
//! The original is never modified; conversions are recorded in each report.
use crate::{
    media::{Media, Rate, probe},
    pipeline::ffmpeg,
    process::{capture, run},
    tools::Tools,
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

#[derive(Debug, Serialize)]
pub struct Conversion {
    pub reason: String,
    pub input_codec: String,
    pub input_pixel_format: String,
    pub target_fps: String,
    pub hdr_to_sdr: bool,
    pub color_assumptions: Vec<String>,
    pub audio: &'static str,
    pub preview_limit_seconds: Option<f64>,
    pub note: &'static str,
}
pub struct Prepared {
    pub path: PathBuf,
    pub media: Media,
    pub conversion: Option<Conversion>,
}
struct Plan {
    video: u64,
    audio: Option<u64>,
    video_start: f64,
    rate: Rate,
    filter: String,
    codec: String,
    pixel: String,
    hdr: bool,
    assumptions: Vec<String>,
}
fn number(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str()?.parse().ok())
        .filter(|n| n.is_finite())
}
fn tag(v: &Value, name: &str) -> String {
    v[name].as_str().unwrap_or("unknown").to_owned()
}
fn plan(data: &Value) -> Result<Plan> {
    let streams = data["streams"]
        .as_array()
        .context("No readable media streams")?;
    let video = streams
        .iter()
        .find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"] != 1)
        .context("No decodable video stream was found")?;
    let audio = streams
        .iter()
        .find(|s| s["codec_type"] == "audio")
        .and_then(|s| s["index"].as_u64());
    let duration = number(&video["duration"]).or_else(|| number(&data["format"]["duration"]));
    if let Some(duration) = duration {
        ensure!(
            duration > 0.0 && duration <= 21600.0,
            "Input duration must be positive and at most six hours"
        );
    }
    ensure!(
        video["width"].as_u64().unwrap_or(0) > 0 && video["height"].as_u64().unwrap_or(0) > 0,
        "Video dimensions are missing"
    );
    let mut assumptions = Vec::new();
    let rate = ["avg_frame_rate", "r_frame_rate"]
        .iter()
        .find_map(|name| video[*name].as_str().and_then(|s| Rate::parse(s).ok()))
        .unwrap_or_else(|| {
            assumptions.push("Frame rate is unknown; using 30 FPS".into());
            Rate {
                numerator: 30,
                denominator: 1,
            }
        });
    let pixel = tag(video, "pix_fmt");
    let rgb = ["rgb", "bgr", "gbr", "pal"]
        .iter()
        .any(|prefix| pixel.starts_with(prefix));
    let sd = video["height"].as_u64().unwrap_or(1080) <= 576;
    let transfer = tag(video, "color_transfer");
    let hdr = matches!(transfer.as_str(), "smpte2084" | "arib-std-b67");
    let matrix = match tag(video, "color_space").as_str() {
        "gbr" | "rgb" => 0,
        "bt709" => 1,
        "fcc" => 4,
        "bt470bg" => 5,
        "smpte170m" => 6,
        "smpte240m" => 7,
        "ycgco" => 8,
        "bt2020nc" => 9,
        "bt2020c" => 10,
        _ if rgb => 0,
        _ => {
            assumptions.push(
                "Missing matrix: assuming BT.601 for SD, BT.709 for HD, or BT.2020 for HDR".into(),
            );
            if hdr {
                9
            } else if sd {
                6
            } else {
                1
            }
        }
    };
    let trc = match transfer.as_str() {
        "bt709" => 1,
        "gamma22" => 4,
        "gamma28" => 5,
        "smpte170m" => 6,
        "smpte240m" => 7,
        "linear" => 8,
        "iec61966-2-1" => 13,
        "bt2020-10" => 14,
        "bt2020-12" => 15,
        "smpte2084" => 16,
        "arib-std-b67" => 18,
        _ => {
            assumptions.push("Missing transfer: assuming sRGB for RGB, BT.709 for YUV".into());
            if rgb { 13 } else { 1 }
        }
    };
    let primaries = match tag(video, "color_primaries").as_str() {
        "bt709" => 1,
        "bt470m" => 4,
        "bt470bg" => 5,
        "smpte170m" => 6,
        "smpte240m" => 7,
        "film" => 8,
        "bt2020" => 9,
        "smpte431" => 11,
        "smpte432" => 12,
        _ => {
            assumptions
                .push("Missing primaries: assuming BT.2020 for HDR, BT.709 otherwise".into());
            if hdr { 9 } else { 1 }
        }
    };
    let range = match tag(video, "color_range").as_str() {
        "pc" => "full",
        "tv" => "limited",
        _ => {
            assumptions
                .push("Missing range: assuming full for RGB/JPEG YUV, limited otherwise".into());
            if rgb || pixel.starts_with("yuvj") {
                "full"
            } else {
                "limited"
            }
        }
    };
    let colors = if hdr {
        format!(
            "zscale=matrixin={matrix}:matrix=gbr:transferin={trc}:transfer=linear:primariesin={primaries}:primaries=709:rangein={range}:range=full:npl=100,format=gbrpf32le,tonemap=tonemap=hable:desat=2,zscale=matrixin=gbr:matrix=709:transferin=linear:transfer=709:primariesin=709:primaries=709:rangein=full:range=limited:dither=error_diffusion,format=yuv420p"
        )
    } else {
        format!(
            "zscale=matrixin={matrix}:matrix=709:transferin={trc}:transfer=709:primariesin={primaries}:primaries=709:rangein={range}:range=limited:dither=error_diffusion,format=yuv420p"
        )
    };
    Ok(Plan {
        video: video["index"]
            .as_u64()
            .context("Missing video stream index")?,
        audio,
        video_start: number(&video["start_time"]).unwrap_or(0.0),
        rate,
        filter: format!(
            "yadif=mode=send_frame:parity=auto:deint=interlaced,scale=w='max(2,round(iw*sar))':h=ih:flags=lanczos,setsar=1,pad=ceil(iw/2)*2:ceil(ih/2)*2,{colors},setpts=PTS-STARTPTS,fps=fps={}:start_time=0:round=near:eof_action=pass",
            rate.ffmpeg()
        ),
        codec: tag(video, "codec_name"),
        pixel,
        hdr,
        assumptions,
    })
}
pub fn prepare(
    tools: &Tools,
    input: &Path,
    work: &Path,
    logs: &Path,
    preview: Option<f64>,
) -> Result<Prepared> {
    let reason = match probe(tools, input) {
        Ok(media) => {
            return Ok(Prepared {
                path: input.to_owned(),
                media,
                conversion: None,
            });
        }
        Err(error) => format!("{error:#}"),
    };
    let bytes = capture(
        Command::new(&tools.ffprobe)
            .args([
                "-v",
                "error",
                "-show_streams",
                "-show_format",
                "-of",
                "json",
            ])
            .arg(input),
    )?;
    let plan = plan(&serde_json::from_slice(&bytes)?)?;
    let video = work.join("import-video.mp4");
    let mut decode = ffmpeg(tools);
    decode.arg("-i").arg(input).args([
        "-map",
        &format!("0:{}", plan.video),
        "-an",
        "-sn",
        "-dn",
        "-vf",
        &plan.filter,
        "-c:v",
        "libx264",
        "-crf",
        "0",
        "-preset",
        "fast",
        "-pix_fmt",
        "yuv420p",
        "-fps_mode",
        "cfr",
        "-color_primaries",
        "bt709",
        "-color_trc",
        "bt709",
        "-colorspace",
        "bt709",
        "-color_range",
        "tv",
        "-video_track_timescale",
        &plan.rate.numerator.to_string(),
        "-map_metadata",
        "-1",
        "-map_chapters",
        "-1",
    ]);
    if let Some(seconds) = preview {
        decode.args(["-t", &format!("{seconds:.9}")]);
    }
    decode.arg(&video);
    run(
        &mut decode,
        "import-video",
        logs,
        Duration::from_secs(21600),
    )?;
    let info = probe(tools, &video).context("Converted video failed timing/color validation")?;
    let path = if let Some(audio) = plan.audio {
        let path = work.join("import.mp4");
        let duration = info.frame_count as f64 / info.rate.fps();
        let mut mux = ffmpeg(tools);
        mux.arg("-i").arg(&video).arg("-copyts").arg("-i").arg(input)
            .args(["-map","0:v:0","-map",&format!("1:{audio}"),"-c:v","copy","-c:a","aac","-b:a","192k","-ar","48000","-ac","2",
                "-af",&format!("asetpts=PTS-({:.9})/TB,aresample=async=1:first_pts=0,apad,atrim=duration={duration:.9}",plan.video_start),
                "-t",&format!("{duration:.9}"),"-map_metadata","-1","-map_chapters","-1","-movflags","+faststart"]).arg(&path);
        run(&mut mux, "import-audio", logs, Duration::from_secs(21600))?;
        std::fs::remove_file(&video)?;
        path
    } else {
        video
    };
    let media = probe(tools, &path).context("Converted media failed validation")?;
    Ok(Prepared {
        path,
        media,
        conversion: Some(Conversion {
            reason,
            input_codec: plan.codec,
            input_pixel_format: plan.pixel,
            target_fps: plan.rate.ffmpeg(),
            hdr_to_sdr: plan.hdr,
            color_assumptions: plan.assumptions,
            audio: if plan.audio.is_some() {
                "First audio stream aligned to video, resampled and encoded to stereo AAC"
            } else {
                "No audio"
            },
            preview_limit_seconds: preview,
            note: "Source unchanged. Import uses lossless H.264 after conversion to 8-bit BT.709 SDR; VFR is resampled, rotation applied, non-square pixels corrected and odd dimensions padded. HDR is tone-mapped, not preserved. Additional audio/subtitle/data tracks are omitted.",
        }),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn accepts_high_depth_hdr_and_high_frame_rates_without_downscaling() {
        let p = plan(&json!({"streams":[{"index":0,"codec_type":"video","codec_name":"hevc","pix_fmt":"yuv420p10le","width":3840,"height":2160,"avg_frame_rate":"240/1","color_transfer":"smpte2084","color_space":"bt2020nc","color_primaries":"bt2020","color_range":"tv"}]})).unwrap();
        assert!(p.hdr && p.filter.contains("tonemap="));
        assert_eq!(p.rate.ffmpeg(), "240/1");
        assert!(!p.filter.contains("1920"));
    }
    #[test]
    fn ignores_cover_art_and_records_missing_color_assumptions() {
        let p = plan(&json!({"streams":[{"index":0,"codec_type":"video","disposition":{"attached_pic":1}},{"index":2,"codec_type":"video","codec_name":"vp9","pix_fmt":"yuv444p","width":641,"height":359,"avg_frame_rate":"0/0","r_frame_rate":"25/1"},{"index":3,"codec_type":"audio"}]})).unwrap();
        assert_eq!((p.video, p.audio), (2, Some(3)));
        assert!(!p.assumptions.is_empty());
        assert!(p.filter.contains("pad=ceil(iw/2)*2"));
        assert_eq!(p.rate.ffmpeg(), "25/1");
    }
}
