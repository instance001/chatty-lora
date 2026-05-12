# Runtime Folder

Drop local helper binaries here when you want Chatty-lora to use a bundled runtime from the project folder.

Current conventions:

- `runtime/ai-toolkit/` or `runtime/ai_toolkit/`
  A local clone of the `ostris/ai-toolkit` repo root. Chatty-lora currently looks for the normal repo shape here, such as:
  - `run.py`
  - `requirements.txt`
  - `toolkit/`
  - `config/examples/`
  - `ui/`

That AI Toolkit checkout is intended to be a shared trainer bucket for future Diffusers-style lanes too, not just the first `Wan 2.2 TI2V 5B` scaffold. Over time it may back Wan, Flux, SDXL, SD 1.5, and similar modern image or video routes.

This folder is intentionally ignored by git except for this README and `.gitkeep`, because runtime builds can be large, platform-specific, and frequently replaced.
