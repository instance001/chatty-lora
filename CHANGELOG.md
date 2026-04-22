# Changelog

## v0.1.0 - 2026-04-22

Initial public-source preparation for Chatty-lora.

### Added

- Two-page dashboard with `Materials` and `Builder` workflows.
- Respectful source registry with bundled sources, custom sources, media-type filters, and small batched previews.
- Local folder cleanup flow for turning manually collected media into clean dataset folders.
- Dataset curation with normalized filenames, sidecar captions, and metadata manifests.
- Scoped site-fix shell for source-specific crawler troubleshooting, proposal review, and backup-first apply.
- Wan 2.1 T2V 1.3B / Musubi Tuner training plan generation for video and still-image visual LoRA lanes.
- Guided app runner for saved Wan/Musubi plans, with staged preflight, latent cache, text cache, and training launch.
- ECG Window near the active saved training plan, showing CPU and GPU activity traces during split cache/training work.
- Beginner-facing docs covering setup, material collection, crawler limits, training settings, and low-VRAM Wan notes.

### Changed

- Treat manual material collection plus `Local folder cleanup` as the primary dataset workflow.
- Frame web crawling as a polite secondary convenience feature, not a guaranteed collection method.
- Keep local models, runtime binaries, generated handoff files, training outputs, and test datasets out of git by default.
- Preserve trained LoRA outputs when deleting a saved training plan from the UI.

### Known Limits

- The first runnable training path is intentionally narrow: Wan 2.1 T2V 1.3B with Musubi Tuner in WSL.
- Web crawling is constrained by robots rules, login walls, anti-scraping systems, JavaScript-only feeds, rate limits, expiring URLs, and site terms.
- Chatty-lora does not try to bypass private feeds, social-media protections, CAPTCHAs, or platform access controls.
- Training history is still lightweight; the saved plan card and output folders are the main run records for now.
