# Getting started with Popy

1. Download and run the Windows x64 Setup EXE from [Popy releases](https://github.com/boranseasonnew/Popy/releases). For portable use, extract the ZIP completely into a writable folder.
2. Open Popy. On first use, select **Set up tools** (**F8**), or run `Setup Tools.cmd`. The setup downloads checksum-pinned FFmpeg, ncnn and model files into `.tools/`. Rust and Visual Studio are not needed to use the packaged app.
3. Select **Add videos** or press **Ctrl+O**. Choose the content type and optional automatic color adjustment.
4. Press **F5** for a short preview. Drag the comparison divider, hold **Space** for the original, or press **B / A** for original / enhanced. The comparison shows the first frame; **Open video** opens the moving result in your system player.
5. Press **Ctrl+Enter** to process the queue. Use **Ctrl+Shift+X** to stop. Every job gets its own folder containing `enhanced.mp4`, `comparison.png`, a report and diagnostic logs.

The source and existing output folders are never overwritten. Successful jobs remove temporary frame data; failed or cancelled jobs keep diagnostic files. Retrying starts from the beginning.

## Make it yours

Open **Settings** with **Ctrl+,**. English is the default; 20 interface languages are included. Choose the light or dark theme, fox or bear companion, reduced motion, interface size, and custom shortcuts. **Ctrl+B** cycles companions, **F6** switches theme, and **F1** opens the shortcut list. Native file dialogs follow the operating system's language. Some low-level diagnostics are not localized.

Select jobs with **Alt+Left / Right**, reorder with **Alt+Up / Down**, and remove a selected non-running job with **Delete**. These actions remove queue entries, not source videos. **Ctrl+Shift+Enter** opens a completed video; **Ctrl+Shift+O** opens its result folder. **Ctrl+Shift+S** changes the save folder. **Alt+C** toggles automatic color; **Alt+1 / 2** selects the content model. **F11** toggles full screen. All bindings can be changed in Settings; Tab / Shift+Tab navigate controls.

## Updates

Select **Check for updates**, or press **Ctrl+U**. If a newer signed release is available, select **Download & install**. Popy verifies the publisher signature and package contents, closes, installs the new app files and restarts. A running video job must finish or be stopped before installation. No browser visit is required. Preview releases can be included or excluded in Settings.

Updates preserve `.tools/`, settings, exports and source videos. If file replacement fails, the installer attempts to restore the previous app files and keeps a backup. The next launch displays the result. There are no scheduled background update checks. First-run tool downloads and explicit update checks use the internet; video processing stays local.

## Supported media

This preview accepts one progressive H.264 video stream, constant 1–120 FPS, 8-bit yuv420p, explicit limited-range BT.709 SDR metadata, square pixels and a near-zero start. Audio is optional; at most one aligned AAC stream is supported.

Output is 2×. General-model input is capped at 960×540 because its internal inference scale is 4×. Anime-model input is capped at 1920×1080. The general model can be slow on integrated graphics. Automatic color adjusts tone, contrast and saturation; it does not colorize black-and-white footage.

HDR, VFR, rotation, interlacing, untagged color and other codecs/layouts are not supported yet. The six-hour input cap is a safety limit, not a tested multi-hour stability claim. Current inference writes bounded PNG batches and temporary lossless segments; total segment storage grows with video duration. A conservative free-space check runs before processing. Lower tile size from 64 to 32 after a GPU-memory failure.

Windows x64 and a Vulkan driver are required. Local GPU integration tests used an AMD Radeon integrated GPU; RTX 30/40/50, discrete AMD and Intel devices are not certified. The executable does not have a Windows Authenticode signature. Update metadata uses a separate Ed25519 publisher signature; these are different mechanisms.

Popy code is Apache-2.0. Third-party tools, model weights and fonts have separate licenses. Package notices are in `DEPENDENCIES.md`, `licenses/` and `THIRD_PARTY.md`.
