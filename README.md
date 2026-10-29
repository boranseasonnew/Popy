<p align="center">
  <img src="branding/popy-logo-v1.png" alt="Popy, the orange red panda" width="180">
</p>

# Popy

**A little clearer. A lot more yours.**

Popy is an open-source desktop app for local video enhancement. Upscale a video, adjust its color, compare the result and process your queue from a calm, keyboard-friendly workspace—with a fox or bear keeping you company.

[Download for Windows](https://github.com/boranseasonnew/Popy/releases/tag/v0.2.0-preview.2) · [Türkçe](README.tr.md) · [Quick start](docs/QUICKSTART.md)

[![Rust checks](https://github.com/boranseasonnew/Popy/actions/workflows/check.yml/badge.svg)](https://github.com/boranseasonnew/Popy/actions/workflows/check.yml)

![Popy desktop application](docs/screenshots/popy-desktop.png)

<details>
<summary>Dark theme with the bear companion</summary>

![Popy dark theme](docs/screenshots/popy-desktop-dark.png)

</details>

## Why Popy?

Popy grew out of our frustration with the cost and limits of closed creative software, including Adobe's ecosystem. We wanted a tool built around the person using it: their device, their privacy and their choices.

Our response is a local, open-source application you can inspect, adapt and help improve. Popy requires no account or subscription. Your videos are processed on your computer. The interface gives you room to choose how you work, from the theme and companion to the keyboard shortcuts.

## What you can do

- **Enhance supported videos at 2× resolution** with Real-ESRGAN models for real footage or animation.
- **Turn automatic color adjustment on or off** for tone, contrast and gentle saturation.
- **Try a short preview** and compare the first frame with a draggable divider. Open the exported video to check motion.
- **Manage a queue** with progress, cancellation, retry, reordering and individual result folders.
- **Make the workspace yours** with light/dark themes, reduced motion, adjustable interface size and a fox, bear or hidden companion.
- **Work from the keyboard**, with shortcuts you can remap in Settings.
- **Use one of 20 interface languages**, with English as the default.
- **Check for updates inside the app** and download verified releases without visiting GitHub.

Automatic color adjustment does not add new colors to black-and-white footage. Popy uses existing models; restoration quality depends on the source and model.

## Get started

1. Download and run the Windows x64 **Setup EXE**, or extract the portable ZIP completely.
2. Open Popy. On first use, select **Set up tools** (**F8**), or run `Setup Tools.cmd`.
3. Add a video with **Ctrl+O**, choose the model and try a preview with **F5**.
4. Process the queue with **Ctrl+Enter**. Results are saved in new folders; your source files stay untouched.

The first setup downloads FFmpeg, ncnn and the models. Rust and Visual Studio are not needed to use the packaged app. Video processing works locally after setup; explicit update checks and downloads use the internet. See the [quick start](docs/QUICKSTART.md) for supported formats and troubleshooting.

| Shortcut | Action |
|---|---|
| Ctrl+O | Add videos |
| F5 | Create a preview |
| Ctrl+Enter | Process the queue |
| Ctrl+Shift+X | Stop the queue |
| Space / B / A | Hold for original / original / enhanced |
| Alt+Left / Right | Select a video |
| Alt+Up / Down | Reorder the selected job |
| Ctrl+, / F1 | Settings / shortcut editor |
| F6 / Ctrl+B | Switch theme / companion |
| Ctrl+U | Check for updates |

## Languages

English, Türkçe, Azərbaycanca, हिन्दी, Русский, Español, Português, Français, Deutsch, العربية, 简体中文, 日本語, 한국어, Italiano, Nederlands, Polski, Українська, Bahasa Indonesia, Tiếng Việt and ไทย.

Interface translations are included; native-speaker review is welcome. Native dialogs follow the operating system and some technical diagnostics are not localized.

## Current release

**0.2.0-preview.2** targets Windows x64 with a Vulkan driver. It accepts progressive H.264, constant FPS, explicit BT.709 SDR color metadata and optional aligned AAC audio. Output is 2×. General-model input is limited to 960×540; anime-model input to 1920×1080.

HDR, VFR, rotation, interlacing and other codecs/layouts are still being developed. The current engine uses bounded frame batches and temporary lossless segments, so long videos require disk space. Natural-video quality, multi-hour stability and a broader GPU matrix remain release gates. Local GPU tests used an AMD Radeon iGPU; RTX, discrete AMD and Intel support is not certified.

The screenshots show a real application session. Generated technical clips demonstrate the workflow; they do not establish restoration quality on natural footage.

## Build and contribute

Popy is written in Rust with egui/eframe and ncnn/Vulkan inference. Contributions to quality testing, hardware validation, translations and usability are welcome.

```powershell
.\scripts\bootstrap.ps1
.\scripts\dev.ps1 test
.\scripts\dev.ps1 build --release
```

[Development status](docs/STATUS.md) · [Roadmap](docs/ROADMAP.md) · [Signed updates](docs/UPDATES.md) · [Benchmark tools](benchmarks/README.md)

## License and credits

Popy's original code is licensed under [Apache-2.0](LICENSE). Tools, fonts and model weights have their own [licenses and provenance](docs/THIRD_PARTY.md).

Built with the work of [Real-ESRGAN](https://github.com/xinntao/Real-ESRGAN), [ncnn](https://github.com/Tencent/ncnn), [FFmpeg](https://ffmpeg.org/) and [egui](https://github.com/emilk/egui). Popy is independent and is not affiliated with Adobe or Topaz Labs.
