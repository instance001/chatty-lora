# Chatty-lora - Handshake

## Module identity

- **module_id**: `chatty_lora`
- **display_name**: `Chatty-lora`

## What this module is for

Chatty-lora is the local LoRA-building department. It handles respectful material search, manual or assisted dataset cleanup, training-plan assembly, backend/lane selection, and source-specific site-fix review before a training run is handed off or launched.

## Inputs this module expects

- A target concept, character, style, object, motion pattern, or training goal
- Dataset constraints: image/video/audio type, quality bar, volume, caption quality, and curation rules
- Source choices or local folders to clean into datasets
- Base model family, backend target, and hardware/runtime constraints
- Trigger phrase, concept-stack blocks, and any guardrails for the training plan
- If troubleshooting sources: a concrete failing source plus reproduction notes

## Outputs this module produces

- Curated dataset folders under `inputs/`
- Saved training plans under `config/projects/`
- Generated trainer handoff files under `config/training/generated/`
- Training logs and outputs under `outputs/training/`
- Source-fix notes, proposals, previews, and apply history under `config/source-fixes/`

## Operating rules / preferences

- Tone: concise, practical, safety-minded
- Risk level: medium
- Default tags to use in logs: lora, dataset, training, source_fix, backend
- Preferred file naming: mention dataset name, model family, and lane/backend when relevant

## Suspend rundown template

> **Status:** Current dataset/training-plan state and next training or source-fix decision are updated.
> **What changed:** Search, curation, builder settings, or source-fix work moved the plan forward and produced a clearer dataset or handoff state.
> **Open questions:** Confirm whether the dataset is ready, whether the backend/lane choice is correct, and whether any source issue still blocks progress.
> **Next action:** Continue curation, save/adjust the plan, launch the next training attempt, or review/apply the pending source fix.
> **Artifacts:** `inputs/`, `config/projects/`, `config/training/generated/`, `outputs/training/`, `config/source-fixes/`

## Cold log envelope hints

- `module_id`: `chatty_lora`
- `event_type`: `suspend_rundown`
- `summary`: one short handoff paragraph focused on dataset readiness, chosen lane/backend, and the next blocking step
- `tags`: `lora`, `dataset`, `training`, `source_fix`, `backend`
- `payload_json`: optional dataset name, model family, backend, and generated-path details

## Portable bridge note

This module is being hosted inside ChattyCog as a docked web dashboard. The hosted UI remains Chatty-lora's own app; ChattyCog should treat the bridge as optional handoff telemetry rather than as the module's primary state store.
