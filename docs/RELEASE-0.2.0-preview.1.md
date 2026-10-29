# Popy 0.2.0-preview.1

Popy now has a native Rust desktop app and full-clip batch export. Add videos, choose the model and optional automatic color correction, try a short preview, then process the queue locally. A slider compares the first original/enhanced frame; open the exported video to assess motion.

## Added

- File picker/drop, sequential queue, progress, cancellation and retry.
- Full-clip export: backpressured RGB decoder, bounded PNG batches, lossless RGB segments and successful-job cleanup.
- Streaming timestamp/AAC hash scans, source-integrity checks and atomic progress files.
- Supervised processes, deadlines and Windows kill-on-close Job Objects.
- Windows x64 packaging with dependency and bundled-font license notices.
- Separate checksum-pinned runtime bootstrap; end users do not need Rust or Visual Studio.

## Install

Extract the ZIP completely. Run `Araclari Kur.cmd` once to download FFmpeg/ncnn/models, then open `popy-desktop.exe`. Internet is required for the initial downloads. The UI is currently Turkish. Processing does not upload footage. The ZIP is unsigned and contains no third-party runtime binaries or weights.

## Scope

This is a **prerelease**, not stable 1.0. Inputs: progressive H.264, constant FPS, 8-bit limited-range BT.709 SDR with explicit color tags and square pixels, with zero or one near-zero-offset AAC stream. HDR, VFR, rotation and other codecs/layouts are rejected. Output is 2×; the general model internally uses 4×, capping general input at 960×540, anime input at 1920×1080.

Automatic color means levels/contrast and mild saturation, not AI grayscale colorization. General inference is slow on integrated graphics. PNG batches are bounded, but lossless segment storage grows with video duration and models reload per batch. Retry starts from the beginning. No embedded video player yet.

Local tests use an AMD Radeon integrated GPU. RTX 30/40/50, discrete AMD and Intel devices are not certified. Natural-video quality, multi-hour stability and clean-machine installation remain stable-release gates. See [status](STATUS.md) for completed tests and [quick start](QUICKSTART.tr.md) for usage.

Popy: Apache-2.0. Runtime tools/weights have their own licenses. See `THIRD_PARTY.md`, `DEPENDENCIES.md` and `licenses/` in the package. Credits: Real-ESRGAN, ncnn, FFmpeg and egui. Independent project; no Topaz Labs affiliation.
