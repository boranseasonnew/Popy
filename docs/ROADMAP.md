# Popy roadmap

## 0.2 preview — implemented

- [x] Rust CLI, reproducible fixtures and repeated benchmark suites.
- [x] Real-ESRGAN general 4× → 2× and animevideo native 2× with ncnn/Vulkan.
- [x] Optional smoothed color correction and explicit sRGB/BT.709 conversion.
- [x] Native desktop: file picker/drop, settings, queue, progress, cancellation and first-frame comparison.
- [x] Full-clip batch export, bounded current PNG data, lossless RGB segments and successful-job cleanup.
- [x] Streaming CFR/AAC hash scans, source-integrity and output checks.
- [x] Supervised processes, timeouts, Windows Job Objects and exclusive output directories.
- [x] Windows packaging script, first-run bootstrap and dependency notices.

## Required before stable 1.0

- [ ] Licensed natural-video quality corpus: faces, motion, textures, text, animation and compression artifacts; target at least 15 clips.
- [ ] Blind comparison against Lanczos, checking flicker, invented detail and color changes. Synthetic patterns alone do not establish quality.
- [ ] Faster, practical general model with verified weight provenance/conversion; current model is expensive and internally 4×.
- [ ] Multi-hour stress, disk exhaustion, corrupted media and forced-close recovery tests.
- [ ] Actual RTX 30/40/50, discrete AMD and selected Intel/AMD iGPU validation; device enumeration and clearer selection.
- [ ] Common untagged SDR import, additional codecs, audio offsets and explicit VFR conversion preserving timing.
- [ ] Embedded playback and moving before/after preview; clearer recovery.
- [ ] Clean Windows installation test, runtime dependency checks and signing decision.
- [ ] Runtime/model redistribution review before a fully offline bundle.

## After a useful, verified 1.0

- [ ] Persistent Rust/FFI inference worker avoiding model reloads and large temporary segments.
- [ ] Checkpoint continuation rather than restarting cancelled jobs.
- [ ] More scales/presets, denoise and deblur after quality benchmarks justify them.
- [ ] Frame interpolation, HDR preservation and AI grayscale colorization as separately researched capabilities.
- [ ] Linux/macOS packaging and hardware tests.

Color correction and grayscale colorization are distinct features. Roadmap items do not imply delivery dates or certified support.
