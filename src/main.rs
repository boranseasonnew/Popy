use anyhow::{Result, bail, ensure};
use popy::{
    media::probe,
    pipeline::{Options, benchmark, fixtures},
    process::{capture, install_cancel_handler, set_cancel_file},
    tools::{Model, Tools},
};
use serde_json::json;
use std::{collections::BTreeMap, path::PathBuf, process::Command};

const HELP: &str = "Popy · yerel video iyileştirme · 0.2 ön sürüm\n\n\
popy doctor\n\
popy fixtures --out fixtures/generated/test-01\n\
popy inspect --input video.mp4\n\
popy enhance --input video.mp4 --out runs/export-01 [--model general|anime] [--auto-color] [--preview-seconds 3] [--gpu 0] [--tile 64] [--cancel-file cancel.flag]\n\
popy suite --manifest benchmarks/technical.json [--out runs/suite-01 | --dry-run]\n\
popy benchmark --input video.mp4 --out runs/test-01 --model general|anime --seconds 10 --gpu 0 --tile 64 --rights \"source rights\" [--auto-color]\n\n\
Genel model yerel 4×, son çıktı 2×; anime modeli yerel 2×.\n\
Otomatik renk: ton/kontrast + hafif doygunluk; siyah beyaz renklendirme değildir.\n\
H.264 / AAC, CFR, 8-bit BT.709 limited SDR; HDR, VFR ve rotasyon desteklenmiyor.\n\
enhance tam video veya kısa önizleme; benchmark en fazla 10 sn/300 kare.\n\
enhance sınırlı PNG grupları ve geçici kayıpsız segmentler kullanır.\n\
Çıktı klasörü yeni olmalı. Ctrl+C mevcut alt süreci sonlandırır, günlükleri korur.";

fn main() {
    install_cancel_handler();
    if let Err(error) = execute() {
        eprintln!("Popy: {error:#}");
        std::process::exit(1);
    }
}

fn execute() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let command = arguments.next().unwrap_or_else(|| "help".into());
    if matches!(command.as_str(), "--version" | "-V" | "version") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let remaining: Vec<_> = arguments.collect();
    if remaining
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        println!("{HELP}");
        return Ok(());
    }
    let flags = parse_options(remaining)?;
    let root = Tools::root()?;
    if command == "enhance" {
        set_cancel_file(flags.get("cancel-file").map(PathBuf::from));
    }
    if command == "suite" {
        allowed(&flags, &["manifest", "out", "dry-run"])?;
        let manifest = PathBuf::from(required(&flags, "manifest")?);
        if flags.contains_key("dry-run") {
            ensure!(
                !flags.contains_key("out"),
                "--dry-run ile --out birlikte kullanılmaz"
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&popy::suite::preview(&manifest)?)?
            );
        } else {
            let tools = Tools::load(&root)?;
            let output =
                popy::suite::run(&tools, &manifest, &PathBuf::from(required(&flags, "out")?))?;
            println!(
                "Suite tamamlandı: {}",
                output.join("summary.json").display()
            );
        }
        return Ok(());
    }
    let tools = Tools::load(&root)?;
    match command.as_str() {
        "doctor" => {
            ensure!(flags.is_empty(), "doctor seçenek almıyor");
            let version = |path: &std::path::Path, option: &str| -> Result<String> {
                let output = capture(Command::new(path).arg(option))?;
                Ok(String::from_utf8_lossy(&output)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_owned())
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({"tools":tools,
                "ffmpeg_version":version(&tools.ffmpeg,"-version")?,"ffprobe_version":version(&tools.ffprobe,"-version")?,
                "status":"Tools found; GPU support requires a successful model run, not this check.",
                "models":{"general":Model::General.hashes(&tools)?,"anime":Model::Anime.hashes(&tools)?}}))?
            );
        }
        "fixtures" => {
            allowed(&flags, &["out"])?;
            fixtures(&tools, &PathBuf::from(required(&flags, "out")?))?;
        }
        "inspect" => {
            allowed(&flags, &["input"])?;
            let info = probe(&tools, &PathBuf::from(required(&flags, "input")?))?;
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
        "enhance" => {
            allowed(
                &flags,
                &[
                    "input",
                    "out",
                    "model",
                    "auto-color",
                    "preview-seconds",
                    "gpu",
                    "tile",
                    "cancel-file",
                ],
            )?;
            let output = popy::export::enhance(
                &tools,
                popy::export::ExportOptions {
                    input: required(&flags, "input")?.into(),
                    output: required(&flags, "out")?.into(),
                    model: Model::parse(
                        flags.get("model").map(String::as_str).unwrap_or("general"),
                    )?,
                    gpu: flags
                        .get("gpu")
                        .map(String::as_str)
                        .unwrap_or("0")
                        .parse()?,
                    tile: flags
                        .get("tile")
                        .map(String::as_str)
                        .unwrap_or("64")
                        .parse()?,
                    auto_color: flags.contains_key("auto-color"),
                    preview_seconds: flags
                        .get("preview-seconds")
                        .map(|s| s.parse())
                        .transpose()?,
                },
            )?;
            println!("Tamamlandı: {}", output.join("enhanced.mp4").display());
        }
        "benchmark" => {
            allowed(
                &flags,
                &[
                    "input",
                    "out",
                    "model",
                    "seconds",
                    "gpu",
                    "tile",
                    "rights",
                    "auto-color",
                ],
            )?;
            let options = Options {
                input: required(&flags, "input")?.into(),
                output: required(&flags, "out")?.into(),
                model: Model::parse(flags.get("model").map(String::as_str).unwrap_or("general"))?,
                seconds: flags
                    .get("seconds")
                    .map(String::as_str)
                    .unwrap_or("10")
                    .parse()?,
                gpu: flags
                    .get("gpu")
                    .map(String::as_str)
                    .unwrap_or("0")
                    .parse()?,
                tile: flags
                    .get("tile")
                    .map(String::as_str)
                    .unwrap_or("64")
                    .parse()?,
                auto_color: flags.contains_key("auto-color"),
                source_rights: required(&flags, "rights")?.into(),
            };
            let report = benchmark(&tools, options)?;
            println!("Tamamlandı: {}", report.join("report.json").display());
        }
        _ => bail!("Bilinmeyen komut: {command}\n{HELP}"),
    }
    Ok(())
}

fn parse_options(args: Vec<String>) -> Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    let mut index = 0;
    while index < args.len() {
        let Some(key) = args[index].strip_prefix("--") else {
            bail!("Seçenek bekleniyor: {}", args[index]);
        };
        ensure!(
            !key.is_empty() && !result.contains_key(key),
            "Boş veya tekrar eden seçenek: {key}"
        );
        if key == "auto-color" || key == "dry-run" {
            result.insert(key.into(), "true".into());
            index += 1;
        } else {
            ensure!(
                index + 1 < args.len() && !args[index + 1].starts_with("--"),
                "--{key} için değer eksik"
            );
            result.insert(key.into(), args[index + 1].clone());
            index += 2;
        }
    }
    Ok(result)
}
fn required<'a>(flags: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str> {
    flags
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| anyhow::anyhow!("--{key} gerekiyor"))
}
fn allowed(flags: &BTreeMap<String, String>, keys: &[&str]) -> Result<()> {
    for key in flags.keys() {
        ensure!(keys.contains(&key.as_str()), "Bilinmeyen seçenek: --{key}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arguments_are_not_silently_ignored() {
        assert!(parse_options(vec!["--tile".into()]).is_err());
        assert!(
            parse_options(vec!["--gpu".into(), "0".into(), "--gpu".into(), "1".into()]).is_err()
        );
        assert!(
            allowed(
                &parse_options(vec!["--typo".into(), "value".into()]).unwrap(),
                &["out"]
            )
            .is_err()
        );
        assert_eq!(
            parse_options(vec!["--input".into(), "Türkçe yol ; test.mp4".into()]).unwrap()["input"],
            "Türkçe yol ; test.mp4"
        );
    }
}
