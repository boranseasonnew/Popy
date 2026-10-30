# Popy 0.2.0-preview.3 — broader video import

Fixes the "Only 8-bit yuv420p SDR" error for otherwise readable videos, including full-range yuvj420p exports.

- Automatic import for FFmpeg-decodable video containers/codecs, 10/12-bit and different pixel formats.
- PQ/HLG HDR tone mapping to BT.709 SDR, full-range conversion and recorded assumptions for missing color metadata.
- VFR-to-CFR conversion, automatic rotation, flagged interlacing, non-square pixels and odd-sized frames.
- The first real video and first audio track are selected; audio is aligned, resampled and converted to stereo AAC when conversion is needed. Other tracks are omitted.
- Removes the old 960×540/general and 1920×1080/anime input rejection. Input is not automatically reduced in size. Actual GPU memory, temporary disk space and encoder/decoder limits still apply.
- Broad file picker plus All files; keeps compatible inputs on the existing path.
- Fixes completed previews occasionally showing an empty comparison while the PNG was still being written.
- Sources stay unchanged. The internal converted video is lossless H.264 after normalization to 8-bit SDR. VFR resampling, HDR tone mapping and audio encoding are conversions, not preservation of the original format.

Output remains 2× H.264/AAC MP4 in 8-bit SDR. This is not a promise to read corrupt, encrypted or every proprietary format, or to retain HDR output. Benchmark/inspect retain the strict research contract; desktop/enhance use automatic import. The six-hour safety cap remains.

## Validation

27 automated tests and the Windows format/Clippy/release build checks passed. The final packaged binaries passed the complete import matrix, desktop comparison-texture checks in light/dark themes, and verification of all 486 extracted package files.

The import matrix passed HEVC 10-bit, PQ HDR, HLG HDR, VP9/Opus with delayed audio, MJPEG AVI, 16-bit RGB, VFR, anamorphic/interlaced, GIF and odd-sized RGB sources. A provided 1920×1080/60 FPS full-range source passed import of all 180 frames with audio and source-integrity checks. Additional checks covered a real display rotation and 240 FPS. Local AMD iGPU inference produced a 3840×2160 frame from that source and passed a general-model job above the previous width limit. These small technical tests do not certify natural-footage quality, every codec, long jobs or other GPUs.

The installed application also imported the provided source successfully while preserving preferences/runtime. An isolated preview.2 client completed the real public GitHub update to preview.3, restarted, and preserved its export sentinel.
