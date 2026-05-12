# Changelog

## Unreleased

### Added

- Builder concept-stack workflow with focused concept blocks instead of one flat concept summary.
- Concept block roles for `Primary lesson`, `Supporting detail`, and `Avoid / don't reinforce`.
- Inline concept-card editing, duplication, reorder controls, and stack export/import through a local transfer box.
- Shared registry groundwork for model families, training backends, and training lanes.
- Family-aware Builder guidance that groups base models, ranks compatible backends, and explains auto-suggested versus manual backend choices.
- Saved backend-choice state persisted into training plans, saved-plan cards, generated handoff readouts, `plan.json`, and generated per-plan `README.md`.
- Exploratory `Musubi Tuner / Wan 2.1 T2V 14B` lane support with task-aware handoff generation, 14B DiT detection, and Builder suggestion/ranking that can distinguish `Wan 14B` from the `1.3B` Wan path.
- Tightened the exploratory `Wan 14B` route around the only local variant that reached live training so far: BF16-loaded DiT weights, lower visible defaults, stronger WSL RAM+swap warnings, persisted per-run logs, and more explicit docs that this lane can still exceed a 32GB-class Windows box even after training starts.
- Updated the `Wan 14B` docs to frame the lane more honestly: it now clearly says the route shows promise and can reach live training, but it was not validated end to end on the current 32GB test rig because system RAM plus WSL swap appear to be the real limiter. The docs now explicitly invite open-expectation testing and refinement from stronger hardware.
- First scaffolded `AI Toolkit / Wan 2.2 TI2V 5B` groundwork: dedicated backend and lane registry entries, local Diffusers-bundle detection, base-model picker support, and generated handoff folders that prepare dataset JSONL and an initial AI Toolkit job scaffold without pretending the route is proven yet.

### Changed

- Frontend concept-stack logic now lives in [`static/concept-stack.js`](static/concept-stack.js) instead of being buried inside the main `app.js` bundle.
- Beginner-facing Builder docs now explain the stacked concept workflow, block roles, and stack reuse tools.
- Model storage now follows family-first buckets such as `models/wan/gguf/`, `models/wan/dependencies/`, `models/flux/gguf/`, and `models/ai_assistant/gguf/`.
- Wan model-path detection now prefers the new family layout while still accepting the legacy `models/wan21_t2v_1_3b/` folder during transition.
- Dashboard model summaries, Builder readouts, and backend cards now use the same family-based architecture instead of flat model-path assumptions.

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
- clarified the `AI Toolkit / Wan 2.2 TI2V 5B` runtime expectation: Chatty-lora now documents `runtime/ai-toolkit/` as the shared repo-root checkout for this lane and future Diffusers-style trainer routes like Flux, SDXL, SD 1.5, and newer Wan or audio-capable workflows
