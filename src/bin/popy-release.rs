//! Maintainer-only signing utility. Never included in end-user packages.
use anyhow::{Context, Result, ensure};
use ed25519_dalek::{Signer, SigningKey};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("init") => {
            ensure!(args.len() == 3, "init <private-key-path> <public-key-path>");
            let private = PathBuf::from(&args[1]);
            let public = PathBuf::from(&args[2]);
            let previous = fs::read(&public).unwrap_or_default();
            ensure!(
                private.is_file() || previous.iter().all(|&b| b == 0),
                "Public key is already pinned. Restore the existing private signing key instead of generating a replacement."
            );
            let mut seed = [0u8; 32];
            if private.is_file() {
                let bytes = fs::read(&private)?;
                ensure!(bytes.len() == 32, "Existing private key must be 32 bytes");
                seed.copy_from_slice(&bytes);
            } else {
                getrandom::fill(&mut seed).map_err(|e| anyhow::anyhow!("OS random source: {e}"))?;
                if let Some(parent) = private.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut file = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&private)?;
                file.write_all(&seed)?;
                file.sync_all()?;
            }
            let key = SigningKey::from_bytes(&seed);
            ensure!(
                previous.iter().all(|&b| b == 0) || previous == key.verifying_key().as_bytes(),
                "Private key does not match the pinned public key"
            );
            fs::write(&public, key.verifying_key().as_bytes())?;
            println!(
                "Public key written to {}. Private key stays local.",
                public.display()
            );
        }
        Some("sign") => {
            ensure!(
                args.len() == 4,
                "sign <private-key-path> <zip-path> <output-directory>"
            );
            let bytes: [u8; 32] = fs::read(&args[1])?
                .try_into()
                .map_err(|_| anyhow::anyhow!("Private key must be 32 bytes"))?;
            let key = SigningKey::from_bytes(&bytes);
            ensure!(
                key.verifying_key().as_bytes() == popy::updater::PUBLIC_KEY,
                "Signing key does not match the app's embedded verification key"
            );
            let archive = PathBuf::from(&args[2]);
            let output = PathBuf::from(&args[3]);
            fs::create_dir_all(&output)?;
            let version = env!("CARGO_PKG_VERSION");
            let name = format!("Popy-{version}-windows-x64.zip");
            ensure!(
                archive.file_name().context("Archive name missing")? == name.as_str(),
                "Archive name/version mismatch"
            );
            let manifest = serde_json::json!({"schema_version":1,"version":version,"platform":"windows-x64",
                "min_updater_version":"0.2.0-preview.1","archive_size":fs::metadata(&archive)?.len(),
                "sha256":popy::tools::sha256(&archive)?,
                "url":format!("https://github.com/boranseasonnew/Popy/releases/download/v{version}/{name}")});
            let bytes = serde_json::to_vec_pretty(&manifest)?;
            let signature = key.sign(&bytes).to_bytes();
            fs::write(output.join("popy-update.json"), &bytes)?;
            fs::write(
                output.join("popy-update.sig"),
                signature
                    .iter()
                    .map(|v| format!("{v:02x}"))
                    .collect::<String>(),
            )?;
            println!("Signed update metadata written to {}", output.display());
        }
        Some("verify") => {
            ensure!(
                args.len() == 4,
                "verify <manifest-path> <signature-path> <zip-path>"
            );
            let manifest = popy::updater::verify_signature(
                &fs::read(&args[1])?,
                &fs::read_to_string(&args[2])?,
                popy::updater::PUBLIC_KEY,
            )?;
            let archive = PathBuf::from(&args[3]);
            ensure!(
                fs::metadata(&archive)?.len() == manifest.archive_size
                    && popy::tools::sha256(&archive)? == manifest.sha256,
                "Signed archive does not match"
            );
            println!("Signature and archive verified: {}", manifest.version);
        }
        Some("check") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&popy::updater::check(true)?)?
            );
        }
        _ => anyhow::bail!("Commands: init, sign, verify, check"),
    }
    Ok(())
}
