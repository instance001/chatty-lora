use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    backend_registry, lane_registry, model_registry,
    state::ProjectPaths,
    types::{TrainingBackendSummary, WanModelFileStatus, WanTrainingDefaults, WanTrainingStatus},
};

pub const WSL_DISTRO: &str = "Ubuntu-24.04";
pub const WSL_MUSUBI_ROOT: &str = "~/train_runtime/musubi-tuner";
pub const WSL_ENV_PREFIX: &str = "export LD_LIBRARY_PATH=/opt/rocm/lib:/opt/rocm-7.2.2/lib:/usr/local/lib:${LD_LIBRARY_PATH:-}; export HSA_ENABLE_DXG_DETECTION=1; export TORCH_ROCM_AOTRITON_ENABLE_EXPERIMENTAL=1";
const WAN22_TI2V_5B_DIFFUSERS_CANDIDATES: &[&str] = &[
    "models/wan/diffusers/Wan2.2-TI2V-5B-Diffusers",
    "models/wan/diffusers/wan2.2-ti2v-5b-diffusers",
    "models/wan/diffusers/Wan2.2_TI2V_5B_Diffusers",
];

#[derive(Clone, Copy)]
struct WanFileDefinition {
    primary_relative_path: &'static str,
    legacy_relative_paths: &'static [&'static str],
    label: &'static str,
    role: &'static str,
    required: bool,
}

const WAN_FILES: &[WanFileDefinition] = &[
    WanFileDefinition {
        primary_relative_path: "models/wan/dependencies/dit/Wan2_1-T2V-1_3B_bf16.safetensors",
        legacy_relative_paths: &["models/wan21_t2v_1_3b/dit/Wan2_1-T2V-1_3B_bf16.safetensors"],
        label: "Wan 2.1 T2V 1.3B DiT BF16",
        role: "Diffusion model",
        required: true,
    },
    WanFileDefinition {
        primary_relative_path: "models/wan/dependencies/dit/wan2.1_t2v_1.3B_fp16.safetensors",
        legacy_relative_paths: &["models/wan21_t2v_1_3b/dit/wan2.1_t2v_1.3B_fp16.safetensors"],
        label: "Wan 2.1 T2V 1.3B DiT FP16 fallback",
        role: "Diffusion model fallback",
        required: false,
    },
    WanFileDefinition {
        primary_relative_path: "models/wan/dependencies/dit/wan2.1_t2v_14B_bf16.safetensors",
        legacy_relative_paths: &[],
        label: "Wan 2.1 T2V 14B DiT BF16",
        role: "Diffusion model",
        required: true,
    },
    WanFileDefinition {
        primary_relative_path: "models/wan/dependencies/dit/wan2.1_t2v_14B_fp16.safetensors",
        legacy_relative_paths: &[],
        label: "Wan 2.1 T2V 14B DiT FP16 fallback",
        role: "Diffusion model fallback",
        required: false,
    },
    WanFileDefinition {
        primary_relative_path: "models/wan/dependencies/t5/models_t5_umt5-xxl-enc-bf16.pth",
        legacy_relative_paths: &["models/wan21_t2v_1_3b/t5/models_t5_umt5-xxl-enc-bf16.pth"],
        label: "UMT5 XXL text encoder BF16",
        role: "Text encoder",
        required: true,
    },
    WanFileDefinition {
        primary_relative_path: "models/wan/dependencies/vae/wan_2.1_vae.safetensors",
        legacy_relative_paths: &["models/wan21_t2v_1_3b/vae/wan_2.1_vae.safetensors"],
        label: "Wan 2.1 VAE",
        role: "VAE",
        required: true,
    },
    WanFileDefinition {
        primary_relative_path: "models/wan/dependencies/clip/models_clip_open-clip-xlm-roberta-large-vit-huge-14.pth",
        legacy_relative_paths: &[
            "models/wan21_t2v_1_3b/clip/models_clip_open-clip-xlm-roberta-large-vit-huge-14.pth",
        ],
        label: "Wan CLIP vision encoder",
        role: "I2V / future reference support",
        required: false,
    },
];

#[allow(dead_code)]
pub fn scan_backends(paths: &ProjectPaths) -> Vec<TrainingBackendSummary> {
    scan_backends_with_wan_status(paths, scan_wan_training(paths))
}

pub fn scan_backends_with_wan_status(
    paths: &ProjectPaths,
    wan_status: WanTrainingStatus,
) -> Vec<TrainingBackendSummary> {
    let mut backends = vec![
        musubi_wan_video_backend(wan_status.clone()),
        musubi_wan_image_backend(wan_status),
        musubi_wan_14b_backend(scan_wan_training(paths)),
        ai_toolkit_wan22_5b_backend(paths, backend_registry::AI_TOOLKIT_WAN22_5B_BACKEND_ID),
        ai_toolkit_wan22_5b_backend(
            paths,
            backend_registry::AI_TOOLKIT_WAN22_5B_IMAGE_BACKEND_ID,
        ),
    ];
    backends.extend(
        backend_registry::BACKENDS
            .iter()
            .filter(|definition| {
                !is_musubi_wan_backend(definition.id)
                    && !is_ai_toolkit_wan22_5b_backend(definition.id)
            })
            .map(|definition| build_summary(paths, definition)),
    );
    backends
}

pub fn recommended_backend_id(backends: &[TrainingBackendSummary]) -> Option<String> {
    backends
        .iter()
        .find(|backend| backend.ready)
        .map(|backend| backend.id.clone())
        .or_else(|| backends.first().map(|backend| backend.id.clone()))
}

pub fn is_known_backend(id: &str) -> bool {
    backend_registry::backend_definition(id).is_some()
}

pub fn is_musubi_wan_backend(id: &str) -> bool {
    id == backend_registry::MUSUBI_WAN_BACKEND_ID
        || id == backend_registry::MUSUBI_WAN_IMAGE_BACKEND_ID
        || id == backend_registry::MUSUBI_WAN_14B_BACKEND_ID
}

pub fn is_ai_toolkit_wan22_5b_backend(id: &str) -> bool {
    id == backend_registry::AI_TOOLKIT_WAN22_5B_BACKEND_ID
        || id == backend_registry::AI_TOOLKIT_WAN22_5B_IMAGE_BACKEND_ID
}

#[derive(Clone)]
struct WanLaneBundleStatus {
    model_bundle_ready: bool,
    selected_dit_relative_path: Option<String>,
    recommended_defaults: WanTrainingDefaults,
    notes: Vec<String>,
}

pub fn backend_label(id: &str) -> String {
    backend_registry::BACKENDS
        .iter()
        .find(|backend| backend.id == id)
        .map(|backend| backend.name.to_string())
        .unwrap_or_else(|| id.to_string())
}

pub fn scan_wan_training(paths: &ProjectPaths) -> WanTrainingStatus {
    let files = WAN_FILES
        .iter()
        .map(|definition| wan_file_status(paths, *definition))
        .collect::<Vec<_>>();

    let selected_dit_relative_path =
        select_dit_path_for_task(&files, lane_registry::TRAINING_LANES[0].task);
    let t5_ready = files
        .iter()
        .any(|file| file.relative_path.contains("/t5/") || file.relative_path.contains("\\t5\\"));
    let t5_ready = t5_ready
        && files
            .iter()
            .find(|file| file.role == "Text encoder")
            .map(|file| file.present)
            .unwrap_or(false);
    let vae_ready = files
        .iter()
        .find(|file| file.role == "VAE")
        .map(|file| file.present)
        .unwrap_or(false);
    let model_bundle_ready = selected_dit_relative_path.is_some() && t5_ready && vae_ready;

    let wsl_ready = run_wsl_check("echo ok");
    let trainer_ready = wsl_ready
        && run_wsl_check(&format!(
            "cd {root} && test -x .venv/bin/python && test -f src/musubi_tuner/wan_train_network.py && test -f src/musubi_tuner/wan_cache_latents.py && test -f src/musubi_tuner/wan_cache_text_encoder_outputs.py",
            root = WSL_MUSUBI_ROOT,
        ));

    let mut notes = Vec::new();
    if model_bundle_ready {
        notes.push(
            "At least one Wan model bundle is present enough for the current Wan/Musubi lanes."
                .to_string(),
        );
    } else {
        notes.push(format!(
            "Wan model bundle is incomplete. The Wan/Musubi lanes need a DiT, T5 encoder, and VAE under {}/ (legacy {}/ is still accepted during transition).",
            model_registry::family_dependency_relative_root(model_registry::WAN_FAMILY_ID)
                .unwrap_or("models/wan/dependencies"),
            model_registry::family_legacy_relative_roots(model_registry::WAN_FAMILY_ID)
                .first()
                .copied()
                .unwrap_or("models/wan21_t2v_1_3b")
        ));
    }

    notes.push(format!(
        "Place Wan GGUF inference experiments under {}/ and Musubi training dependency files under {}/.",
        model_registry::family_gguf_relative_root(model_registry::WAN_FAMILY_ID)
            .unwrap_or("models/wan/gguf"),
        model_registry::family_dependency_relative_root(model_registry::WAN_FAMILY_ID)
            .unwrap_or("models/wan/dependencies")
    ));
    notes.push(
        "For the current Wan 1.3B and 14B training lanes, GGUF files do not satisfy the model-bundle requirement. Training looks for a task-matched DiT plus the shared T5 encoder and Wan VAE in models/wan/dependencies/."
            .to_string(),
    );

    if let Some(selected_dit) = &selected_dit_relative_path {
        notes.push(format!(
            "Selected default Wan DiT for generated plans: {}.",
            selected_dit
        ));
    }

    if trainer_ready {
        notes.push(
            "WSL Ubuntu, Musubi Tuner, and the Wan cache/train scripts were detected.".to_string(),
        );
    } else if wsl_ready {
        notes.push("WSL Ubuntu responded, but Musubi Tuner was not detected at ~/train_runtime/musubi-tuner.".to_string());
    } else {
        notes.push("WSL Ubuntu-24.04 did not respond from the Windows app yet.".to_string());
    }

    notes.push("Generated scripts will set the ROCm/ROCDXG environment variables explicitly so they do not depend on an interactive terminal session.".to_string());

    let default_lane = lane_registry::lane_definition(backend_registry::MUSUBI_WAN_BACKEND_ID)
        .expect("default wan lane definition should exist");

    WanTrainingStatus {
        id: "wan21_t2v_1_3b".to_string(),
        label: "Wan 2.1 T2V 1.3B training lane".to_string(),
        ready: model_bundle_ready && trainer_ready,
        model_bundle_ready,
        trainer_ready,
        wsl_ready,
        wsl_distro: WSL_DISTRO.to_string(),
        wsl_musubi_root: WSL_MUSUBI_ROOT.to_string(),
        selected_dit_relative_path,
        recommended_defaults: WanTrainingDefaults {
            resolution: default_lane.defaults.resolution,
            target_frames: default_lane.defaults.target_frames.unwrap_or(17),
            source_fps: default_lane.defaults.source_fps.unwrap_or(16.0),
            batch_size: default_lane.defaults.batch_size,
            rank: default_lane.defaults.rank,
            epochs: default_lane.defaults.epochs,
            learning_rate: default_lane.defaults.learning_rate,
            blocks_to_swap: default_lane.defaults.blocks_to_swap,
            route_label: default_lane.defaults.route_label.to_string(),
            route_summary: default_lane.defaults.route_summary.to_string(),
            hardware_note: default_lane.defaults.hardware_note.to_string(),
            exploratory: default_lane.defaults.exploratory,
        },
        files,
        notes,
    }
}

pub fn selected_wan_paths(paths: &ProjectPaths, backend_id: &str) -> Option<WanPathSet> {
    let lane_status = resolve_wan_lane_bundle(paths, backend_id)?;
    let t5 = resolved_wan_file_path(paths, "Text encoder")?;
    let vae = resolved_wan_file_path(paths, "VAE")?;
    Some(WanPathSet {
        dit: paths.root.join(lane_status.selected_dit_relative_path?),
        t5,
        vae,
    })
}

#[derive(Debug, Clone)]
pub struct WanPathSet {
    pub dit: PathBuf,
    pub t5: PathBuf,
    pub vae: PathBuf,
}

fn musubi_wan_video_backend(status: WanTrainingStatus) -> TrainingBackendSummary {
    let definition = backend_registry::backend_definition(backend_registry::MUSUBI_WAN_BACKEND_ID)
        .expect("musubi wan backend definition should exist");
    let lane = lane_registry::lane_definition(definition.id)
        .expect("musubi wan lane definition should exist");
    let lane_bundle = resolve_wan_lane_bundle_from_files(&status.files, definition.id);
    let mut notes = lane_bundle
        .as_ref()
        .map(|bundle| bundle.notes.clone())
        .unwrap_or_default();
    notes.extend(status.notes.clone());
    TrainingBackendSummary {
        id: definition.id.to_string(),
        name: definition.name.to_string(),
        description: definition.description.to_string(),
        best_for: definition.best_for.to_string(),
        lane_label: Some(lane.label.to_string()),
        lane_dataset_kind: Some(lane.dataset_kind.media_label().to_string()),
        lane_task: Some(lane.task.to_string()),
        compatible_family_ids: vec![model_registry::WAN_FAMILY_ID.to_string()],
        compatible_family_labels: backend_registry::compatible_family_labels(definition),
        model_bundle_ready: lane_bundle.as_ref().map(|bundle| bundle.model_bundle_ready),
        selected_dit_relative_path: lane_bundle
            .as_ref()
            .and_then(|bundle| bundle.selected_dit_relative_path.clone()),
        recommended_defaults: lane_bundle
            .as_ref()
            .map(|bundle| bundle.recommended_defaults.clone()),
        ready: lane_bundle
            .as_ref()
            .map(|bundle| bundle.model_bundle_ready)
            .unwrap_or(false)
            && status.trainer_ready,
        relative_path: Some(format!("{}: {}", status.wsl_distro, status.wsl_musubi_root)),
        notes,
    }
}

fn musubi_wan_image_backend(status: WanTrainingStatus) -> TrainingBackendSummary {
    let definition =
        backend_registry::backend_definition(backend_registry::MUSUBI_WAN_IMAGE_BACKEND_ID)
            .expect("musubi wan image backend definition should exist");
    let lane = lane_registry::lane_definition(definition.id)
        .expect("musubi wan image lane definition should exist");
    let lane_bundle = resolve_wan_lane_bundle_from_files(&status.files, definition.id);
    let mut notes = lane_bundle
        .as_ref()
        .map(|bundle| bundle.notes.clone())
        .unwrap_or_default();
    notes.extend(status.notes.clone());
    TrainingBackendSummary {
        id: definition.id.to_string(),
        name: definition.name.to_string(),
        description: definition.description.to_string(),
        best_for: definition.best_for.to_string(),
        lane_label: Some(lane.label.to_string()),
        lane_dataset_kind: Some(lane.dataset_kind.media_label().to_string()),
        lane_task: Some(lane.task.to_string()),
        compatible_family_ids: vec![model_registry::WAN_FAMILY_ID.to_string()],
        compatible_family_labels: backend_registry::compatible_family_labels(definition),
        model_bundle_ready: lane_bundle.as_ref().map(|bundle| bundle.model_bundle_ready),
        selected_dit_relative_path: lane_bundle
            .as_ref()
            .and_then(|bundle| bundle.selected_dit_relative_path.clone()),
        recommended_defaults: lane_bundle
            .as_ref()
            .map(|bundle| bundle.recommended_defaults.clone()),
        ready: lane_bundle
            .as_ref()
            .map(|bundle| bundle.model_bundle_ready)
            .unwrap_or(false)
            && status.trainer_ready,
        relative_path: Some(format!("{}: {}", status.wsl_distro, status.wsl_musubi_root)),
        notes,
    }
}

fn musubi_wan_14b_backend(status: WanTrainingStatus) -> TrainingBackendSummary {
    let definition =
        backend_registry::backend_definition(backend_registry::MUSUBI_WAN_14B_BACKEND_ID)
            .expect("musubi wan 14b backend definition should exist");
    let lane =
        lane_registry::lane_definition(definition.id).expect("musubi wan 14b lane should exist");
    let lane_bundle = resolve_wan_lane_bundle_from_files(&status.files, definition.id);
    let mut notes = lane_bundle
        .as_ref()
        .map(|bundle| bundle.notes.clone())
        .unwrap_or_default();
    notes.push(
        "Wan 14B is bolted in as an exploratory lane. Expect materially higher VRAM and system-memory pressure than the proven 1.3B route."
            .to_string(),
    );
    notes.push(
        "On the current WSL + ROCm route, Wan 14B can be Linux-OOM-killed on CPU-side RAM/swap pressure before Windows Task Manager shows obvious GPU load. Treat WSL memory and swap as first-class requirements, not just VRAM."
            .to_string(),
    );
    notes.push(
        "The current experimental 14B handoff now avoids the FP8 weight-cast path on this chassis because the BF16-loaded route was the only local variant that reached live training before system memory became the next limit."
            .to_string(),
    );
    notes.extend(status.notes.clone());
    TrainingBackendSummary {
        id: definition.id.to_string(),
        name: definition.name.to_string(),
        description: definition.description.to_string(),
        best_for: definition.best_for.to_string(),
        lane_label: Some(lane.label.to_string()),
        lane_dataset_kind: Some(lane.dataset_kind.media_label().to_string()),
        lane_task: Some(lane.task.to_string()),
        compatible_family_ids: vec![model_registry::WAN_FAMILY_ID.to_string()],
        compatible_family_labels: backend_registry::compatible_family_labels(definition),
        model_bundle_ready: lane_bundle.as_ref().map(|bundle| bundle.model_bundle_ready),
        selected_dit_relative_path: lane_bundle
            .as_ref()
            .and_then(|bundle| bundle.selected_dit_relative_path.clone()),
        recommended_defaults: lane_bundle
            .as_ref()
            .map(|bundle| bundle.recommended_defaults.clone()),
        ready: lane_bundle
            .as_ref()
            .map(|bundle| bundle.model_bundle_ready)
            .unwrap_or(false)
            && status.trainer_ready,
        relative_path: Some(format!("{}: {}", status.wsl_distro, status.wsl_musubi_root)),
        notes,
    }
}

fn ai_toolkit_wan22_5b_backend(
    paths: &ProjectPaths,
    backend_id: &'static str,
) -> TrainingBackendSummary {
    let definition = backend_registry::backend_definition(backend_id)
        .expect("ai toolkit wan22 5b backend definition should exist");
    let lane = lane_registry::lane_definition(definition.id)
        .expect("ai toolkit wan22 5b lane definition should exist");
    let runtime_root = &paths.runtime;
    let detected_folder = detect_trainer_root(runtime_root, definition);
    let runtime_ready = detected_folder.is_some();
    let bundle = resolve_wan22_5b_bundle(paths);
    let mut notes = Vec::new();
    if let Some(folder) = detected_folder.as_ref() {
        notes.push(format!(
            "Detected AI Toolkit folder at {}.",
            folder.strip_prefix(&paths.root).unwrap_or(folder).display()
        ));
        if runtime_ready {
            notes.push(
                "AI Toolkit marker files were found, including the usual repo-root launcher shape."
                    .to_string(),
            );
        } else {
            notes.push(
                "The AI Toolkit folder exists, but the usual repo-root entry files were not all detected yet."
                    .to_string(),
            );
        }
    } else {
        notes.push(
            "No local AI Toolkit folder detected yet under runtime/ai_toolkit or runtime/ai-toolkit. Chatty-lora expects the repo root here, with files like run.py, requirements.txt, toolkit/, config/examples/, and ui/."
                .to_string(),
        );
    }
    if bundle.model_bundle_ready {
        if lane.dataset_kind == lane_registry::TrainingDatasetKind::Image {
            notes.push(
                "Wan 2.2 TI2V 5B local model bundle looks complete enough for the image-first scaffolded lane."
                    .to_string(),
            );
        } else {
            notes.push(
                "Wan 2.2 TI2V 5B local model bundle looks complete enough for the scaffolded lane."
                    .to_string(),
            );
        }
    } else {
        notes.push(format!(
            "Wan 2.2 TI2V 5B bundle is incomplete. Chatty-lora currently expects a Diffusers bundle under {} plus the shared UMT5 encoder and Wan 2.2 VAE.",
            model_registry::family_diffusers_relative_root(model_registry::WAN_FAMILY_ID)
                .unwrap_or("models/wan/diffusers")
        ));
    }
    if let Some(relative_root) = &bundle.bundle_relative_root {
        notes.push(format!(
            "Selected Wan 2.2 TI2V 5B Diffusers bundle: {}.",
            relative_root
        ));
    }
    notes.push(
        "This lane is scaffolded for AI Toolkit groundwork in Chatty-lora. The builder can prepare the handoff folder, and the in-browser runner can now launch the generated PowerShell scaffold."
            .to_string(),
    );
    notes.push(if lane.dataset_kind == lane_registry::TrainingDatasetKind::Image {
        "Wan 2.2 5B image-first LoRAs are being treated as the more practical first verified target than the heavier video and 14B routes, but the exact AI Toolkit launch recipe is still expected to evolve."
            .to_string()
    } else {
        "Wan 2.2 5B is being treated as the more realistic next verified Wan 2.2 target than the heavier 14B routes, but the exact AI Toolkit launch recipe is still expected to evolve."
            .to_string()
    });
    notes.push(
        "A single local AI Toolkit checkout is intended to be reusable across future Diffusers-style lanes too, such as Flux, SDXL, SD 1.5, and other newer image or video workflows."
            .to_string(),
    );

    TrainingBackendSummary {
        id: definition.id.to_string(),
        name: definition.name.to_string(),
        description: definition.description.to_string(),
        best_for: definition.best_for.to_string(),
        lane_label: Some(lane.label.to_string()),
        lane_dataset_kind: Some(lane.dataset_kind.media_label().to_string()),
        lane_task: Some(lane.task.to_string()),
        compatible_family_ids: vec![model_registry::WAN_FAMILY_ID.to_string()],
        compatible_family_labels: backend_registry::compatible_family_labels(definition),
        model_bundle_ready: Some(bundle.model_bundle_ready),
        selected_dit_relative_path: bundle.bundle_relative_root.clone(),
        recommended_defaults: Some(WanTrainingDefaults {
            resolution: lane.defaults.resolution,
            target_frames: lane.defaults.target_frames.unwrap_or(17),
            source_fps: lane.defaults.source_fps.unwrap_or(16.0),
            batch_size: lane.defaults.batch_size,
            rank: lane.defaults.rank,
            epochs: lane.defaults.epochs,
            learning_rate: lane.defaults.learning_rate,
            blocks_to_swap: lane.defaults.blocks_to_swap,
            route_label: lane.defaults.route_label.to_string(),
            route_summary: lane.defaults.route_summary.to_string(),
            hardware_note: lane.defaults.hardware_note.to_string(),
            exploratory: lane.defaults.exploratory,
        }),
        ready: bundle.model_bundle_ready && runtime_ready,
        relative_path: detected_folder.as_ref().map(|folder| {
            folder
                .strip_prefix(&paths.root)
                .unwrap_or(folder)
                .display()
                .to_string()
        }),
        notes,
    }
}

fn wan_file_status(paths: &ProjectPaths, definition: WanFileDefinition) -> WanModelFileStatus {
    let resolved_relative_path = resolve_existing_wan_relative_path(paths, definition)
        .unwrap_or_else(|| definition.primary_relative_path.to_string());
    let absolute_path = paths.root.join(&resolved_relative_path);
    let bytes = std::fs::metadata(&absolute_path)
        .ok()
        .map(|metadata| metadata.len());
    WanModelFileStatus {
        label: definition.label.to_string(),
        role: definition.role.to_string(),
        relative_path: resolved_relative_path,
        present: absolute_path.exists(),
        required: definition.required,
        bytes,
    }
}

fn select_dit_path_for_task(files: &[WanModelFileStatus], task: &str) -> Option<String> {
    let target_label = match task {
        "t2v-14B" => "14B",
        _ => "1.3B",
    };
    files
        .iter()
        .filter(|file| file.label.contains(target_label))
        .find(|file| file.relative_path.contains("bf16") && file.present)
        .or_else(|| {
            files
                .iter()
                .filter(|file| file.label.contains(target_label))
                .find(|file| file.relative_path.contains("fp16") && file.present)
        })
        .map(|file| file.relative_path.clone())
}

pub fn wan_bundle_ready_for_backend(paths: &ProjectPaths, backend_id: &str) -> bool {
    if is_ai_toolkit_wan22_5b_backend(backend_id) {
        return resolve_wan22_5b_bundle(paths).model_bundle_ready;
    }
    resolve_wan_lane_bundle(paths, backend_id)
        .map(|bundle| bundle.model_bundle_ready)
        .unwrap_or(false)
}

#[derive(Debug, Clone)]
pub struct Wan22Ti2v5bBundleStatus {
    pub model_bundle_ready: bool,
    pub bundle_relative_root: Option<String>,
    pub bundle_path: Option<PathBuf>,
    pub t5_path: Option<PathBuf>,
    pub vae_path: Option<PathBuf>,
}

pub fn selected_wan22_ti2v_5b_bundle(paths: &ProjectPaths) -> Option<Wan22Ti2v5bBundleStatus> {
    let bundle = resolve_wan22_5b_bundle(paths);
    bundle.model_bundle_ready.then_some(bundle)
}

fn resolve_wan22_5b_bundle(paths: &ProjectPaths) -> Wan22Ti2v5bBundleStatus {
    let bundle_relative_root = WAN22_TI2V_5B_DIFFUSERS_CANDIDATES
        .iter()
        .find(|relative| paths.root.join(relative).join("model_index.json").exists())
        .map(|relative| (*relative).to_string());
    let bundle_path = bundle_relative_root
        .as_ref()
        .map(|relative| paths.root.join(relative));
    let t5_path = resolved_wan_file_path(paths, "Text encoder");
    let vae_path = [
        "models/wan/dependencies/vae/wan2.2_vae.safetensors",
        "models/wan/dependencies/vae/Wan2.2_VAE.pth",
    ]
    .iter()
    .map(|relative| paths.root.join(relative))
    .find(|path| path.exists());
    let model_bundle_ready = bundle_path.is_some() && t5_path.is_some() && vae_path.is_some();
    Wan22Ti2v5bBundleStatus {
        model_bundle_ready,
        bundle_relative_root,
        bundle_path,
        t5_path,
        vae_path,
    }
}

fn resolve_wan_lane_bundle(paths: &ProjectPaths, backend_id: &str) -> Option<WanLaneBundleStatus> {
    let files = WAN_FILES
        .iter()
        .map(|definition| wan_file_status(paths, *definition))
        .collect::<Vec<_>>();
    resolve_wan_lane_bundle_from_files(&files, backend_id)
}

fn resolve_wan_lane_bundle_from_files(
    files: &[WanModelFileStatus],
    backend_id: &str,
) -> Option<WanLaneBundleStatus> {
    let lane = lane_registry::lane_definition(backend_id)?;
    let selected_dit_relative_path = select_dit_path_for_task(files, lane.task);
    let t5_ready = files
        .iter()
        .find(|file| file.role == "Text encoder")
        .map(|file| file.present)
        .unwrap_or(false);
    let vae_ready = files
        .iter()
        .find(|file| file.role == "VAE")
        .map(|file| file.present)
        .unwrap_or(false);
    let model_bundle_ready = selected_dit_relative_path.is_some() && t5_ready && vae_ready;
    let variant_label = match lane.task {
        "t2v-14B" => "Wan 2.1 T2V 14B",
        _ => "Wan 2.1 T2V 1.3B",
    };
    let mut notes = Vec::new();
    if model_bundle_ready {
        notes.push(format!(
            "{variant_label} model bundle is present enough for this lane."
        ));
    } else {
        notes.push(format!(
            "{variant_label} bundle is incomplete. This lane needs a matching DiT plus the shared T5 encoder and Wan VAE."
        ));
    }
    if let Some(selected_dit) = &selected_dit_relative_path {
        notes.push(format!("Selected DiT for this lane: {selected_dit}."));
    }
    Some(WanLaneBundleStatus {
        model_bundle_ready,
        selected_dit_relative_path,
        recommended_defaults: WanTrainingDefaults {
            resolution: lane.defaults.resolution,
            target_frames: lane.defaults.target_frames.unwrap_or(17),
            source_fps: lane.defaults.source_fps.unwrap_or(16.0),
            batch_size: lane.defaults.batch_size,
            rank: lane.defaults.rank,
            epochs: lane.defaults.epochs,
            learning_rate: lane.defaults.learning_rate,
            blocks_to_swap: lane.defaults.blocks_to_swap,
            route_label: lane.defaults.route_label.to_string(),
            route_summary: lane.defaults.route_summary.to_string(),
            hardware_note: lane.defaults.hardware_note.to_string(),
            exploratory: lane.defaults.exploratory,
        },
        notes,
    })
}

fn resolve_existing_wan_relative_path(
    paths: &ProjectPaths,
    definition: WanFileDefinition,
) -> Option<String> {
    std::iter::once(definition.primary_relative_path)
        .chain(definition.legacy_relative_paths.iter().copied())
        .find(|relative_path| paths.root.join(relative_path).exists())
        .map(str::to_string)
}

fn resolved_wan_file_path(paths: &ProjectPaths, role: &str) -> Option<PathBuf> {
    WAN_FILES
        .iter()
        .find(|definition| definition.role == role)
        .and_then(|definition| resolve_existing_wan_relative_path(paths, *definition))
        .map(|relative_path| paths.root.join(relative_path))
}

fn run_wsl_check(command: &str) -> bool {
    Command::new("wsl")
        .args(["-d", WSL_DISTRO, "--", "bash", "-lc", command])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn build_summary(
    paths: &ProjectPaths,
    definition: &backend_registry::BackendDefinition,
) -> TrainingBackendSummary {
    let runtime_root = &paths.runtime;
    let detected_folder = detect_trainer_root(runtime_root, definition);
    let ready = detected_folder.is_some();

    let relative_path = detected_folder.as_ref().map(|folder| {
        folder
            .strip_prefix(&paths.root)
            .unwrap_or(folder)
            .display()
            .to_string()
    });

    let mut notes = Vec::new();
    if let Some(folder) = detected_folder.as_ref() {
        notes.push(format!(
            "Detected trainer folder at {}.",
            folder.strip_prefix(&paths.root).unwrap_or(folder).display()
        ));
        if ready {
            notes.push(
                "Marker files were found, so this backend looks ready to wire later.".to_string(),
            );
        } else {
            notes.push(
                "The folder exists, but Chatty-lora could not find the usual entry files yet."
                    .to_string(),
            );
        }
    } else {
        notes.push(format!(
            "No local trainer folder detected yet. If you want this lane later, drop it under runtime/ as one of: {}.",
            definition.folder_candidates.join(", ")
        ));
    }

    let compatible_family_labels = backend_registry::compatible_family_labels(definition);

    TrainingBackendSummary {
        id: definition.id.to_string(),
        name: definition.name.to_string(),
        description: definition.description.to_string(),
        best_for: definition.best_for.to_string(),
        lane_label: lane_registry::lane_definition(definition.id)
            .map(|lane| lane.label.to_string()),
        lane_dataset_kind: lane_registry::lane_definition(definition.id)
            .map(|lane| lane.dataset_kind.media_label().to_string()),
        lane_task: lane_registry::lane_definition(definition.id).map(|lane| lane.task.to_string()),
        compatible_family_ids: definition
            .compatible_family_ids
            .iter()
            .map(|family_id| family_id.to_string())
            .collect(),
        compatible_family_labels,
        model_bundle_ready: None,
        selected_dit_relative_path: None,
        recommended_defaults: None,
        ready,
        relative_path,
        notes,
    }
}

fn has_marker(folder: &Path, markers: &[&str]) -> bool {
    markers.iter().any(|marker| folder.join(marker).exists())
}

fn detect_trainer_root(
    runtime_root: &Path,
    definition: &backend_registry::BackendDefinition,
) -> Option<PathBuf> {
    definition.folder_candidates.iter().find_map(|candidate| {
        let candidate_root = runtime_root.join(candidate);
        find_trainer_root_from_candidate(&candidate_root, definition.marker_files, 0)
    })
}

fn find_trainer_root_from_candidate(
    folder: &Path,
    markers: &[&str],
    depth: usize,
) -> Option<PathBuf> {
    if !folder.exists() {
        return None;
    }
    if has_marker(folder, markers) {
        return Some(folder.to_path_buf());
    }
    if depth >= 2 {
        return None;
    }
    let entries = std::fs::read_dir(folder).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_trainer_root_from_candidate(&path, markers, depth + 1) {
                return Some(found);
            }
        }
    }
    None
}
