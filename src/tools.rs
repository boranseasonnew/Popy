use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub ncnn: PathBuf,
    pub models: PathBuf,
}

impl Tools {
    pub fn root() -> Result<PathBuf> {
        let current = std::env::current_dir()?;
        let executable = std::env::current_exe()?;
        // An extracted portable package owns its state even before first-run
        // setup, and even when launched from a different working directory.
        if let Some(directory) = executable.parent()
            && (directory.join("Setup Tools.cmd").is_file()
                || directory.join("Araclari Kur.cmd").is_file()
                || directory.join(".tools/runtime.json").is_file())
        {
            return Ok(directory.to_owned());
        }
        if std::env::var_os("POPY_TOOLS").is_some() || current.join(".tools/runtime.json").is_file()
        {
            return Ok(current);
        }
        for parent in executable.ancestors().skip(1).take(3) {
            if parent.join(".tools/runtime.json").is_file() {
                return Ok(parent.to_owned());
            }
        }
        Ok(current)
    }
    pub fn load(root: &Path) -> Result<Self> {
        let config = std::env::var_os("POPY_TOOLS")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join(".tools/runtime.json"));
        let mut tools: Self = serde_json::from_reader(File::open(&config).with_context(|| {
            // The interrupted-setup marker is checked before executing tools.
            format!(
                "Araç ayarı yok: {}. scripts/bootstrap.ps1 çalıştırın.",
                config.display()
            )
        })?)?;
        if config
            .parent()
            .is_some_and(|parent| parent.join("setup-incomplete").is_file())
        {
            bail!("Processing tool setup is incomplete. Run Setup Tools again.");
        }
        // Portable packages store paths relative to the config directory.
        let base = config.parent().unwrap_or(root);
        for path in [
            &mut tools.ffmpeg,
            &mut tools.ffprobe,
            &mut tools.ncnn,
            &mut tools.models,
        ] {
            if path.is_relative() {
                *path = base.join(&*path);
            }
        }
        for path in [&tools.ffmpeg, &tools.ffprobe, &tools.ncnn] {
            if !path.is_file() {
                bail!("Araç bulunamadı: {}", path.display());
            }
        }
        if !tools.models.is_dir() {
            bail!("Model klasörü bulunamadı: {}", tools.models.display());
        }
        Ok(tools)
    }
}

pub fn sha256(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Legacy ncnn image readers reject Windows verbatim-path prefixes.
/// Canonicalize for identity checks, then use an ordinary absolute local path.
pub fn external_path(path: &Path) -> Result<PathBuf> {
    let canonical = path.canonicalize()?;
    #[cfg(windows)]
    {
        let text = canonical
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Dosya yolu Unicode olmalı"))?;
        if let Some(tail) = text.strip_prefix(r"\\?\UNC\") {
            return Ok(PathBuf::from(format!(r"\\{tail}")));
        }
        if let Some(tail) = text.strip_prefix(r"\\?\") {
            return Ok(PathBuf::from(tail));
        }
    }
    Ok(canonical)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Model {
    General,
    Anime,
}

impl Model {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "general" => Ok(Self::General),
            "anime" => Ok(Self::Anime),
            _ => bail!("Model general veya anime olmalı."),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::General => "realesrgan-x4plus",
            Self::Anime => "realesr-animevideov3",
        }
    }
    pub fn native_scale(self) -> u32 {
        match self {
            Self::General => 4,
            Self::Anime => 2,
        }
    }
    pub fn stem(self) -> &'static str {
        match self {
            Self::General => "realesrgan-x4plus",
            Self::Anime => "realesr-animevideov3-x2",
        }
    }
    pub fn hashes(self, tools: &Tools) -> Result<serde_json::Value> {
        let mut hashes = serde_json::Map::new();
        for extension in ["bin", "param"] {
            let name = format!("{}.{}", self.stem(), extension);
            hashes.insert(name.clone(), sha256(&tools.models.join(name))?.into());
        }
        Ok(hashes.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_setup_is_not_treated_as_a_ready_runtime() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::current_dir()
            .unwrap()
            .join(format!("target/tool-config-test/{stamp}"));
        std::fs::create_dir_all(root.join(".tools/models")).unwrap();
        std::fs::write(
            root.join(".tools/tool.exe"),
            b"test placeholder, never executed",
        )
        .unwrap();
        let tools = Tools {
            ffmpeg: "tool.exe".into(),
            ffprobe: "tool.exe".into(),
            ncnn: "tool.exe".into(),
            models: "models".into(),
        };
        serde_json::to_writer(
            File::create(root.join(".tools/runtime.json")).unwrap(),
            &tools,
        )
        .unwrap();
        assert!(Tools::load(&root).is_ok());
        std::fs::write(root.join(".tools/setup-incomplete"), b"in progress").unwrap();
        assert!(
            Tools::load(&root)
                .unwrap_err()
                .to_string()
                .contains("incomplete")
        );
    }
}
