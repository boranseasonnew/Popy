//! Explicit, signed updates for the portable Windows app. No background checks.
use anyhow::{Context, Result, bail, ensure};
use ed25519_dalek::{Signature, VerifyingKey};
use reqwest::blocking::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const PUBLIC_KEY: &[u8; 32] = include_bytes!("update_public_key.bin");
const REPOSITORY: &str = "https://github.com/boranseasonnew/Popy/releases/download/";
const MAX_ARCHIVE: u64 = 64 * 1024 * 1024;
const MAX_UNPACKED: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateManifest {
    pub schema_version: u32,
    pub version: String,
    pub platform: String,
    pub min_updater_version: String,
    pub archive_size: u64,
    pub sha256: String,
    pub url: String,
}
#[derive(Clone, Debug)]
pub struct StagedUpdate {
    pub root: PathBuf,
    pub version: String,
    pub files: Vec<String>,
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

fn client(timeout: u64) -> Result<Client> {
    Ok(Client::builder()
        .user_agent(concat!("Popy/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(timeout))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?)
}
fn get_small(client: &Client, url: &str, limit: u64) -> Result<Vec<u8>> {
    let response = client.get(url).send()?.error_for_status()?;
    let mut bytes = Vec::new();
    response.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= limit,
        "Update response exceeded size limit"
    );
    Ok(bytes)
}
fn decode_signature(text: &str) -> Result<Signature> {
    let text = text.trim();
    ensure!(
        text.is_ascii() && text.len() == 128,
        "Invalid update signature encoding"
    );
    let mut bytes = [0u8; 64];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)?;
    }
    Ok(Signature::from_bytes(&bytes))
}
pub fn verify_signature(
    bytes: &[u8],
    signature: &str,
    public: &[u8; 32],
) -> Result<UpdateManifest> {
    ensure!(bytes.len() <= 8192, "Update manifest too large");
    let key = VerifyingKey::from_bytes(public)?;
    ensure!(!key.is_weak(), "Invalid update verification key");
    key.verify_strict(bytes, &decode_signature(signature)?)
        .context("Update signature verification failed")?;
    let manifest: UpdateManifest = serde_json::from_slice(bytes)?;
    ensure!(
        manifest.schema_version == 1 && manifest.platform == "windows-x64",
        "Unsupported update format/platform"
    );
    let target = Version::parse(&manifest.version)?;
    Version::parse(&manifest.min_updater_version)?;
    ensure!(
        (1..=MAX_ARCHIVE).contains(&manifest.archive_size),
        "Invalid update archive size"
    );
    ensure!(
        manifest.sha256.len() == 64 && manifest.sha256.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid archive hash"
    );
    let expected = format!("{REPOSITORY}v{target}/Popy-{target}-windows-x64.zip");
    ensure!(
        manifest.url == expected,
        "Update archive must belong to the Popy release repository"
    );
    Ok(manifest)
}

pub fn verify_manifest(
    bytes: &[u8],
    signature: &str,
    public: &[u8; 32],
    current: &str,
) -> Result<UpdateManifest> {
    let manifest = verify_signature(bytes, signature, public)?;
    let current = Version::parse(current)?;
    ensure!(
        Version::parse(&manifest.version)? > current,
        "Update is not newer than this version"
    );
    ensure!(
        Version::parse(&manifest.min_updater_version)? <= current,
        "This update requires a newer updater"
    );
    Ok(manifest)
}

pub fn check(include_previews: bool) -> Result<Option<UpdateManifest>> {
    let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let client = client(25)?;
    let bytes = get_small(
        &client,
        "https://api.github.com/repos/boranseasonnew/Popy/releases?per_page=20",
        1_048_576,
    )?;
    let mut releases: Vec<Release> = serde_json::from_slice(&bytes)?;
    releases.retain(|r| !r.draft && (include_previews || !r.prerelease));
    let mut candidates: Vec<_> = releases
        .into_iter()
        .filter_map(|r| {
            let version =
                Version::parse(r.tag_name.strip_prefix('v').unwrap_or(&r.tag_name)).ok()?;
            (version > current).then_some((version, r))
        })
        .collect();
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    for (version, release) in candidates {
        let Some(manifest_asset) = release.assets.iter().find(|a| a.name == "popy-update.json")
        else {
            continue;
        };
        let signature_asset = release
            .assets
            .iter()
            .find(|a| a.name == "popy-update.sig")
            .context("Release has no update signature")?;
        for (asset, name) in [
            (manifest_asset, "popy-update.json"),
            (signature_asset, "popy-update.sig"),
        ] {
            ensure!(
                asset.browser_download_url == format!("{REPOSITORY}v{version}/{name}"),
                "Unexpected update metadata URL"
            );
        }
        let bytes = get_small(&client, &manifest_asset.browser_download_url, 8192)?;
        let signature = get_small(&client, &signature_asset.browser_download_url, 1024)?;
        let manifest = verify_manifest(
            &bytes,
            std::str::from_utf8(&signature)?,
            PUBLIC_KEY,
            env!("CARGO_PKG_VERSION"),
        )?;
        ensure!(
            manifest.version == version.to_string(),
            "Release version and signed manifest disagree"
        );
        return Ok(Some(manifest));
    }
    Ok(None)
}

pub fn download(
    root: &Path,
    manifest: &UpdateManifest,
    cancel: &AtomicBool,
    progress: impl Fn(f32),
) -> Result<StagedUpdate> {
    ensure!(
        cfg!(windows),
        "In-app installation currently supports Windows x64 only"
    );
    let updates = root.join(".tools/updates");
    fs::create_dir_all(&updates)?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let directory = updates.join(format!("stage-{stamp}"));
    fs::create_dir(&directory)?;
    let archive = directory.join("update.zip");
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&archive)?;
    let mut response = client(300)?.get(&manifest.url).send()?.error_for_status()?;
    if let Some(size) = response.content_length() {
        ensure!(
            size == manifest.archive_size,
            "Update download length mismatch"
        );
    }
    let mut buffer = [0u8; 65536];
    let mut total = 0u64;
    loop {
        ensure!(!cancel.load(Ordering::Relaxed), "Update download cancelled");
        let count = response.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        ensure!(
            total <= manifest.archive_size && total <= MAX_ARCHIVE,
            "Update download exceeded signed size"
        );
        file.write_all(&buffer[..count])?;
        progress(total as f32 / manifest.archive_size as f32);
    }
    file.sync_all()?;
    drop(file);
    ensure!(total == manifest.archive_size, "Incomplete update download");
    ensure!(
        crate::tools::sha256(&archive)?.eq_ignore_ascii_case(&manifest.sha256),
        "Update archive hash mismatch"
    );
    ensure!(!cancel.load(Ordering::Relaxed), "Update download cancelled");
    extract_verified(&archive, &directory.join("unpacked"), &manifest.version)
}

fn safe_relative(path: &Path) -> Result<()> {
    ensure!(
        !path.as_os_str().is_empty() && !path.is_absolute(),
        "Unsafe archive path"
    );
    for component in path.components() {
        let Component::Normal(part) = component else {
            bail!("Unsafe archive path")
        };
        let text = part.to_str().context("Non-Unicode archive path")?;
        ensure!(
            !text.contains([':', '\\', '\0']) && !text.ends_with(['.', ' ']),
            "Unsafe Windows archive component"
        );
        let stem = text
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        ensure!(
            !matches!(
                stem.as_str(),
                "CON"
                    | "PRN"
                    | "AUX"
                    | "NUL"
                    | "COM1"
                    | "COM2"
                    | "COM3"
                    | "COM4"
                    | "COM5"
                    | "COM6"
                    | "COM7"
                    | "COM8"
                    | "COM9"
                    | "LPT1"
                    | "LPT2"
                    | "LPT3"
                    | "LPT4"
                    | "LPT5"
                    | "LPT6"
                    | "LPT7"
                    | "LPT8"
                    | "LPT9"
            ),
            "Reserved Windows filename"
        );
    }
    Ok(())
}
fn extract_verified(archive: &Path, destination: &Path, version: &str) -> Result<StagedUpdate> {
    let mut zip = zip::ZipArchive::new(File::open(archive)?)?;
    ensure!(zip.len() <= 5000, "Too many update files");
    fs::create_dir(destination)?;
    let mut expanded = 0u64;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        ensure!(!entry.is_symlink(), "Update symlinks are forbidden");
        let relative = entry.enclosed_name().context("Archive path traversal")?;
        safe_relative(&relative)?;
        expanded = expanded
            .checked_add(entry.size())
            .context("Archive size overflow")?;
        ensure!(
            expanded <= MAX_UNPACKED,
            "Expanded update exceeds size limit"
        );
        let target = destination.join(&relative);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }
        fs::create_dir_all(target.parent().context("Missing archive parent")?)?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&target)?;
        let expected = entry.size();
        let copied = std::io::copy(&mut entry.by_ref().take(expected + 1), &mut output)?;
        ensure!(copied == expected, "Unpacked file length mismatch");
    }
    let package = destination.join(format!("Popy-{version}-windows-x64"));
    #[derive(Deserialize)]
    struct Package {
        version: String,
        files: Vec<PackageFile>,
    }
    #[derive(Deserialize)]
    struct PackageFile {
        path: String,
        sha256: String,
        bytes: u64,
    }
    let content: Package = serde_json::from_reader(File::open(package.join("manifest.json"))?)?;
    ensure!(content.version == version, "Package version mismatch");
    ensure!(
        !content.files.is_empty() && content.files.len() <= 5000,
        "Invalid package file count"
    );
    let mut files = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for item in content.files {
        let path = Path::new(&item.path);
        safe_relative(path)?;
        let first = path
            .components()
            .next()
            .context("Missing path")?
            .as_os_str()
            .to_string_lossy()
            .to_ascii_lowercase();
        ensure!(
            !matches!(first.as_str(), ".tools" | "exports" | "runs" | "fixtures"),
            "Update may not overwrite user data"
        );
        ensure!(
            seen.insert(item.path.to_ascii_lowercase()),
            "Duplicate package filename"
        );
        let source = package.join(path);
        ensure!(
            fs::metadata(&source)?.len() == item.bytes
                && crate::tools::sha256(&source)?.eq_ignore_ascii_case(&item.sha256),
            "Package file failed verification"
        );
        files.push(item.path);
    }
    ensure!(
        files.iter().any(|p| p == "popy.exe") && files.iter().any(|p| p == "popy-desktop.exe"),
        "Update lacks app executables"
    );
    files.push("manifest.json".into());
    Ok(StagedUpdate {
        root: package,
        version: version.into(),
        files,
    })
}

pub fn install(root: &Path, staged: &StagedUpdate) -> Result<()> {
    ensure!(cfg!(windows), "Windows updater only");
    let root = crate::tools::external_path(root)?;
    ensure!(
        root.join("popy-desktop.exe").canonicalize()? == std::env::current_exe()?.canonicalize()?,
        "Run the installed portable app to install updates; development checkouts are not replaced"
    );
    let probe = root.join(format!(".popy-write-check-{}", std::process::id()));
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)
        .context("App directory is not writable")?;
    fs::remove_file(probe)?;
    let directory = staged
        .root
        .parent()
        .context("Missing update staging directory")?;
    let config = directory.join("install.json");
    let helper = directory.join("install.ps1");
    crate::pipeline::write_json(
        &config,
        &serde_json::json!({"root":root,"staged":staged.root,"version":staged.version,"files":staged.files,"parent_pid":std::process::id()}),
    )?;
    fs::write(&helper, include_str!("../scripts/update-install.ps1"))?;
    let powershell = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:\\Windows"))
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut command = Command::new(powershell);
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&helper)
        .arg("-Config")
        .arg(&config);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    // Intentionally independent of the GUI's worker Job Objects: the helper
    // must survive app exit to replace executables and start the new version.
    command
        .spawn()
        .context("Update installer could not start")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    #[test]
    fn update_signature_rejects_tampering_downgrades_and_other_origins() {
        let key = SigningKey::from_bytes(&[17; 32]); // Test-only, never a publisher key.
        let manifest = UpdateManifest {
            schema_version: 1,
            version: "0.3.0".into(),
            platform: "windows-x64".into(),
            min_updater_version: "0.1.0".into(),
            archive_size: 123,
            sha256: "a".repeat(64),
            url: format!("{REPOSITORY}v0.3.0/Popy-0.3.0-windows-x64.zip"),
        };
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let sign = |bytes: &[u8]| {
            key.sign(bytes)
                .to_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        assert!(
            verify_manifest(
                &bytes,
                &sign(&bytes),
                key.verifying_key().as_bytes(),
                "0.2.0"
            )
            .is_ok()
        );
        let mut changed = bytes.clone();
        changed[0] = b' ';
        assert!(
            verify_manifest(
                &changed,
                &sign(&bytes),
                key.verifying_key().as_bytes(),
                "0.2.0"
            )
            .is_err()
        );
        assert!(
            verify_manifest(
                &bytes,
                &sign(&bytes),
                key.verifying_key().as_bytes(),
                "0.3.0"
            )
            .is_err()
        );
        let mut other = manifest;
        other.url = "https://example.com/popy.zip".into();
        let bytes = serde_json::to_vec(&other).unwrap();
        assert!(
            verify_manifest(
                &bytes,
                &sign(&bytes),
                key.verifying_key().as_bytes(),
                "0.2.0"
            )
            .is_err()
        );
    }
    #[test]
    fn rejects_windows_archive_escape_paths() {
        for path in [
            "../escape",
            "C:/evil",
            "folder/file:stream",
            "folder/CON.txt",
            "folder/trailing.",
            "folder\\..\\..\\evil",
        ] {
            assert!(safe_relative(Path::new(path)).is_err(), "accepted {path}");
        }
        assert!(safe_relative(Path::new("licenses/font/OFL.txt")).is_ok());
    }
}
