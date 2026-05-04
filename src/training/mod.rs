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
    ];
    backends.extend(
        backend_registry::BACKENDS
            .iter()
            .filter(|definition| !is_musubi_wan_backend(definition.id))
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
    resolve_wan_lane_bundle(paths, backend_id)
        .map(|bundle| bundle.model_bundle_ready)
        .unwrap_or(false)
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
    let detected_folder = definition
        .folder_candidates
        .iter()
        .map(|candidate| runtime_root.join(candidate))
        .find(|folder| folder.exists());

    let ready = detected_folder
        .as_ref()
        .map(|folder| has_marker(folder, definition.marker_files))
        .unwrap_or(false);

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
