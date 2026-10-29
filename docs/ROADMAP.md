# Popy roadmap

The working CLI is the first technical milestone. The full application is still in development.

## 0. Reproducible research

- [x] Local ncnn/Vulkan inference, media checks and validated output.
- [x] Generated 144p/360p fixtures and optional automatic color adjustment.
- [x] Strict benchmark manifests and repeated comparisons.
- [ ] A 15-clip test set with documented usage rights, including real footage.
- [ ] A lighter general 2× model with documented provenance and redistribution terms.
- [ ] Repeated hardware measurements and temporal-quality review.

**Gate:** demonstrate a useful quality/speed tradeoff for the target content. Synthetic patterns alone do not pass this gate.

## 1. Streaming engine

- Bounded frame queues instead of full PNG sequences.
- A supervised worker and narrow native inference bridge.
- Robust timestamps, color conversion, audio handling and cancellation.
- Long-run memory and failure-recovery tests.

## 2. Desktop application

- Rust desktop interface using the selected Popy identity.
- Short before/after preview and understandable presets.
- Job queue, GPU selection and useful error messages.

## 3. Packaging and compatibility

- Clean-machine Windows install and model management.
- Real tests across selected RTX and AMD cards; explicit experimental iGPU status.
- Third-party notices, source/build provenance and reproducible packages.

## Later research

RIFE interpolation, temporal restoration models, HDR/VFR workflows, checkpoint/resume and additional platforms. Black-and-white AI colorization is a separate model task, not part of the current automatic color control.
