<p align="center">
  <img src="branding/popy-logo-v1.png" alt="Popy, the orange red panda" width="180">
</p>

# Popy

Local, open-source video enhancement built with Rust and ncnn/Vulkan.

[![Rust checks](https://github.com/boranseasonnew/Popy/actions/workflows/check.yml/badge.svg)](https://github.com/boranseasonnew/Popy/actions/workflows/check.yml)

[Türkçe README](README.tr.md) · [Roadmap](docs/ROADMAP.md) · [Development status](docs/STATUS.md)

**0.2 preview — Windows desktop + CLI.** Popy processes supported SDR videos locally with a native Rust interface, a sequential queue and a before/after image preview. Perceptual quality and broad GPU compatibility have not passed the stable-release gates.

## Download and run

Download the Windows x64 ZIP from [Releases](https://github.com/boranseasonnew/Popy/releases). Extract it completely into a writable directory, run `Araclari Kur.cmd` once to download the pinned runtime tools/models, then open `popy-desktop.exe`. Rust and Visual Studio are not needed to use the package. Internet is required for the initial downloads; video processing stays local.

Choose a video, model and optional automatic color adjustment. Try a 3-second preview and open the moving result before processing the full queue. The UI is currently Turkish. See the [quick start and format limits](docs/QUICKSTART.tr.md).

![Popy desktop, generated technical clip](docs/screenshots/popy-desktop.png)

## What works

- Real-ESRGAN inference on a selected Vulkan GPU.
- Native desktop: file picker/drop, settings, queue, progress, cancellation and first-frame comparison slider.
- Full-clip export with bounded PNG batches and lossless RGB segments; successful jobs clean temporary data.
- 2× output with an anime model, or a general 4× model followed by downsampling to 2×.
- Optional automatic levels/contrast and mild saturation correction.
- Source, Lanczos and AI comparison images, plus JSON/Markdown reports.
- Model/tool/source hashes, stage timings and per-process peak RAM on Windows.
- Input checks for frame timestamps, color metadata, audio and geometry.
- Output validation, AAC packet checks, cancellation and protection against overwriting an existing run.
- Supervised child processes, timeouts and Windows kill-on-close Job Objects.
- Repeated benchmark suites with strict manifests, sequential GPU jobs and JSON/CSV summaries.

Automatic color adjustment does not colorize black-and-white footage. Models are not trained by Popy, and results depend on the source and selected model.

## Development setup

The supported development target is **Windows x64 with PowerShell 7**. You need a working Vulkan driver and MSVC/Windows SDK libraries. The bootstrap script downloads Rust, FFmpeg/ffprobe and the upstream ncnn model package into `.tools/`; it does not change the user PATH or install Visual Studio.

```powershell
git clone https://github.com/boranseasonnew/Popy.git
cd Popy
.\scripts\bootstrap.ps1
.\scripts\dev.ps1 test
.\scripts\dev.ps1 run -- doctor
.\scripts\dev.ps1 build --release
.\target\release\popy-desktop.exe
```

Review [third-party provenance and licenses](docs/THIRD_PARTY.md) before redistributing tools or model weights. Downloads are pinned by archive checksums. The current local FFmpeg package is a GPL build and is not embedded in the Popy executable.

## Try a generated clip

```powershell
.\scripts\dev.ps1 run -- fixtures --out fixtures/generated/my-test
.\scripts\dev.ps1 run -- benchmark --input fixtures/generated/my-test/input-144p.mp4 --out runs/my-test --model anime --seconds 10 --gpu 0 --tile 64 --rights "Generated FFmpeg technical fixture"
```

Use a new `--out` directory for every run. Add `--auto-color` to enable color adjustment. Use `--model general` for the heavier general model; its native inference scale is 4× even though the final output is 2×.

To export a complete supported video:

```powershell
.\target\release\popy.exe enhance --input video.mp4 --out runs/export-01 --model general --auto-color
```

Add `--preview-seconds 3` for a sample. `benchmark` remains the short research workflow with a Lanczos comparison. The export engine reads RGB frames through a pipe, limits PNG batches to a 192 MiB raw RGB estimate, and saves lossless RGB segments until final encoding. This is not FFI inference: ncnn reloads its model per batch and segment storage grows with video duration. Color adjustment runs once after concatenation, preserving temporal smoothing across batch boundaries.

Each run produces `enhanced.mp4`, `lanczos.mp4`, `comparison.png`, reports and logs. Files stay on the local machine; the CLI does not upload videos.

For repeated comparisons, see [benchmark suites](benchmarks/README.md). A dry run validates the manifest without loading models or creating an output directory:

```powershell
.\scripts\dev.ps1 run -- suite --manifest benchmarks/technical.json --dry-run
```

## Current limits

| Area | 0.2 preview scope |
|---|---|
| Input | One progressive H.264 video stream, 8-bit yuv420p, explicit limited-range BT.709 SDR, square pixels, near-zero start |
| Audio | At most one AAC stream with a near-zero start offset |
| Processing | Full clip with `enhance`, 6-hour safety cap; `benchmark` up to 10 seconds / 300 frames. The cap is not a tested duration claim. |
| Output | 2× dimensions; native model output capped at 3840×2160 |
| Unsupported | HDR, VFR, interlacing, rotation and complex stream layouts |
| Storage | Bounded PNG batches plus lossless RGB segments; free-space preflight; failed/cancelled jobs keep diagnostic data |
| UI | Native Turkish desktop, sequential queue, external video player; no embedded playback |

General input is capped at 960×540 because its internal scale is 4×; anime input at 1920×1080. Inputs without explicit color metadata are rejected. HDR/VFR conversion and arbitrary consumer formats are not yet implemented. Retry starts from the beginning.

NVIDIA RTX 30/40/50 and selected AMD GPUs are intended targets, not a certified compatibility list. The first local tests used an AMD Radeon integrated GPU. Intel/AMD iGPU support remains experimental.

## Initial measurements

One generated 10-second, 12 FPS clip; AMD Radeon iGPU; animevideo 2×; tile 64. AI timings include cold model loading and PNG I/O.

| Input → output | Frames | AI stage |
|---|---:|---:|
| 144p → 288p | 120 | 21.55 s |
| 360p → 720p | 120 | 125.20 s |

These are integration measurements, not evidence of restoration quality on faces or natural footage. Peak process RAM is not GPU VRAM. See [measurement records](docs/measurements/2026-10-29.json) and [limitations](docs/STATUS.md).

## Checks and contributions

```powershell
.\scripts\dev.ps1 fmt --check
.\scripts\dev.ps1 test
.\scripts\dev.ps1 clippy --all-targets
.\scripts\dev.ps1 build --release
.\scripts\smoke.ps1 -WithGpu
.\scripts\export-smoke.ps1 -WithDesktop
.\scripts\package.ps1
```

Unit tests and Clippy do not require model downloads. GPU smoke tests do. To report an issue, include the Popy revision, model, GPU/driver, input metadata and the relevant log. Review logs for local paths before sharing them; do not attach private footage by default.

Stable-release gates include a licensed natural-video quality set, a practical general model, longer stress tests and a real NVIDIA/AMD/Intel hardware matrix. See the [roadmap](docs/ROADMAP.md) and [release notes](docs/RELEASE-0.2.0-preview.1.md). The Windows ZIP is unsigned; third-party runtimes and weights are downloaded separately.

## License and credits

Popy's original code is licensed under [Apache-2.0](LICENSE). Third-party tools and weights have their own licenses.

Thanks to [Real-ESRGAN](https://github.com/xinntao/Real-ESRGAN), [ncnn](https://github.com/Tencent/ncnn), [FFmpeg](https://ffmpeg.org/) and their contributors. Popy is an independent project and is not affiliated with Topaz Labs.
