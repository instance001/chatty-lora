# Wan Dependency Folder

This folder holds the training dependencies for Chatty-lora's current Wan/Musubi lanes.

Right now that means:

```text
Musubi Tuner / Wan 2.1 T2V 1.3B
Musubi Tuner / Wan 2.1 T2V 1.3B / Image visual LoRA
Musubi Tuner / Wan 2.1 T2V 14B
```

The `1.3B` lanes are still the proven starter path. The `14B` lane is present as an exploratory heavier route, not the default beginner recommendation. On the current app author's WSL + ROCm test rig, the only `14B` handoff that reached live training used BF16-loaded weights rather than the earlier FP8 weight-cast path, and it still could not be validated end to end. The current evidence suggests the limiter is system RAM plus WSL swap more than the 8GB GPU itself, and that this lane likely wants slightly more than a 32GB-class Windows box for reliable completion. Proceed with open expectations. If you test, refine, or improve the route on stronger hardware, that feedback is welcomed and appreciated.

## Family Layout

The Wan family now uses this layout:

```text
models/wan/
  gguf/
  dependencies/
    dit/
    vae/
    t5/
    clip/
```

This `README.md` lives inside `models/wan/dependencies/`, which is the training-dependency side of that layout.

## Required Files

### DiT

Put the DiT files in:

```text
models/wan/dependencies/dit/
```

#### For the proven `1.3B` lanes

Preferred:

```text
wan2.1_t2v_1.3B_bf16.safetensors
```

Fallback:

```text
wan2.1_t2v_1.3B_fp16.safetensors
```

Some downloads may use this capitalization instead:

```text
Wan2_1-T2V-1_3B_bf16.safetensors
Wan2_1-T2V-1_3B_fp16.safetensors
```

#### For the exploratory `14B` lane

Preferred:

```text
wan2.1_t2v_14B_bf16.safetensors
```

Fallback:

```text
wan2.1_t2v_14B_fp16.safetensors
```

Chatty-lora currently looks for those exact `14B` names. If a provider renames the files later, keep the `Wan 2.1 T2V 14B` identity obvious in the filename and we can extend detection.

For `1.3B`, Chatty-lora's preflight already accepts the known capitalization variants. If a provider renames files again, keep the model family obvious in the name and we can add another detection pattern.

### VAE

Put this in:

```text
models/wan/dependencies/vae/
```

Expected file:

```text
wan_2.1_vae.safetensors
```

### T5 Text Encoder

Put this in:

```text
models/wan/dependencies/t5/
```

Expected file:

```text
models_t5_umt5-xxl-enc-bf16.pth
```

### CLIP Encoder

Put this in:

```text
models/wan/dependencies/clip/
```

Expected file:

```text
models_clip_open-clip-xlm-roberta-large-vit-huge-14.pth
```

For the current T2V lanes, CLIP is not the most important piece. We keep it here because it is part of the wider Wan 2.1 toolchain and useful for future I2V/reference work.

## Optional GGUF Files

Put local inference GGUF files in:

```text
models/wan/gguf/
```

Examples:

```text
wan2.1_t2v_1.3b-q4_0.gguf
wan2.1-t2v-14b-Q4_K_M.gguf
```

These GGUF inference copies are not used by the Musubi training path. They live in the family `gguf` bucket for convenience, but they are not required for LoRA training.

## Where To Find The Files

Exact pages move. Search terms are more durable than hard-coded old download buttons.

Recommended search terms:

- `Comfy-Org Wan_2.1_ComfyUI_repackaged split_files diffusion_models wan2.1_t2v_1.3B_bf16.safetensors`
- `Comfy-Org Wan_2.1_ComfyUI_repackaged split_files diffusion_models wan2.1_t2v_1.3B_fp16.safetensors`
- `Comfy-Org Wan_2.1_ComfyUI_repackaged split_files diffusion_models wan2.1_t2v_14B_bf16.safetensors`
- `Comfy-Org Wan_2.1_ComfyUI_repackaged split_files diffusion_models wan2.1_t2v_14B_fp16.safetensors`
- `Comfy-Org Wan_2.1_ComfyUI_repackaged split_files vae wan_2.1_vae.safetensors`
- `Wan-AI Wan2.1-I2V-14B-720P models_t5_umt5-xxl-enc-bf16.pth`
- `Wan-AI Wan2.1-I2V-14B-720P models_clip_open-clip-xlm-roberta-large-vit-huge-14.pth`
- `DeepBeepMeep Wan2.1 models_t5_umt5-xxl-enc-bf16.pth`
- `DeepBeepMeep Wan2.1 models_clip_open-clip-xlm-roberta-large-vit-huge-14.pth`

Helpful source pages:

- Musubi Wan docs: `https://github.com/kohya-ss/musubi-tuner/blob/main/docs/wan.md`
- Comfy-Org Wan 2.1 DiT files: `https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/tree/main/split_files/diffusion_models`
- Comfy-Org Wan 2.1 VAE files: `https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/tree/main/split_files/vae`
- Wan-AI encoder source named by Musubi: `https://huggingface.co/Wan-AI/Wan2.1-I2V-14B-720P/tree/main`
- DeepBeepMeep mirror/searchable bundle: `https://huggingface.co/DeepBeepMeep/Wan2.1/tree/main`

## Beginner Advice

If Hugging Face shows a folder tree instead of a big obvious download button, that is normal.

Usually you need to:

1. Open `Files and versions`.
2. Go into the relevant folder.
3. Click the exact file.
4. Use the small download icon or copy/download link.

For very large files, the command line is often less painful than browser downloads. If you use Hugging Face CLI, download into a temporary folder first, then move only the files listed above into this model folder.

## Ask An AI Helper If You Get Stuck

Useful prompt:

```text
I am setting up Chatty-lora's Wan 2.1 Musubi training lanes. Please find the current Hugging Face pages for the Wan 2.1 T2V 1.3B DiT, the Wan 2.1 T2V 14B DiT, the Wan 2.1 VAE, the UMT5 text encoder, and the optional CLIP encoder. I need filenames and which local folder each file should go in. Please separate the 1.3B and 14B DiT names clearly, and avoid Wan 2.2 or VACE unless I ask.
```

The important thing is matching the lane on purpose:

```text
Wan 2.1 T2V 1.3B
Wan 2.1 T2V 14B
```

Do not mix random Wan 14B, Wan 2.2, VACE, Fun Control, or inference-only GGUF files into the required training slots unless they match the lane you actually intend to run.
