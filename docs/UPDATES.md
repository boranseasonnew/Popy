# Signed portable updates

The desktop app checks the public `boranseasonnew/Popy` GitHub Releases API only after the user requests a check. The selected channel controls prerelease inclusion. Versions use semantic-version ordering; equal or older versions are rejected.

The client downloads `popy-update.json` and `popy-update.sig`, verifies the exact manifest bytes with its embedded Ed25519 public key, then validates version, platform, minimum updater version, URL, archive size and SHA-256. Release metadata and archives must use the configured Popy GitHub release path. Downloads are HTTPS-only, bounded and time-limited. There is no GitHub login and no video data in these requests.

After download, the client verifies the ZIP digest, extracts into a new staging directory with path/symlink/size checks, and verifies the package's file manifest. User-data directories are excluded from installation. The Windows helper waits for the parent app to exit, backs up replaced files, installs and restarts. Errors trigger rollback attempts and a result record. The helper is intentionally outside the GUI's video-worker Job Objects.

This is a portable updater: it requires a writable installed app directory and does not replace development checkouts. It does not modify the registry, install drivers or request administrator rights. Backups and staging files are retained under `.tools/updates/`. Authenticode signing is not provided by the Ed25519 update mechanism.

## Publishing a compatible update

Keep the original private signing key outside Git and backups intended for public distribution. The current maintainer copy is in the ignored `.tools/` directory. Preserve it securely: generating a different key will break trust for existing clients. `init` refuses accidental replacement of a pinned key. Never include the private key in releases, issues or logs.

1. Update the Cargo version and release documentation. Keep compatible update contracts or increase the minimum updater version deliberately.
2. Build and validate the app. Run `scripts/package.ps1` after committing the source; its manifest records that revision.
3. Sign the resulting ZIP with the maintainer tool:

```powershell
.\target\release\popy-release.exe sign .tools/update-signing.key dist/Popy-VERSION-windows-x64.zip dist
.\target\release\popy-release.exe verify dist/popy-update.json dist/popy-update.sig dist/Popy-VERSION-windows-x64.zip
```

4. Publish the ZIP, `.sha256`, `popy-update.json` and `popy-update.sig` on the matching `vVERSION` release. Draft releases are invisible to clients. Mark prereleases accurately.

For first-time users, also build `scripts/installer.ps1 -Compiler PATH_TO_ISCC.exe` with official Inno Setup 6.7+ and publish the Setup EXE and its `.sha256`. It packages the same portable files, installs per user without elevation, and leaves generated runtime/preferences/exports on uninstall. The in-app updater continues to use the signed portable ZIP, including for Setup-installed copies.

`popy-release.exe` is a maintainer utility and is not shipped in the end-user ZIP. The app embeds only the public verification key. The signer checks that the supplied private key matches that public key.

## Validation

Unit tests cover altered signatures, downgrades, wrong origins and unsafe Windows paths. `scripts/updater-smoke.ps1` runs the Windows helper in isolated test folders and checks successful replacement, rollback, protected user data and path escape rejection. `-NoRestart` is a test-only helper invocation option; the desktop app does not use it.

For a real release transition, the `--smoke-update-result PATH` desktop test mode uses the same check/download/verify/install path against published releases. Run it only on a disposable installed test copy. It records terminal check failures or installer startup at PATH; the helper records installation and restart under that copy's `.tools/update-result.json`. Test metadata is not a substitute for an actual completed transition.

On 2026-10-30, an isolated local `0.2.0-preview.1` client successfully updated from the public GitHub `v0.2.0-preview.2` release through this route. The restarted GUI, CLI version, all 486 packaged file hashes, runtime configuration, language/theme/companion preferences and an export sentinel were verified. The old client was an internal fixture, not a separately published release. This test used the current Windows machine; it does not replace an independent clean-OS installation test.
