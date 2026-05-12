# Chatty-lora User Manual

This manual assumes **zero prior knowledge**.

If you have never built a LoRA before, never used a crawler before, and are not a coder, this is for you.

## What Chatty-lora Is

Chatty-lora is a local helper for making LoRA training less miserable.

In plain language, it helps you:
- find training material
- sort the material into a clean dataset folder
- set up a LoRA training plan
- check whether the first Wan training lane has the pieces it needs
- generate Musubi Tuner handoff files for a WSL training run
- launch the first Wan/Musubi training lane from a saved plan card
- keep track of source-specific fixes if a website changes how it works

The first built-in runner is narrow on purpose: saved Wan 2.1 / Musubi Tuner plans only. The manual WSL scripts are still shown as a fallback when you need to debug outside the app.

## What a LoRA Is

A LoRA is a small add-on that teaches a base model a narrow idea better than it knew before.

That idea might be:
- a visual style
- a character
- an object
- a location
- a motion or video concept
- an assistant persona

Instead of building a whole giant model from scratch, a LoRA teaches one smaller concept on top of an existing base model.

## The Two Main Pages

Chatty-lora has two main pages.

### Materials

This is where you search for material and build a dataset.

### Builder

This is where you turn that dataset into a saved LoRA training plan and, for the current Wan lanes, a Musubi Tuner handoff folder.

There is also a side helper panel that answers questions in plain language while you work.

## Before You Start

The app expects these folders:
- [`inputs/`](inputs/)
- [`outputs/`](outputs/)
- [`models/`](models/)
- [`runtime/`](runtime/)
- [`config/`](config/)

You do not need to understand every folder to use the app.

The most important ones are:
- `inputs/` for curated datasets
- `models/` for base model files
- `config/training/generated/` for generated Musubi handoff files

## Page 1: Materials

The `Materials` page is where you search for things like:
- reference images
- video clips
- other usable training material

### Recommended Material Collection Order

The most reliable way to use Chatty-lora is:

1. Gather media yourself.
2. Put the messy folder under `inputs/`.
3. Use `Local folder cleanup`.
4. Train from the cleaned dataset.

That is the primary path.

The web search/crawler is a helpful secondary feature. Treat it as:

```text
Nice when it works with your source.
Not the main way you should expect to collect everything.
```

This is intentional. The crawler is designed to be polite and limited, not a giant scraping machine that tries to bulldoze through every website.

### Source Registry

At the top-left is the source registry.

This is the list of websites Chatty-lora can search.

Each source has:
- a name
- a URL
- an adapter type
- a media capability
- an enabled/disabled checkbox

The source cards also include practical controls:
- `Open source` opens the source URL in your browser.
- `Open site fix shell` starts a source-specific troubleshooting note.
- `Remove from list` prunes a source from your local `config/sources.json`.

Removing a bundled source only changes your local curated source list. It does not delete code, and it does not affect anyone else's copy of Chatty-lora.

If a source is enabled, it can be included in search.

If a source is disabled, it will be ignored.

Adapter types mean:
- `Openverse images` and `Openverse audio` use the public Openverse API.
- `Wikimedia Commons` uses the Wikimedia API.
- `Generic gallery HTML` is a cautious best-effort scanner for common media links on a page.

For generic gallery sources, the URL can include placeholders:
- `{query}` for the search words
- `{page}` for the page number

Example:

```text
https://example.com/search?q={query}&page={page}
```

If you type a search term and do not use `{query}`, Chatty-lora appends a `q` query parameter automatically.

If you do not use `{page}`, Chatty-lora appends a `page` query parameter automatically.

Search terms are URL-encoded before they are placed into `{query}`. That means handles, hashtags, product codes, and awkward real-world labels are safer to use.

Examples:
- `@my_leather_shop`
- `journal cover #4598`
- `handmade wallet & strap`

This helps with custom source templates, but it does not bypass login walls, private posts, JavaScript-only feeds, or platform rules. For social media archives, the most reliable path is still a public listing/export page, a platform data export, or manually saved media under `inputs/`.

Generic gallery sources are useful for testing your own small galleries or simple pages, but they are not magic. If a site has unusual JavaScript, login walls, strange pagination, or hidden media links, use the site-fix shell to keep the troubleshooting scoped to that source.

The site-fix shell can now help with the boring URL detective work. For a `Generic gallery HTML` source, `Draft scoped fix proposal` fetches the selected source page, looks for obvious next/page links, and suggests a `base_url_template` if it can infer one. It may also suggest a selector profile from the sampled page.

Example:

```text
Detected page link:
https://example.com/images/category/page2

Suggested template:
https://example.com/images/category/page{page}
```

Nothing is applied from that inspection automatically. You still review the proposed JSON and diff before applying the fix.

### Search Term

This is simply what you want to look for.

Examples:
- `vintage uniforms`
- `rainforest frog`
- `gothic architecture`
- `kookaburra on telephone pole`

You can also leave the search box blank.

Blank search means `Browse mode`.

Browse mode asks selected sources for their first available media pages. This is useful when you want to see what a source exposes without picking a specific search phrase.

Important plain-language version:
- API sources like Openverse can usually return a general first page.
- Wikimedia can return a broad first page of files.
- Generic gallery sources can only see media linked from the fetched page or page window.
- Pointing at a homepage does not magically scan the whole website.
- For better generic-site coverage, use a real listing/search URL, add `{page}` if the site supports pages, or apply a source connection fix through the site-fix shell.

Chatty-lora does try one beginner-friendly rescue step. If search or Browse mode starts on a generic homepage and finds no media, it looks for obvious same-site gallery, media, feed, and sitemap links, then scans a few of those pages. If you typed a search term, Chatty-lora can also carry that term into obvious same-site search or gallery links that do not already have query parameters. This helps with simple sites where the homepage points to a gallery or search page.

It still will not solve every site:
- JavaScript-only galleries may hide links from the simple crawler.
- Login pages cannot be crawled politely.
- A homepage with no useful links gives Chatty-lora nowhere safe to go.
- Big sites usually need a proper search/listing URL, a `{page}` template, or a source profile.

### Honest Webcrawler Limits

This part is important.

Chatty-lora's crawler is deliberately cautious. It is designed this way to:
- reduce the chance that users get IP banned
- reduce the chance that a source site is hammered by accident
- keep searches reviewable instead of turning into a runaway download pile
- respect that websites are owned by other people and may have rules, rate limits, or anti-bot systems

That means web search will sometimes return nothing, even when you can see media in your browser.

Reasons outside Chatty-lora's control include:
- the site requires login
- the media is in a private feed
- the page is built by JavaScript after load
- media URLs expire quickly
- the site blocks automated requests
- the site uses CAPTCHA or bot detection
- the site's terms of service do not allow automated collection
- the source only exposes media through an internal API the crawler does not understand

This is not a failure of the whole app. It just means that source is not a good crawler target.

The practical rule is:
- Use `Local folder cleanup` as your dependable primary workflow.
- Use web search when a source plays nicely.
- Use the site-fix shell for simple public pages that only need a better URL template or selector profile.
- Do not expect Chatty-lora to bypass platform defenses, private accounts, or social-media walls.

### Media To Search

Under the search term are three media buttons:
- `Images`
- `Video`
- `Audio`

These buttons answer a different question than the source checkboxes.

Source checkboxes mean:
- where should Chatty-lora search?

Media buttons mean:
- what kind of material should Chatty-lora look for?

You can select more than one media type at the same time.

Examples:
- `Images` only: useful for still-image visual LoRA datasets.
- `Video` only: useful when you are hunting for motion clips.
- `Images` plus `Video`: useful for Wan visual foundations and video refinement.
- `Audio`: future-facing for audio material curation.

If a source cannot provide the selected media type, Chatty-lora skips it and explains that in the search notes. For example, an image-only source will be skipped during a video-only search.

### Why Results Load In Small Page Groups

Chatty-lora deliberately loads results in small `3-page` batches.

That is done to:
- keep the app responsive
- avoid unnecessary clutter
- be kinder to the source sites
- reduce the chance of a user getting blocked or rate-limited

### Preview Results

When you search, Chatty-lora shows preview cards.

These are not the final dataset yet. They are just a review step so you can choose what you want first.

### Selection Tray

When you tick preview items, they move into `Selection tray`.

Think of this as your shopping basket before download.

It helps you:
- review what you chose
- remove mistakes
- clear everything if you want to start again

### Dataset Folder Name

Once you have picked the items you want, give the dataset a folder name.

Example:
- `kookaburra-reference-set`

Chatty-lora then creates a dataset folder under `inputs/`.

### Curate Selected Into Dataset

When you click this button, Chatty-lora does the boring work:
- downloads the selected files
- names them consistently
- writes simple `.txt` sidecar captions beside saved files where possible
- creates a dataset folder
- writes a metadata manifest

This is what "curation" means in this app.

You choose the material. The app handles the grunt work.

### Local Folder Cleanup

Sometimes you already have the material.

Maybe you dragged a messy folder of product photos, phone videos, screenshots, or social export files into `inputs/` yourself.

Use `Local folder cleanup` when you want Chatty-lora to turn that jumble folder into a clean dataset without touching the original.

The flow is:

1. Put a folder under `inputs/`.
2. Click `Refresh scan`.
3. Pick that folder under `Jumble folder`.
4. Give the cleaned copy a dataset name.
5. Click `Clean folder into dataset`.

Chatty-lora creates a new dataset folder under `inputs/` with:
- media sorted into `images/`, `video/`, and `audio/`
- predictable filenames like `image-0001-product-name.jpg`
- simple `.txt` sidecar captions
- a `metadata.json` manifest

Original files are left alone. The cleaned copy is what you select on the `Builder` page.

This is also the recommended path for awkward sources like social media. If a platform makes crawling unreliable, use exports, screenshots, downloaded posts, phone backups, or manually saved media, then let Chatty-lora handle the naming and sorting tedium.

## Page 2: Builder

The `Builder` page turns a curated dataset into a reusable training plan.

For the current Wan training lanes, it also generates Musubi Tuner handoff files.

### Curated Dataset

This is the dataset you prepared from the `Materials` page.

If you just curated one, Chatty-lora will usually recommend it automatically.

The selected dataset area also shows a plain-language preflight panel.

It looks for:
- video or image count, depending on the selected Wan backend
- sidecar caption files
- curation metadata
- extra files that the selected lane will ignore
- clip duration and resolution for video plans, when `ffprobe` is available

For the Wan smoke-test lanes, `Smoke-test ready` means the dataset is enough to prove the pipeline can run. It does not mean the final LoRA will be amazing. A tiny set is useful for testing the machine, trainer, and file handoff. A stronger LoRA will usually need more clean, consistent material.

### Base Model

This is the model you want the future LoRA to sit on top of.

For now, the real training lane is:
- `Wan 2.1 T2V 1.3B`

Think of the base model as:
- the foundation model
- the thing your LoRA modifies later

Chatty-lora currently has two Wan foundations for this same model family:
- `Wan video lane`, which learns from clips and produces `video_metadata.jsonl`
- `Wan image visual lane`, which learns still-image identity, object, or style foundations and produces `image_metadata.jsonl`

The image visual lane is not a normal Stable Diffusion image trainer. It is a practical bridge for teaching the Wan family visual concepts from images before later video refinement.

### Training Backend

This tells Chatty-lora which trainer family the plan is shaped for.

The first real backend targets are:
- `Musubi Tuner / Wan 2.1 T2V 1.3B`
- `Musubi Tuner / Wan 2.1 T2V 1.3B / Image visual LoRA`
- `Musubi Tuner / Wan 2.1 T2V 14B`
- `AI Toolkit / Wan 2.2 TI2V 5B`

Choosing a backend does not start training by itself. It shapes the saved plan and generated WSL files. Once the plan card is saved and marked ready, the `Run this saved plan` button can launch the sequence.

The `Wan 2.1 T2V 14B` option is intentionally not framed as the default safe starter route. It uses the same Musubi handoff shape, but it is much heavier than the proven `1.3B` lane and should be treated as an intentional stronger-hardware experiment. Chatty-lora now squeezes that lane down on purpose for first-pass survival: very low resolution, fewer frames, tiny rank, and near-maximum block swap before you try to scale it back up. On the current WSL + ROCm test rig, the only `14B` route that reached live training used BF16-loaded weights instead of the earlier FP8 weight-cast path, and it still could not be validated end to end. On the current author test box, the likely limiter is system RAM plus WSL swap rather than the 8GB GPU itself, and the lane appears to want slightly more than a 32GB-class Windows setup for reliable completion. Proceed with open expectations, and please treat successful testing, refinements, and stronger-hardware reports as genuinely helpful contributions back to the project.

The new `AI Toolkit / Wan 2.2 TI2V 5B` option is the first planned non-Musubi Wan lane in Chatty-lora. It is being positioned as the more achievable eventual Wan 2.2 verification target than the heavier Wan 2.2 14B route. Today, Chatty-lora can detect the expected local bundle shape, build the dataset JSONL, and generate a first scaffold handoff folder, but you should still treat the launch recipe as early groundwork until the local AI Toolkit workflow is proven end to end.

This lane also expects a separate local AI Toolkit runtime checkout under `runtime/ai-toolkit/` or `runtime/ai_toolkit/`. Chatty-lora treats that runtime as a shared trainer bucket for future Diffusers-style lanes too, not just this Wan experiment. The current upstream AI Toolkit project explicitly covers Flux, SDXL, SD 1.5, Wan video models including `Wan2.2-TI2V-5B`, and even newer audio-capable families, so it is the natural home for future non-Musubi routes.

The Builder now helps more actively here:
- it groups base models by family
- it ranks compatible backends higher for the selected family
- it can auto-suggest a better backend when the chosen base-model family changes
- it shows a small state note so you can tell whether the current backend is `auto-suggested` or a `manual choice`

If you manually pick a backend and it still matches the selected family, Chatty-lora keeps that choice. If it becomes a family mismatch after a base-model change, Chatty-lora may switch back to a more compatible suggestion.

### Project Name

This is the name of your LoRA training plan.

Chatty-lora will suggest one, but you can change it.

### Concept Stack

The Builder no longer assumes one flat concept summary.

Instead, you build a `Concept stack` one block at a time.

Think of each block as one lesson:
- one main lesson
- one supporting detail
- one guardrail you do not want reinforced

The compact composer at the top stays simple:
1. pick a block type
2. pick a block role
3. enter a trigger term
4. write the training intent
5. write the concept details
6. click `Add concept block`

The card appears in the stack underneath.

You can then:
- expand or collapse it with `+` and `-`
- move it `Up` or `Down`
- `Edit` it inline
- `Duplicate` it and tweak the copy
- remove it with `X`

This keeps the form beginner-friendly while still letting one plan teach more than one idea cleanly.

### Block Type

Block type tells Chatty-lora what kind of lesson that block is trying to teach.

Examples in the UI:
- `Style / aesthetic`
- `Character / likeness`
- `Face / portrait`
- `Outfit / costume`
- `Object / product`
- `Location / environment`
- `Pose / action`
- `Motion pattern`
- `Composition / camera language`
- `Expression / mood`

Use the closest match. It mainly helps the guidance text and caption recipe stay aligned with what you are actually teaching.

### Block Role

Role tells Chatty-lora how important that block is.

The roles mean:
- `Primary lesson`: the main thing the LoRA should learn first
- `Supporting detail`: useful context that should help without taking over
- `Avoid / don't reinforce`: a saved reminder about recurring mistakes or distractions in the dataset

For the current Wan lane, avoid blocks are kept out of the positive caption recipe.

### Trigger Term

This is the special short phrase people will later use to call the LoRA concept.

Examples:
- `chatty_lora_style`
- `janet_character`
- `kookaburra_motion`

Beginner rule:
- keep it short
- keep it uncommon
- prefer one underscored term over a long normal-language phrase

### Training Intent

This is the plain-language sentence for what that one block is trying to teach.

Examples:
- `keep facial identity stable across angles`
- `preserve the same tropical paint texture and bright palette`
- `teach the recurring standing pose without overpowering the likeness`

This is not meant to describe the whole project. It is the purpose of one block.

### Concept Details

This is where you describe the visual lesson for that one block in plain language.

Examples:
- `A bright painterly tropical bird illustration style with bold color separation and soft textured brush edges`
- `A particular character with consistent hair shape, face structure, and recognizable markings`
- `A short wildlife motion concept with a kookaburra perched upright on a pole before hopping`

If you catch yourself writing "and also the outfit, and also the background, and also the camera angle", that is a clue to add another block instead of overstuffing one.

### Reusing Good Stacks

The transfer box under the stack is there so good concept setups do not have to be rebuilt by hand.

Buttons:
- `Export stack` writes the current concept stack into the transfer box and also copies it to the clipboard when the browser allows it
- `Import replace` swaps the current stack for the pasted one
- `Import append` adds pasted blocks onto the current stack
- `Clear transfer` clears the transfer box only

This is a practical reuse tool for local work, not a cloud sync system.

### Training Preset

This is a beginner-friendly starting shape for future LoRA settings.

You do not need to overthink it yet. Use the preset that feels closest to your goal and refine later.

### Caption Strategy

This tells the future project how to think about text descriptions for the files.

For now, the options are simple:
- use source titles
- use filenames only
- plan to caption manually later

### Training Settings In Plain English

The training settings are not "make it good" buttons.

They are closer to:
- how much room the LoRA has to learn
- how many times the trainer studies the dataset
- how hard each training step pushes
- how much memory and time the run is likely to use

The safest beginner rule is:

```text
Start with the defaults.
Run a tiny proof test.
Only raise one thing at a time.
```

#### Rank

Rank is the LoRA's learning capacity.

Plain version:
- low rank means smaller, safer, less flexible
- high rank means more room to learn, but more overfitting and memory risk

For the current Wan smoke-test lane, `rank 8` is intentionally cautious.

Use lower ranks when:
- the dataset is tiny
- you are testing the pipeline
- you want less VRAM pressure

Consider higher ranks later when:
- the dataset is cleaner and larger
- the concept is complex
- you have already proven the pipeline works

#### Repeats

Repeats means how many times each file is shown during one epoch.

Plain version:
- more repeats makes a small dataset louder
- too many repeats can make the model memorize flaws

If you only have a few files, repeats can help the trainer pay attention. But if the files are messy, repeated mess becomes learned mess.

#### Epochs

Epochs means how many full passes the trainer makes through the dataset.

Plain version:
- one epoch is one complete study pass
- more epochs means more learning
- too many epochs can overcook the LoRA

The first tiny run uses one epoch because the goal is proving the pipeline, not final quality.

#### Training Resolution

Resolution is the size the trainer works at.

Plain version:
- higher resolution can preserve more detail
- higher resolution costs more memory and time
- higher resolution can break consumer GPU runs quickly

For the current AMD/Wan route, `512px` is the sane first test. Treat `768px` and above as "later, once the small run works."

#### Batch Size

Batch size means how many samples the trainer processes at once.

Plain version:
- batch `1` is slow but safest
- higher batch sizes can be faster or smoother, but need more VRAM

On 8GB-class consumer GPUs, start with batch `1`.

#### Learning Rate

Learning rate is how hard the trainer pushes each step.

Plain version:
- too low learns slowly
- too high can wreck the concept quickly
- small datasets are easier to overcook

The default is a cautious starter value. If outputs become warped, repetitive, or too aggressively locked to the training set, learning rate may be one of the reasons.

#### Validation Split %

Validation split holds back part of the dataset for checking instead of training on it.

Plain version:
- `0%` means train on everything
- higher values reserve some files as a test set

For tiny smoke tests, leave this at `0%`. Validation becomes more useful when you have enough material that holding some back will not starve the trainer.

#### Presets

Presets are just starting shapes.

They do not guarantee quality. They choose a sane cluster of settings for a goal like:
- balanced first attempt
- stronger identity
- lighter style touch
- fast test pass

Use presets to get started, then judge results and adjust slowly.

### Wan Preflight Card

The Builder has a Wan preflight card for the current Wan training lanes.

It checks:
- whether the Wan dependency files are in `models/wan/dependencies/`
- whether WSL is available
- whether the Musubi Tuner scripts exist in the expected WSL folder
- which DiT file will be used for generated plans

If this card says something is missing, it usually means either:
- a file is in the wrong folder
- the file name is different enough that Chatty-lora does not recognize it
- WSL or Musubi is not installed where the app expects it

### Save Training Plan

This saves a reusable local training plan into `config/projects/`.

For the Wan/Musubi backends, it also writes a generated handoff folder into:

```text
config/training/generated/<plan-slug>/
```

That generated folder contains:
- `dataset.toml`
- `video_metadata.jsonl` for video plans, or `image_metadata.jsonl` for image plans
- `preflight.sh`
- `cache_latents.sh`
- `cache_text.sh`
- `launch.sh`
- `run_all.sh`
- `plan.json`
- `README.md`

The generated `plan.json` and per-plan `README.md` now also record:
- the training lane label
- the backend id
- whether the backend came from an `auto-suggested` match or a manual override

The saved plan card also shows a `Wan handoff` block.

That handoff block now repeats the saved backend-choice state as part of the readout, so the card, the generated files, and the Builder all tell the same story.

That block now has two ways to run:
- the `Run this saved plan` app button
- copyable PowerShell commands for manual fallback

The app runner automatically goes through:
- running preflight checks without training
- caching latents
- caching text
- starting training

If a stage fails, the runner stops there and shows the log tail in the saved-plan card.

The first runnable saved-plan card also shows an `ECG Window`. It is a small two-line activity graph:
- the CPU line shows cache/prep work, which can be busy while the GPU looks quiet
- the GPU line shows Radeon training bursts, similar to the activity spikes in Windows Task Manager

This is a quick heartbeat view, not a full profiler. It is meant to answer "is the machine doing anything?" while the training path moves between CPU-heavy and GPU-heavy stages.

The app runner uses the saved plan card you click. It does not use unsaved changes sitting in the form above it. If you change the concept type, project name, trigger phrase, resolution, rank, or other settings, click `Save training plan` first and then run the new saved card.

Saved plan buttons:
- `Load into editor` puts a saved plan back into the form so you can inspect or tweak it.
- `Duplicate as new plan` copies the saved settings into the form with a new name, ready to branch.
- `Delete saved plan` removes a saved card and its generated handoff folder when the Builder panel is getting cluttered.
- If the form says `unsaved edits`, the saved card below is still the original. Click `Save training plan` or `Save edited copy` to make a new runnable card.

Successful LoRA files are auto-saved under:

```text
outputs/training/<plan-slug>/loras/
```

There is no extra save button. If you run the same saved plan again, it may replace the same `.safetensors` filename. The saved output row can copy the path or ask Windows Explorer to open/select the file.

Deleting a saved training plan does not delete those trained LoRA outputs. It only removes the saved plan JSON and generated handoff files. If you want to reclaim disk space from old trained outputs, delete the matching folder under `outputs/training/` manually after you are sure you do not need it.

The manual commands are still useful when:
- you want to debug directly in WSL
- you want to rerun only one generated script
- the UI runner is not enough information for a weird driver or Musubi error

### Low-VRAM Wan Settings

The first working Wan path is tuned for cautious consumer hardware, especially 8GB AMD/Radeon cards.

Chatty-lora's generated Wan scripts do a slightly unusual thing on purpose:
- `cache_latents.sh` uses CPU for VAE latent caching
- `cache_text.sh` uses CPU for T5 text-encoder caching
- `launch.sh` uses the GPU for training
- training uses split attention, FP8-scaled Wan weights, input offload, and block swapping

Plain English version:

The app tries not to make the GPU hold every heavy piece at the same time. Some preparation steps happen on CPU, and training swaps many Wan blocks through CPU memory while the Radeon does the actual learning work.

This can be slower than a big dedicated-VRAM card. That is normal. Slower and working is much better than fast and exploding at step one.

The first proven tiny test used:
- `512px`
- `17` frames
- batch size `1`
- rank `8`
- one epoch
- four short videos

If you later raise resolution, frame count, rank, epochs, or dataset size, expect memory pressure and training time to climb.

## First Training Lane

The first real training lanes are intentionally narrow:
- Windows app
- WSL Ubuntu trainer
- AMD ROCm / ROCDXG for Radeon GPU compute
- PyTorch ROCm inside WSL
- Musubi Tuner
- Wan 2.1 T2V 1.3B

This is not because other paths are impossible. It is because Wan LoRA training is fiddly, fast-moving, and easy to break. One proven model family with a video lane and an image visual lane beats ten half-working lanes.

Once this lane is stable, other model families and backends can be added more safely.

## App Runner And Manual Training Handoff

After saving a Wan/Musubi plan, the simplest route is to use the `Run this saved plan` button on the saved plan card.

If you need to debug manually, open WSL and go to the generated handoff folder.

The intended manual order is:

```bash
./preflight.sh
./cache_latents.sh
./cache_text.sh
./launch.sh
```

Or, once you trust the setup:

```bash
./run_all.sh
```

For the first real run, keep it tiny:
- small dataset
- `512px`
- `17` frames
- batch size `1`
- rank `8`
- one epoch

The goal of the first run is not to make a masterpiece. The goal is to prove the pipeline works end-to-end.

## What You Need For The First Wan Lane

Exact versions change. The safest way to think about setup is:

You want the current matching version family of each part, not a random old file from a forum thread.

Chatty-lora now uses a family-first model folder layout:
- `models/wan/gguf/` for Wan GGUF inference files
- `models/wan/dependencies/` for Wan training dependencies like DiT, VAE, T5, and CLIP
- `models/flux/gguf/` for Flux GGUF inference files
- `models/flux/dependencies/` for future Flux support files
- `models/ai_assistant/gguf/` for local app-assistant GGUF models

The `ai_assistant` family is for regular GGUFs that pilot Chatty-lora itself, such as helper chat and web-crawl/support workflows. Those models do not participate in LoRA training and are not meant to be used as Builder training base models.

This family-first layout is mirrored in the app too. The Materials summary, Builder base-model picker, backend compatibility guidance, and training-lane labels are all using the same family-based groundwork instead of each inventing their own folder assumptions.

### Core App Tools

Use these search terms:
- `rustup install windows rust`
- `nodejs lts windows download`
- `git for windows download`
- `ffmpeg windows install path`

FFmpeg is optional for the app itself, but its `ffprobe` tool lets the dataset preflight panel read video duration, resolution, and FPS from Windows before you run a training plan.

### Windows, WSL, And AMD Compute

Use these search terms:
- `Microsoft install WSL Ubuntu 24.04`
- `wsl install Ubuntu-24.04`
- `AMD Software Adrenalin Edition WSL2 Radeon ROCm`
- `AMD ROCm Radeon WSL ROCDXG Ubuntu 24.04`
- `ROCm librocdxg GitHub Quickstart`
- `Windows SDK download 10.0.26100`
- `AMD ROCm WSL PyTorch pip Ubuntu 24.04`

In plain language, this stack is:
- Windows runs the Chatty-lora app
- WSL runs the Linux training tools
- AMD's WSL driver and ROCDXG bridge let Linux see the Radeon GPU
- PyTorch ROCm is the Python machine-learning layer
- Musubi Tuner does the actual Wan LoRA training

### Trainer

Use these search terms:
- `kohya-ss musubi-tuner GitHub`
- `musubi-tuner Wan 2.1 2.2 docs wan.md`

### Wan 2.1 Model Files

Use these search terms:
- `Comfy-Org Wan_2.1_ComfyUI_repackaged wan2.1_t2v_1.3B_bf16.safetensors`
- `Comfy-Org Wan_2.1_ComfyUI_repackaged wan2.1_t2v_1.3B_fp16.safetensors`
- `Comfy-Org Wan_2.1_ComfyUI_repackaged wan2.1_t2v_14B_bf16.safetensors`
- `Comfy-Org Wan_2.1_ComfyUI_repackaged wan2.1_t2v_14B_fp16.safetensors`
- `Comfy-Org Wan_2.1_ComfyUI_repackaged wan_2.1_vae.safetensors`
- `Wan-AI Wan2.1-I2V-14B-720P models_t5_umt5-xxl-enc-bf16.pth`
- `Wan-AI Wan2.1-I2V-14B-720P models_clip_open-clip-xlm-roberta-large-vit-huge-14.pth`

Useful current source pages:
- `https://github.com/kohya-ss/musubi-tuner`
- `https://github.com/kohya-ss/musubi-tuner/blob/main/docs/wan.md`
- `https://learn.microsoft.com/en-us/windows/wsl/install`
- `https://rocm.docs.amd.com/projects/radeon-ryzen/en/latest/docs/install/installryz/wsl/howto_wsl.html`
- `https://github.com/ROCm/librocdxg`
- `https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/tree/main/split_files/diffusion_models`
- `https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/tree/main/split_files/vae`
- `https://huggingface.co/Wan-AI/Wan2.1-I2V-14B-720P/tree/main`
- `https://huggingface.co/DeepBeepMeep/Wan2.1/tree/main`

Put the Wan training dependency files here:
- `models/wan/dependencies/dit/`
- `models/wan/dependencies/vae/`
- `models/wan/dependencies/t5/`
- `models/wan/dependencies/clip/`

Optional Wan GGUF inference files can live here:
- `models/wan/gguf/`

For the current training lane, the CLIP file is optional. It is kept because it is useful for future image/video reference work.

## Ask An AI Helper If You Get Stuck

This setup crosses Windows, Linux, AMD drivers, Python, PyTorch, Musubi, and Hugging Face. That is a lot of moving pieces. Getting stuck does not mean you are bad at computers. It means the stack is spicy.

If you get lost, ask ChatGPT or your preferred flagship AI for help.

A good model-finding prompt is:

```text
I am setting up Chatty-lora for Wan 2.1 T2V 1.3B LoRA training with Musubi Tuner on Windows + WSL Ubuntu 24.04 + AMD ROCm/ROCDXG. Please check the current official Musubi docs and Hugging Face model pages and tell me the current download links for the Wan 2.1 T2V 1.3B DiT, Wan 2.1 VAE, UMT5 text encoder, and optional CLIP encoder. Do not suggest Wan 2.2 or VACE unless I ask.
```

A good troubleshooting prompt is:

```text
I am on Windows with WSL Ubuntu 24.04 and an AMD Radeon GPU. PyTorch ROCm inside WSL should see the GPU, but training fails. Here is my error log. Please help me identify whether the issue is WSL, ROCDXG, ROCm runtime, PyTorch, Musubi Tuner, model file placement, or my generated training command.
```

Paste the exact error log. Do not paraphrase it. Error logs are ugly, but they are useful ugly.

## Helper Chat

The right-side helper panel is there to answer practical questions while you work.

On `Materials`, it can help with things like:
- why a search returned nothing
- whether you have selected enough material
- whether you should use fewer sources at once

On `Builder`, it can help with things like:
- what rank means
- how repeats and epochs work
- whether the dataset looks too small or too mixed
- which part of the Wan setup is missing

It is designed to be useful, not magical.

## Scoped Site-Fix Shell

This is the more advanced maintenance area on the `Materials` page.

You only need it if a particular source site starts behaving oddly.

Examples:
- empty results
- broken thumbnails
- wrong media links
- pagination drift
- metadata fields moving around

### Why This Exists

Websites change.

When one site changes, we do not want to rewrite the whole crawler.

So Chatty-lora keeps fixes tightly scoped:
- one source
- one adapter file or one source connection fix
- no crawler-core sprawl

### The Site-Fix Flow

1. Pick the broken source.
2. Open the source shell.
3. Describe the problem.
4. Add reproduction notes.
5. Add patch notes or selector ideas if you have them.
6. Draft a scoped proposal.
7. Review the proposal.
8. Optionally save the proposal snapshot.
9. Click `Review proposed fix`.
10. Review the target, backup path, before/after excerpt, and diff summary.
11. Click `Apply proposed fix` only if it looks right.

The main buttons are deliberately separate:
- `Draft scoped fix proposal` creates a plan only.
- `Review proposed fix` shows what Chatty-lora intends to change and where the backup will go.
- `Apply proposed fix` is the step that actually writes the source-specific fix.
- `Save site-fix brief` saves your notes without applying anything.

You do not have to use the proposal drafter every time. If you already know the URL template or selector profile you want, paste it into `Patch notes / selector ideas`, click `Review proposed fix`, then apply only after the review looks correct.

For a custom `Generic gallery HTML` source, the safest fix is usually not a code edit. Chatty-lora can apply a small source connection fix instead. That fix can tell the generic adapter which URL pattern to use, which CSS selectors to use for this one site, or both.

Paste a profile like this into `Patch notes / selector ideas`, then replace the example URL and selectors with the site's real pattern:

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

Plain-language meaning:
- `base_url_template` is the exact paging/search URL pattern Chatty-lora should use for this source. Use `{page}` where the page number changes, and `{query}` where the search term belongs.
- `item_selector` is the repeating card or result wrapper.
- `media_selector` is the image, video, audio, or download link inside the card.
- `media_attribute` is where the real file URL lives, usually `src`, `href`, `data-src`, `srcset`, or `poster`.
- `title_selector` and `title_attribute` help name the preview.
- `thumbnail_selector` and `thumbnail_attribute` help show a preview image.
- `link_selector` and `link_attribute` point back to the source/detail page.

When applied, this changes only that source entry in `config/sources.json`. It can update the source URL template, the source selector profile, or both. It does not rewrite crawler core or unrelated sources.

### Proposal History Vs Applied Patch History

These are intentionally separate.

`Proposal history` means:
- draft ideas
- not necessarily applied

`Applied patch history` means:
- real adapter edits or source-profile updates
- backup-first writes

This distinction helps you understand what was merely suggested versus what really changed.

## Respectful Crawling

Chatty-lora is not trying to be a scraping monster.

Its search flow is intentionally conservative:
- small preview batches
- local review before download
- adapter-based source handling
- source-specific fixes instead of giant crawler rewrites

The goal is to reduce friction, not brute-force the internet.

The dependable workflow is still manual material collection plus `Local folder cleanup`. Web search is a convenience layer for friendly public sources, APIs, and simple galleries. If a source blocks bots, hides media behind login, relies on browser-only JavaScript, or starts returning junk, the correct answer is often to collect the material manually rather than trying to fight the site.

## Typical Beginner Workflow

If you are brand new, use this order:

1. Open `Materials`.
2. Enable one or two sources.
3. Search with a specific phrase.
4. Tick a handful of good results.
5. Review them in `Selection tray`.
6. Curate them into a named dataset.
7. Switch to `Builder`.
8. Choose the curated dataset.
9. Choose the Wan/Musubi backend target.
10. Build at least one concept block: choose a type, role, trigger term, training intent, and concept details, then click `Add concept block`.
11. Save the training plan.
12. Check the generated handoff folder.
13. Click `Run this saved plan`, or run the scripts manually in WSL if you need the fallback route.

## Practical Beginner Tips

- Start with smaller, cleaner datasets rather than giant messy ones.
- Fewer good files are often better than many bad ones.
- Keep search terms specific.
- Use the helper panel when you get stuck.
- Only use the site-fix shell when a source is clearly drifting.
- Treat proposal drafts as drafts, not as truth.
- Treat setup docs as a working recipe, not a sacred scroll.

## What To Ignore For Now

If you are new, you can safely ignore:
- adapter implementation details
- backup file paths
- patch diff details
- advanced training tuning

Those are there for power users and future workflow depth.

For a beginner, the important thing is:
- find material
- curate it cleanly
- save a training plan
- prove the first tiny training handoff

## Current Project State

Chatty-lora is already useful for:
- respectful search
- curation
- project setup
- Wan/Musubi handoff generation
- source-adapter maintenance

It is not yet a one-click training dashboard.

That is deliberate. We are building it in logical order.

## License

This project uses the GNU Affero General Public License v3.0 or later (`AGPL-3.0-or-later`).

See:
- [Cargo.toml](Cargo.toml)
- [LICENSE](LICENSE)
