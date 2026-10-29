use crate::{
    process::{capture, capture_lines},
    tools::Tools,
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::Path, process::Command, time::Duration};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Rate {
    pub numerator: u32,
    pub denominator: u32,
}
impl Rate {
    pub fn parse(text: &str) -> Result<Self> {
        let (n, d) = text.split_once('/').context("Kare hızı kesir olmalı")?;
        let rate = Self {
            numerator: n.parse()?,
            denominator: d.parse()?,
        };
        ensure!(
            rate.numerator > 0 && rate.denominator > 0,
            "Geçersiz kare hızı"
        );
        Ok(rate)
    }
    pub fn fps(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
    pub fn ffmpeg(self) -> String {
        format!("{}/{}", self.numerator, self.denominator)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Media {
    pub width: u32,
    pub height: u32,
    pub rate: Rate,
    pub frame_count: usize,
    pub duration: f64,
    pub has_audio: bool,
    pub audio_duration: Option<f64>,
    pub audio_start: Option<f64>,
    pub color_space: String,
    pub color_transfer: String,
    pub first_pts: f64,
}

fn number(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}

pub fn probe(tools: &Tools, input: &Path) -> Result<Media> {
    ensure!(input.is_file(), "Video bulunamadı: {}", input.display());
    let metadata = capture(
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
    let data: Value = serde_json::from_slice(&metadata)?;
    let streams = data["streams"].as_array().context("Akış bilgisi yok")?;
    let videos: Vec<_> = streams
        .iter()
        .filter(|s| s["codec_type"] == "video")
        .collect();
    let audios: Vec<_> = streams
        .iter()
        .filter(|s| s["codec_type"] == "audio")
        .collect();
    ensure!(
        videos.len() == 1 && audios.len() <= 1,
        "Tek video ve en fazla bir ses akışı destekleniyor"
    );
    let v = videos[0];
    ensure!(v["codec_name"] == "h264", "Bu sürüm H.264 girdisi bekliyor");
    ensure!(
        v["pix_fmt"] == "yuv420p",
        "Yalnızca 8-bit yuv420p SDR destekleniyor"
    );
    ensure!(
        v["field_order"] == "progressive",
        "Interlaced veya belirsiz alan sırası desteklenmiyor"
    );
    ensure!(
        v["sample_aspect_ratio"] == "1:1",
        "Yalnızca kare piksel destekleniyor"
    );
    ensure!(
        v["color_space"] == "bt709"
            && v["color_transfer"] == "bt709"
            && v["color_primaries"] == "bt709",
        "Renk uzayı açıkça BT.709 SDR olmalı; sessiz renk/HDR dönüşümü yapılmaz"
    );
    ensure!(
        v["color_range"] == "tv",
        "Yalnızca limited-range BT.709 destekleniyor"
    );
    if let Some(side_data) = v["side_data_list"].as_array() {
        ensure!(
            !side_data
                .iter()
                .any(|s| number(&s["rotation"]).is_some_and(|r| r.abs() > 0.01)),
            "Döndürülmüş video bu prototipte desteklenmiyor"
        );
    }
    ensure!(
        v["tags"]["rotate"].as_str().is_none_or(|r| r == "0"),
        "Video rotasyonu desteklenmiyor"
    );
    let rate = Rate::parse(v["r_frame_rate"].as_str().context("FPS eksik")?)?;
    let duration = number(&v["duration"])
        .or_else(|| number(&data["format"]["duration"]))
        .context("Video süresi eksik")?;
    ensure!(
        duration.is_finite() && duration > 0.0 && duration <= 21600.0,
        "Video süresi 0–6 saat aralığında olmalı"
    );
    let mut previous: Option<f64> = None;
    let mut first_pts = None;
    let mut frame_count = 0;
    capture_lines(
        Command::new(&tools.ffprobe)
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_frames",
                "-show_entries",
                "frame=best_effort_timestamp_time",
                "-of",
                "csv=p=0",
            ])
            .arg(input),
        Duration::from_secs(3600),
        |line| {
            if line.is_empty() {
                return Ok(());
            }
            let time: f64 = line
                .split(',')
                .next()
                .context("Zaman damgası eksik")?
                .parse()
                .context("Kare zaman damgası eksik")?;
            ensure!(time.is_finite(), "Kare zaman damgası geçersiz");
            if let Some(last) = previous {
                validate_cfr(&[last, time], rate)?;
            }
            first_pts.get_or_insert(time);
            previous = Some(time);
            frame_count += 1;
            Ok(())
        },
    )?;
    ensure!(frame_count >= 1, "Video karesi bulunamadı");
    let first_pts = first_pts.context("Kare yok")?;
    ensure!(
        first_pts.abs() < 0.002,
        "Sıfırdan farklı video başlangıcı henüz desteklenmiyor"
    );
    let frame_duration = frame_count as f64 / rate.fps();
    ensure!(
        (duration - frame_duration).abs() <= 1.0 / rate.fps() + 0.002,
        "Kare sayısı ve süre tutarsız"
    );
    let (audio_duration, audio_start) = if let Some(a) = audios.first() {
        ensure!(a["codec_name"] == "aac", "Prototip AAC ses bekliyor");
        let start = number(&a["start_time"]).context("Ses başlangıcı eksik")?;
        let length = number(&a["duration"]).context("Ses süresi eksik")?;
        ensure!(
            start.abs() < 0.002,
            "Ses başlangıç ofseti henüz desteklenmiyor"
        );
        ensure!(
            (length - duration).abs() <= 0.1,
            "Ses/video süreleri uyuşmuyor"
        );
        (Some(length), Some(start))
    } else {
        (None, None)
    };
    Ok(Media {
        width: v["width"].as_u64().context("Genişlik eksik")? as u32,
        height: v["height"].as_u64().context("Yükseklik eksik")? as u32,
        rate,
        frame_count,
        duration,
        has_audio: !audios.is_empty(),
        audio_duration,
        audio_start,
        color_space: "bt709".into(),
        color_transfer: "bt709".into(),
        first_pts,
    })
}

pub fn validate_cfr(times: &[f64], rate: Rate) -> Result<()> {
    let step = 1.0 / rate.fps();
    for pair in times.windows(2) {
        if !pair[0].is_finite()
            || !pair[1].is_finite()
            || ((pair[1] - pair[0]) - step).abs() > 0.0001
        {
            bail!("Değişken FPS, eksik veya sırasız kareler desteklenmiyor");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_fps_preserved() {
        let rate = Rate::parse("30000/1001").unwrap();
        assert_eq!(rate.ffmpeg(), "30000/1001");
        assert!(validate_cfr(&[0.0, 0.033367, 0.066733, 0.1001], rate).is_ok());
    }
    #[test]
    fn vfr_and_dropped_frames_rejected() {
        let rate = Rate::parse("30/1").unwrap();
        for times in [
            [0.0, 0.033333, 0.083333],
            [0.0, 0.033333, 0.1],
            [0.0, 0.0, 0.033333],
        ] {
            assert!(validate_cfr(&times, rate).is_err());
        }
    }
    #[test]
    fn invalid_rates_rejected() {
        for text in ["0/0", "30/0", "0/1", "30", "NaN/1"] {
            assert!(Rate::parse(text).is_err());
        }
        assert!(Rate::parse("240/1").is_ok());
        assert!(Rate::parse("1/2").is_ok());
    }
}
