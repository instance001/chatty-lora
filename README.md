# Chatty-lora
Sister repo to Chatty-art - local media generation tool found at:
https://github.com/instance001/chatty-art

Chatty-lora is a local LoRA builder dashboard for:
- respectful training-material search
- dataset curation
- training plan preparation
- source-specific crawl adapter review and repair

It is a standalone sister tool to Chatty-art, but it is not wired into Chatty-art. The copy of `chatty-art/` sitting beside it is reference material only.

## What It Does Today

Chatty-lora currently focuses on the front half of the LoRA workflow:

- `Materials` page
  - browse bundled and custom search sources
  - search in polite `3-page` batches
  - preview results before downloading
  - keep selected items in a lightweight selection tray
  - curate chosen files into a named dataset folder under `inputs/`

- `Builder` page
  - choose a curated dataset
  - choose a detected base model with family-aware grouping
  - choose a training backend target with compatibility guidance
  - define trigger phrase, concept type, preset, and starter settings
  - save a reusable training plan into `config/projects/`
  - generate Wan 2.1 / Musubi Tuner handoff files into `config/training/generated/`
  - surface a Wan preflight card for model files, WSL, and trainer readiness
  - explain when a backend was auto-suggested versus manually overridden

- `Helper Chat`
  - page-aware local guidance for both `Materials` and `Builder`

- `Scoped Site-Fix Shell`
  - inspect one source at a time
  - save a source-specific fix brief
  - draft a source-specific proposal
  - preview a backup-first adapter patch
  - apply the patch to one adapter file only
  - keep proposal history and applied-patch history per source

## Current Training Scope

Chatty-lora can now launch the first proven training lane from the Builder UI:

- Wan 2.1 T2V 1.3B
- Musubi Tuner inside WSL
- AMD ROCm/Radeon GPU training
- generated preflight, cache, and launch scripts

There are now two early Wan/Musubi foundations:
- `Video motion lane` for video clips and motion/style experiments
- `Image visual lane` for still-image identity, object, and style concepts trained into the same Wan 2.1 T2V family

The app runner is intentionally scoped to saved Wan/Musubi plan cards. It does not yet train arbitrary model families, manage multiple simultaneous jobs, or provide a full historical run database. Manual PowerShell/WSL commands remain visible as the fallback path when a driver, ROCm, or Musubi issue needs closer inspection.

Under the hood, Chatty-lora now keeps this groundwork separate on purpose:
- model families such as `wan`, `flux`, and `ai_assistant`
- training backends such as Musubi, `kohya_ss`, `AI Toolkit`, and `OneTrainer`
- training lanes that connect a family to a backend and dataset kind

That keeps the current Wan path simple while giving future Flux, audio, and non-Musubi routes somewhere clean to plug in.

## Design Principles

- `Respectful crawling first`
  - low-volume search
  - batched previews
  - adapter-based source handling
  - no crawler-core sprawl when one site shifts

- `Local and review-first`
  - source fixes are scoped to one adapter file
  - auto-apply writes a backup first
  - proposal history and apply history are kept separately

- `Beginner-friendly on the surface`
  - clear panels
  - small number of pages
  - helper guidance always nearby

- `Advanced where it matters`
  - source registry editing
  - adapter review/apply flow
  - structured builder settings

## Folder Layout

- [`inputs/`](inputs/)
  Curated datasets live here.

- [`outputs/`](outputs/)
  Training outputs and future exports.

- [`models/`](models/)
  Base models and helper weights that the future training flow can detect.

- [`runtime/`](runtime/)
  Local runtime support files. A bundled `llama.cpp` runtime can live here.

- [`defaults/`](defaults/)
  Bundled defaults such as source definitions.

- [`config/`](config/)
  Local app data:
  - source registry
  - saved training plans
  - generated trainer handoff files
  - site-fix notes
  - proposal history
  - applied patch history

## Launching

From the project root:

```powershell
cargo run
```

Chatty-lora starts on:

```text
http://127.0.0.1:7879
```

## Materials Workflow

1. Enable one or more sources.
2. Enter a search term.
3. Choose which media types to search: `Images`, `Video`, and/or `Audio`.
4. Review preview results in `3-page` batches.
5. Tick the items you want.
6. Review them in `Selection tray`.
7. Name the dataset folder.
8. Click `Curate selected into dataset`.

Chatty-lora then:
- downloads the selected items
- names them cleanly
- writes simple sidecar `.txt` captions beside saved media where possible
- writes a `metadata.json` manifest
- hands the new dataset over to the `Builder` page

If you already gathered material manually, use `Local folder cleanup` on the Materials page instead. Drop a jumble folder into `inputs/`, refresh the scan, pick it as the source folder, name the cleaned copy, and click `Clean folder into dataset`. Chatty-lora leaves the original alone and creates a new curated dataset with media buckets, normalized filenames, sidecar captions, and `metadata.json`.

That manual-folder path is the primary, most reliable material collection workflow. The web search/crawler is a secondary convenience feature: excellent when a source exposes simple public media pages or a friendly API, but not something to depend on as the only way to build a dataset.

Search now has two separate axes:
- source toggles decide where Chatty-lora searches
- media toggles decide whether the search is looking for images, video, audio, or a mix

Leaving the search term blank now runs `Browse mode`. Browse mode asks selected sources for their first available media pages instead of a specific query. This works best with API-backed sources or custom listing URLs that expose media in page order.

### Honest Crawler Limits

Chatty-lora's web search is intentionally conservative. It is designed to reduce the chance of users being IP banned, rate-limited, or accidentally hammering a source site.

It does this by:
- loading small preview batches
- keeping searches user-reviewed before download
- staying same-site during homepage rescue
- respecting robots and source boundaries where possible
- using source-specific fixes instead of turning the crawler into a giant internet vacuum

That also means it will not, and should not, bypass every barrier. Results may fail or be incomplete because of circumstances outside this project:
- login walls and private feeds
- social-media anti-scraping protections
- JavaScript-only infinite scroll
- expiring media URLs
- CAPTCHAs, bot detection, and rate limits
- site-specific terms of service
- pages that hide media behind APIs the browser can see but a simple respectful fetch cannot

Use the crawler as "nice when it works with your source." Use manual collection plus `Local folder cleanup` as the dependable path.

Source support now has three practical tiers:
- Openverse and Wikimedia use purpose-built public API adapters.
- Generic gallery HTML sources use a cautious best-effort scanner for common image/audio/video links.
- Unknown or broken sites should be handled through the scoped site-fix shell rather than changing crawler core.

Sources can be added, enabled/disabled, opened in the browser, removed from the local registry, or inspected through the site-fix shell. Removing a bundled source only prunes the local `config/sources.json` list. Generic gallery URLs can include `{query}` and `{page}` placeholders. If a search term is present and `{query}` is not used, Chatty-lora appends a `q` query parameter. If `{page}` is not used, Chatty-lora appends a `page` query parameter.

`{query}` values are URL-encoded before being inserted into custom source templates, so searches like `@maker_handle`, `journal cover #4598`, or `wallet & strap` do not accidentally break the URL. This improves public gallery/search pages and personal archive workflows, but it does not bypass social-media login walls, private feeds, JavaScript-only infinite scroll, robots rules, or anti-bot systems.

Pointing a generic source at a homepage is enough only when that homepage directly links to the media you want. It is not a site-wide crawl. For better coverage, point the source at a gallery/search/listing URL, include `{page}` when the site supports paging, or apply a source-specific connection fix through the site-fix shell.

To soften the common "I pasted the homepage and got nothing" case, generic sources include a bounded homepage rescue for both search and blank Browse mode. If the starting page has no direct media, Chatty-lora cautiously checks same-site gallery/media/feed/sitemap links and scans a few of those pages. For a normal search term, it can also carry the query into obvious same-site search/gallery links that do not already have query parameters. It stays same-site, small-batch, and robots-aware; if that still returns nothing, the source needs a better listing URL, a `{page}` template, or a selector profile.

The generic site-fix proposal step now performs a small inspection pass. When you click `Draft scoped fix proposal`, Chatty-lora fetches the selected source page, looks for obvious pagination links, and drafts a `base_url_template` when it can infer one. For example, a detected next link like `https://example.com/images/category/page2` may become `https://example.com/images/category/page{page}`. It also drafts a selector profile from the sampled HTML when it can see repeated media containers. The apply step is still review-first: no source entry is changed until `Review proposed fix` shows the diff and you click `Apply proposed fix`.

## Builder Workflow

1. Pick the curated dataset.
2. Pick a base model.
3. Pick the training backend target.
4. Build a `Concept stack` one block at a time:
   select a block type, pick a role, enter a trigger term, add the training intent, and describe that one lesson.
5. Click `Add concept block`.
6. Repeat for supporting pose, outfit, environment, composition, or guardrail blocks as needed.
7. Choose a preset and starter settings.
8. Click `Save training plan`.

That writes a reusable training plan into [`config/projects/`](config/projects/).

Each concept block stays compact in the stack below the composer. You can expand/collapse, reorder, edit inline, duplicate, or delete blocks without leaving the Builder page.

The Builder is now family-aware too:
- base-model choices are grouped by model family
- backend choices are ranked toward compatible lanes first
- Chatty-lora can auto-suggest a better backend when the selected family changes
- a visible badge explains whether the current backend is `auto-suggested` or a `manual choice`

Concept block roles:

- `Primary lesson`: the main identity, style, or core concept. Primary blocks lead the generated caption recipe.
- `Supporting detail`: secondary context such as outfit, pose, environment, or camera language.
- `Avoid / don't reinforce`: saved guardrails for recurring distractions or mistakes; these are kept out of the positive caption recipe in the current Wan lane.

Concept stack reuse:

- `Export stack` writes the current block stack into the transfer box and also copies it to the clipboard when the browser allows it.
- `Import replace` swaps the current stack for the pasted one.
- `Import append` merges pasted blocks into the current stack.

For the Wan/Musubi backends, it also writes a trainer handoff folder into [`config/training/generated/`](config/training/generated/).

The handoff folder contains:
- `dataset.toml`
- `video_metadata.jsonl` for video plans, or `image_metadata.jsonl` for image plans
- `preflight.sh`
- `cache_latents.sh`
- `cache_text.sh`
- `launch.sh`
- `run_all.sh`
- `plan.json`
- `README.md`

`plan.json` and the generated `README.md` now also record:
- the selected training lane
- the backend id
- whether the backend was an `auto-suggested` match or a deliberate manual override

The saved plan card also surfaces a `Wan handoff` block with copyable PowerShell commands for:
- running preflight checks without training
- caching latents
- caching text encoder outputs
- launching training
- running the whole sequence once the step-by-step flow is trusted

The saved plan card also has an app runner. `Run this saved plan` launches the generated sequence from Chatty-lora:

1. preflight
2. cache latents
3. cache text encoder outputs
4. train the LoRA

The runner keeps those as separate internal stages, so a failure can report whether the problem was setup, VAE latent cache, T5 text cache, or the training launch itself. Manual PowerShell commands remain visible as the fallback path.

Saved plan cards now keep the backend-choice state too, so when you reload or inspect a plan later you can still see whether that backend came from Chatty-lora's family-aware suggestion or from an intentional manual override.

The top runnable saved-plan card also shows an `ECG Window`: a small CPU/GPU activity graph using Windows performance counters. The CPU line helps make cache/prep work visible during long GPU-quiet stages, while the GPU line shows training bursts once Musubi is doing Radeon compute work.

Important output behavior:

- The runner uses the saved plan card, not unsaved form edits above it.
- If you change concept type, settings, trigger phrase, or project name, click `Save training plan` first to create a new saved plan.
- LoRA files are auto-saved under `outputs/training/<plan-slug>/loras/`; there is no extra save button.
- Re-running the same saved plan can replace the same `.safetensors` filename.

Saved plan lifecycle tools:

- `Load into editor` copies a saved plan's settings back into the Builder form for review.
- `Duplicate as new plan` starts a safe branch with the same settings and a new name.
- `Delete saved plan` removes the saved plan card and generated handoff folder when you no longer need that run setup.
- If a loaded plan is edited, the form marks it as `unsaved edits`; saving creates an edited copy instead of silently overwriting the original.
- Saved LoRA outputs show copy/open actions so the generated `.safetensors` is easier to find from the UI.

Deleting a saved plan does not delete trained LoRA outputs under `outputs/training/<plan-slug>/`. Those are preserved so a cleanup click cannot accidentally throw away a successful `.safetensors`.

Current runner limits:

- one active training job at a time
- live log tail in the saved-plan card
- stop button uses Windows process termination for the active WSL process
- Wan/Musubi only for now
- no historical run database yet

## Training Settings In Plain Terms

These settings shape the generated training plan. They are not magic quality sliders. Higher numbers usually cost more time, memory, or overfitting risk.

- `Concept stack`: the concept is no longer one flat summary. Build it out of focused blocks so the generated captions know what should lead, what should support, and what should stay out.
- `Concept block type`: tells Chatty-lora what kind of lesson that one block is teaching, such as style, portrait, outfit, pose, composition, or motion.
- `Block role`: marks whether that block is the primary lesson, a supporting detail, or an avoid guardrail.
- `Trigger term`: the short distinct handle you will use later to call the LoRA concept.
- `Training intent`: a plain-language note about what that one block should teach.
- `Training preset`: chooses a starter personality for the run. Use `Balanced starter` until you have a reason not to.
- `Trigger phrase`: the rare phrase you will later type in prompts to call the LoRA concept.
- `Concept summary`: the plain-language target. This is where you say what should stay consistent.
- `Caption strategy`: controls where file descriptions come from. Better captions usually help, but tiny tests can start with filenames/source titles.
- `Rank`: how much capacity the LoRA has. Low rank is smaller and safer; high rank can learn more but can overfit, slow down, and use more memory.
- `Repeats`: how many times each file is shown per epoch. More repeats makes a small dataset shout louder, but can overcook repeated mistakes.
- `Epochs`: how many full passes the trainer makes through the dataset. More epochs means more learning and more risk of overfitting.
- `Training resolution`: the size the trainer tries to learn at. Higher resolution can preserve detail but increases memory and time sharply.
- `Batch size`: how many samples train at once. `1` is safest on consumer GPUs. Higher batch sizes need more VRAM.
- `Learning rate`: how hard each step pushes the LoRA. Too low learns slowly; too high can damage quality fast.
- `Validation split %`: holds back part of the dataset for checking instead of training. Useful later, but usually off for tiny smoke tests.

The visible defaults are deliberately conservative for the first Wan 2.1 / Musubi path. They are designed to prove the pipeline works before chasing maximum quality.

## Low-VRAM Wan Route

The first real Wan run proved a cautious path for an 8GB AMD/Radeon setup.

Chatty-lora now generates Wan/Musubi scripts with these assumptions:

- `cache_latents.sh` runs VAE latent caching on CPU.
- `cache_text.sh` runs T5 text-encoder caching on CPU without fp8.
- `launch.sh` still trains on the GPU.
- The training launch uses split attention, FP8-scaled Wan weights, input offload, and block swapping.
- Block swapping deliberately moves many Wan transformer blocks through CPU memory so dedicated VRAM does not have to hold everything at once.

In plain language: the caches may look CPU-heavy, and training may be slower than a big-VRAM workstation. That is intentional. It keeps the Wan lanes more survivable on consumer AMD hardware while still using the GPU for the actual training pass.

The first proven smoke test was:

- Wan 2.1 T2V 1.3B
- Musubi Tuner in WSL Ubuntu 24.04
- AMD ROCm/ROCDXG
- 4 short videos
- `512px`
- `17` frames
- batch size `1`
- rank `8`
- one epoch

## Wan Training Lanes

The first real training paths are intentionally narrow:

- Windows app
- WSL Ubuntu trainer
- AMD ROCm/ROCDXG for Radeon GPU compute
- PyTorch ROCm inside WSL
- Musubi Tuner
- Wan 2.1 T2V 1.3B

Why narrow? Because Wan LoRA training is still a fast-moving, fiddly space. A small pair of proven lanes is better than a dozen half-working options. Once this path is stable, future lanes can be added more safely.

The image visual lane deliberately reuses the same Wan 2.1 T2V model bundle and Musubi scripts. It creates an `image_metadata.jsonl` handoff instead of a `video_metadata.jsonl` handoff. In plain language: this is for teaching Wan visual identity/style foundations from still images, not for replacing a normal Stable Diffusion image trainer.

The Builder preflight card checks:
- Wan dependency files under `models/wan/dependencies/`
- WSL distro availability
- Musubi Tuner script availability
- the selected DiT file used by generated plans

The selected dataset also gets its own preflight readout before you save or run:
- whether the dataset has enough video clips or still images for the selected Wan lane
- whether captions and curation metadata are present
- whether files outside the selected lane will be ignored by the handoff
- clip duration and resolution summaries when `ffprobe` is available on Windows

The badges are intentionally plain language. `Smoke-test ready` means the dataset is good enough to prove the wiring, not necessarily good enough for a polished LoRA. Thin datasets can still be useful for pipeline tests, but serious training wants more clean, consistent examples.

The current generated defaults are conservative:
- `512px` is the safer first app/manual test on 8GB AMD/Radeon hardware
- `17` target frames
- `16.0` source FPS
- batch size `1`
- rank `8` for the smallest smoke test, with higher ranks left for later tuning

For the first real run, use a tiny dataset and choose `512px`, `1` epoch, and `batch size 1`.

## Setup Parts And Search Terms

The exact versions will change. The important thing is to get the matching family of parts, not blindly copy an old version number.

Model folders now follow a family-first layout:

- `models/wan/gguf/` for Wan GGUF inference files
- `models/wan/dependencies/` for Wan training dependencies like DiT, VAE, T5, and CLIP
- `models/flux/gguf/` for Flux GGUF inference files
- `models/flux/dependencies/` for future Flux support files
- `models/ai_assistant/gguf/` for local app-assistant GGUF models

Core app tools:
- Rust: search `rustup install windows rust`
- Node.js: search `nodejs lts windows download`
- Git: search `git for windows download`
- FFmpeg / ffprobe, optional but useful for dataset video duration checks: search `ffmpeg windows install path`

Windows / WSL / AMD stack:
- WSL: search `Microsoft install WSL Ubuntu 24.04`
- Ubuntu distro: search `wsl install Ubuntu-24.04`
- AMD Windows driver for WSL: search `AMD Software Adrenalin Edition WSL2 Radeon ROCm`
- ROCm on WSL: search `AMD ROCm Radeon WSL ROCDXG Ubuntu 24.04`
- ROCDXG bridge: search `ROCm librocdxg GitHub Quickstart`
- Windows SDK: search `Windows SDK download 10.0.26100`
- PyTorch ROCm WSL: search `AMD ROCm WSL PyTorch pip Ubuntu 24.04`

Trainer:
- Musubi Tuner: search `kohya-ss musubi-tuner GitHub`
- Musubi Wan docs: search `musubi-tuner Wan 2.1 2.2 docs wan.md`

Wan 2.1 model files for these Wan lanes:
- DiT: search `Comfy-Org Wan_2.1_ComfyUI_repackaged wan2.1_t2v_1.3B_bf16.safetensors`
- fallback DiT: search `Comfy-Org Wan_2.1_ComfyUI_repackaged wan2.1_t2v_1.3B_fp16.safetensors`
- VAE: search `Comfy-Org Wan_2.1_ComfyUI_repackaged wan_2.1_vae.safetensors`
- T5: search `Wan-AI Wan2.1-I2V-14B-720P models_t5_umt5-xxl-enc-bf16.pth`
- CLIP, optional for future I2V/reference work: search `Wan-AI Wan2.1-I2V-14B-720P models_clip_open-clip-xlm-roberta-large-vit-huge-14.pth`

Useful current source pages:
- Musubi Tuner: `https://github.com/kohya-ss/musubi-tuner`
- Musubi Wan docs: `https://github.com/kohya-ss/musubi-tuner/blob/main/docs/wan.md`
- Microsoft WSL install docs: `https://learn.microsoft.com/en-us/windows/wsl/install`
- AMD ROCm on Radeon / WSL docs: `https://rocm.docs.amd.com/projects/radeon-ryzen/en/latest/docs/install/installryz/wsl/howto_wsl.html`
- ROCm ROCDXG bridge: `https://github.com/ROCm/librocdxg`
- Comfy-Org Wan 2.1 DiT files: `https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/tree/main/split_files/diffusion_models`
- Comfy-Org Wan 2.1 VAE files: `https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/tree/main/split_files/vae`

The Wan training dependency files should land here:
- `models/wan/dependencies/dit/`
- `models/wan/dependencies/vae/`
- `models/wan/dependencies/t5/`
- `models/wan/dependencies/clip/`

Optional Wan GGUF inference files can live here:
- `models/wan/gguf/`

If a model page shifts, ask your preferred flagship AI for help. A useful prompt is:

```text
I am setting up Chatty-lora for Wan 2.1 T2V 1.3B LoRA training with Musubi Tuner on Windows + WSL Ubuntu 24.04 + AMD ROCm/ROCDXG. Please check the current official Musubi docs and Hugging Face model pages and tell me the current download links for the Wan 2.1 T2V 1.3B DiT, Wan 2.1 VAE, UMT5 text encoder, and optional CLIP encoder. Do not suggest Wan 2.2 or VACE unless I ask.
```

Another useful troubleshooting prompt is:

```text
I am on Windows with WSL Ubuntu 24.04 and an AMD Radeon GPU. PyTorch ROCm inside WSL should see the GPU, but training fails. Here is my error log. Please help me identify whether the issue is WSL, ROCDXG, ROCm runtime, PyTorch, Musubi Tuner, or model file placement.
```

The ecosystem moves quickly. Treat these docs as a working recipe, not a sacred scroll.

## Scoped Site-Fix Flow

The site-fix shell exists so one drifting source does not force a rewrite of the whole crawler.

The intended flow is:

1. Pick the broken source.
2. Open its shell.
3. Write:
   - issue summary
   - reproduction notes
   - patch notes / selector ideas
4. Draft a scoped proposal.
5. Save the proposal if you want a review trail.
6. Click `Review proposed fix`.
7. Review:
   - target file or source connection profile
   - backup path
   - before/after excerpt
   - diff summary
8. Click `Apply proposed fix` only if it looks correct.

The visible control row mirrors that order:
- `Draft scoped fix proposal` creates a plan only.
- `Review proposed fix` generates the scoped review and backup plan.
- `Apply proposed fix` actually writes the selected source fix.
- `Save site-fix brief` stores notes without applying anything.

If you already know the URL template or selector profile, you can paste it into `Patch notes / selector ideas` and go straight to `Review proposed fix`. You do not have to ask the helper to draft a proposal first.

Guardrails:
- one source at a time
- one adapter file or one source connection fix at a time
- crawler core stays out of bounds
- a backup is written before apply

For `generic_gallery_html` custom sources, apply does not rewrite Rust. It writes a validated connection fix into that one source entry in `config/sources.json`. That fix can include a URL template, a selector profile, or both. The generic adapter uses the saved source URL/profile on the next search before falling back to broad HTML scanning.

The connection-fix JSON can be pasted into `Patch notes / selector ideas`:

```json
{
  "base_url_template": "https://example.com/gallery/page/{page}",
  "item_selector": ".gallery-card",
  "media_selector": "img",
  "media_attribute": "src",
  "title_selector": "img",
  "title_attribute": "alt",
  "thumbnail_selector": "img",
  "thumbnail_attribute": "src",
  "link_selector": "a",
  "link_attribute": "href"
}
```

Use the URL pattern and selectors from the target site, then click `Review proposed fix` before applying. If the review shows anything outside that one source entry in `config/sources.json`, do not apply it.

## Source Fix Records

Chatty-lora stores two different histories on purpose:

- `Proposal history`
  - ideas and review drafts
  - not applied

- `Applied patch history`
  - actual adapter edits or source-profile updates that were applied
  - tied to backup-first writes

This makes it easier to see what was merely suggested versus what really changed.

## Good Defaults

For day-to-day use:
- search one or two sources at a time
- keep search terms specific
- curate in smaller batches first
- prepare the dataset shell before thinking about training

## License

This project is licensed under the GNU Affero General Public License v3.0 or later (`AGPL-3.0-or-later`). See [LICENSE](LICENSE) and the package metadata in [Cargo.toml](Cargo.toml).
