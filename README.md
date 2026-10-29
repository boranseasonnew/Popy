<p align="center">
  <img src="branding/popy-logo-v1.png" alt="Popy, the orange red panda" width="180">
</p>

# Popy

Local, open-source video enhancement built with Rust and ncnn/Vulkan.

[![Rust checks](https://github.com/boranseasonnew/Popy/actions/workflows/check.yml/badge.svg)](https://github.com/boranseasonnew/Popy/actions/workflows/check.yml)

[Türkçe README](README.tr.md) · [Roadmap](docs/ROADMAP.md) · [Development status](docs/STATUS.md)

**Early research prototype.** Popy currently has a working CLI, not a desktop interface. It processes short SDR clips locally, compares AI upscaling with Lanczos, and records the settings and measurements needed to reproduce a test.

## What works

- Real-ESRGAN inference on a selected Vulkan GPU.
- 2× output with an anime model, or a general 4× model followed by downsampling to 2×.
- Optional automatic levels/contrast and mild saturation correction.
- Source, Lanczos and AI comparison images, plus JSON/Markdown reports.
- Model/tool/source hashes, stage timings and per-process peak RAM on Windows.
- Input checks for frame timestamps, color metadata, audio and geometry.
- Output validation, AAC packet checks, cancellation and protection against overwriting an existing run.
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
```

Review [third-party provenance and licenses](docs/THIRD_PARTY.md) before redistributing tools or model weights. Downloads are pinned by archive checksums. The current local FFmpeg package is a GPL build and is not embedded in the Popy executable.

## Try a generated clip

```powershell
.\scripts\dev.ps1 run -- fixtures --out fixtures/generated/my-test
.\scripts\dev.ps1 run -- benchmark --input fixtures/generated/my-test/input-144p.mp4 --out runs/my-test --model anime --seconds 10 --gpu 0 --tile 64 --rights "Generated FFmpeg technical fixture"
```

Use a new `--out` directory for every run. Add `--auto-color` to enable color adjustment. Use `--model general` for the heavier general model; its native inference scale is 4× even though the final output is 2×.

Each run produces `enhanced.mp4`, `lanczos.mp4`, `comparison.png`, reports and logs. Files stay on the local machine; the CLI does not upload videos.

For repeated comparisons, see [benchmark suites](benchmarks/README.md). A dry run validates the manifest without loading models or creating an output directory:

```powershell
.\scripts\dev.ps1 run -- suite --manifest benchmarks/technical.json --dry-run
```

## Current limits

| Area | Research prototype scope |
|---|---|
| Input | Up to 30 seconds; one progressive H.264 video stream, 8-bit yuv420p, explicit limited-range BT.709 SDR, square pixels |
| Audio | At most one AAC stream with a near-zero start offset |
| Processing | Up to 10 seconds / 300 frames per job; 2 GB estimated raw frame budget |
| Output | 2× dimensions; native model output capped at 3840×2160 |
| Unsupported | HDR, VFR, interlacing, rotation and complex stream layouts |
| Storage | PNG frame sequences on disk; a streaming worker is planned |
| UI | CLI only; desktop preview and queue are planned |

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
```

Unit tests and Clippy do not require model downloads. GPU smoke tests do. To report an issue, include the Popy revision, model, GPU/driver, input metadata and the relevant log. Review logs for local paths before sharing them; do not attach private footage by default.

The next milestone is a licensed quality test set and a lighter general 2× model, followed by a streaming worker and desktop preview. See the [roadmap](docs/ROADMAP.md) before starting a large feature.

## License and credits

Popy's original code is licensed under [Apache-2.0](LICENSE). Third-party tools and weights have their own licenses.

Thanks to [Real-ESRGAN](https://github.com/xinntao/Real-ESRGAN), [ncnn](https://github.com/Tencent/ncnn), [FFmpeg](https://ffmpeg.org/) and their contributors. Popy is an independent project and is not affiliated with Topaz Labs.
