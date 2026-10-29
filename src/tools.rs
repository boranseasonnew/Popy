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
    pub fn load(root: &Path) -> Result<Self> {
        let config = std::env::var_os("POPY_TOOLS")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join(".tools/runtime.json"));
        let tools: Self = serde_json::from_reader(File::open(&config).with_context(|| {
            format!(
                "Araç ayarı yok: {}. scripts/bootstrap.ps1 çalıştırın.",
                config.display()
            )
        })?)?;
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

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
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
