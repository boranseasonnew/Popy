# Popy 0.2.0-preview.2

Popy now has a softer, customizable desktop workspace with a fox or bear companion. English is the default, with 20 interface languages and remappable keyboard shortcuts. Updates can be checked, downloaded, verified and installed from inside the app.

## Changes

- Light/dark themes, gentle animations, reduced motion, adjustable interface size and resizable panels.
- Before/after divider, original hold, zoom/fit, queue selection and reordering from the keyboard.
- 29 configurable shortcuts with conflict checks and saved preferences.
- English, Turkish, Azerbaijani, Hindi, Russian, Spanish, Portuguese, French, German, Arabic, Chinese, Japanese, Korean, Italian, Dutch, Polish, Ukrainian, Indonesian, Vietnamese and Thai.
- Explicit update checking with preview-channel preference, Ed25519 verification, bounded downloads, file validation, backup/rollback and restart.
- In-app first-run tool setup, portable relative paths and interrupted-setup recovery.
- Refreshed English/Turkish README and quick-start documentation, preserving the original logo.

## Download and start

Use the Windows x64 Setup EXE for installation, or extract the portable ZIP completely. Open Popy, select **Set up tools** (F8), then add a video (Ctrl+O) and try a short preview (F5). The first setup downloads checksum-pinned tools and models. Rust and Visual Studio are not required.

The ZIP contains both `popy-desktop.exe` and its CLI worker. Keep the packaged files together. Tool/model files are downloaded separately; this is not a fully offline distribution.

## Validation and limits

24 automated tests cover media contracts, timing, supervision, shortcuts, translations and update security. Local integration checks cover GPU processing, cancellation, AAC/source integrity and the desktop preview. Update-helper tests cover replacement, rollback, protected data and path escape rejection. GitHub Actions runs format, Clippy, tests and a Windows release build.

The final ZIP passed 486 file checks, native first-run setup using verified cached archives, and a GPU preview after moving to a Unicode folder. The actual Setup EXE passed installation and uninstall tests while preserving generated settings/exports. An internal preview.1 client also completed a real signed update from this public GitHub release, restarted as preview.2, and preserved its preferences/runtime/exports. These checks used the existing Windows machine, not an independent clean OS.

This remains a preview. Output is 2×; accepted input is progressive constant-FPS H.264 with explicit BT.709 SDR metadata and optional aligned AAC. General input is limited to 960×540; anime input to 1920×1080. Automatic color is tone/contrast/saturation correction, not grayscale AI colorization. The first-frame comparison is static; moving results open in the system player.

Natural-footage quality, multi-hour stability and broader GPU hardware validation remain open. Local GPU tests used an AMD Radeon iGPU. The EXE has no Windows Authenticode signature; the update package uses a separate Ed25519 signature.
