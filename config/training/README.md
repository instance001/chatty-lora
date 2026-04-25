# Training Handoff Files

This folder is for trainer-facing config, not downloaded model weights.

Model weights belong under:

```text
models/
```

## Generated Wan / Musubi Plans

When you save a Builder plan for one of the current Wan/Musubi lanes, Chatty-lora writes a handoff folder here:

```text
generated/<plan-slug>/
```

Each generated folder contains:

- `dataset.toml`
- `video_metadata.jsonl` for video-lane plans, or `image_metadata.jsonl` for image-visual-lane plans
- `preflight.sh`
- `cache_latents.sh`
- `cache_text.sh`
- `launch.sh`
- `run_all.sh`
- `plan.json`
- `README.md`

## What Each File Does

- `dataset.toml` tells Musubi where the curated dataset lives and how to read it.
- `video_metadata.jsonl` or `image_metadata.jsonl` gives Musubi the per-item metadata it needs for the selected lane.
- `preflight.sh` checks generated files, model paths, video metadata, Musubi, PyTorch ROCm GPU access, `ffmpeg`, `ffprobe`, and shell syntax without starting training.
- `cache_latents.sh` prepares latent cache files before training.
- `cache_text.sh` prepares text-encoder cache files before training.
- `launch.sh` starts the actual LoRA training command.
- `run_all.sh` runs the cache steps and training step in order.
- `plan.json` keeps Chatty-lora's saved settings beside the generated trainer files, including lane metadata and whether the backend was auto-suggested or manually overridden.
- `README.md` is a per-plan quick note for the generated folder, including the selected lane, backend id, and backend-choice mode.

## Manual Run Order

The safe first-run order inside WSL is:

```bash
./preflight.sh
./cache_latents.sh
./cache_text.sh
./launch.sh
```

Once the setup is proven, you can use:

```bash
./run_all.sh
```

`run_all.sh` runs `preflight.sh` first.

The Chatty-lora UI runner uses the same generated scripts, but runs them as visible stages from the saved training plan card:

1. preflight
2. cache latents
3. cache text
4. train

If one stage fails, later stages are not started.

For the first tiny test, prefer:

- `512px`
- `17` frames
- batch size `1`
- rank `8`
- one epoch
- a small curated dataset

The first run is a pipeline test. It does not need to produce the perfect LoRA.

## Low-VRAM Wan Notes

The generated Wan/Musubi scripts are intentionally conservative for the first working lane.

- `cache_latents.sh` runs VAE latent caching on CPU.
- `cache_text.sh` runs T5 text-encoder caching on CPU without fp8.
- `launch.sh` still trains on the GPU.
- The training command uses split attention, FP8-scaled Wan weights, input offload, and block swapping.

Block swapping means Musubi does not try to keep every Wan transformer block in dedicated VRAM at the same time. On an 8GB AMD/Radeon card this can be the difference between a clean tiny run and an OOM during the backward pass.

Expect this route to be slower than a high-VRAM GPU setup. That tradeoff is intentional for the first lane.

## WSL And ROCm Notes

The generated scripts are written for the current first training lane:

- Windows app
- WSL Ubuntu
- AMD ROCm / ROCDXG
- PyTorch ROCm
- Musubi Tuner
- Wan 2.1 T2V 1.3B

Chatty-lora includes the ROCm library hints we currently need in the generated scripts:

```bash
export LD_LIBRARY_PATH=/opt/rocm/lib:/opt/rocm-7.2.2/lib:/usr/local/lib:$LD_LIBRARY_PATH
export HSA_ENABLE_DXG_DETECTION=1
export TORCH_ROCM_AOTRITON_ENABLE_EXPERIMENTAL=1
```

If your ROCm version changes, these paths may need to be updated later.

## If A Generated Script Fails

Keep the full error log. Do not shorten it.

Useful things to check:

- Does WSL see the GPU?
- Does PyTorch ROCm return `True` for `torch.cuda.is_available()`?
- Does Musubi Tuner exist at the expected WSL path?
- Are the Wan model files in `models/wan/dependencies/`?
- Did the dataset folder contain usable files?

This area is intentionally manual for now. Once the command path is boring and repeatable, the web UI can grow run/cancel/log controls safely.
