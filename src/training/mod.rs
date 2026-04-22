use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    state::ProjectPaths,
    types::{TrainingBackendSummary, WanModelFileStatus, WanTrainingDefaults, WanTrainingStatus},
};

pub const MUSUBI_WAN_BACKEND_ID: &str = "musubi_wan21_t2v_1_3b";
pub const MUSUBI_WAN_IMAGE_BACKEND_ID: &str = "musubi_wan21_t2v_1_3b_image";
pub const MUSUBI_WAN_LABEL: &str = "Musubi Tuner / Wan 2.1 T2V 1.3B";
pub const MUSUBI_WAN_IMAGE_LABEL: &str = "Musubi Tuner / Wan 2.1 T2V 1.3B / Image visual LoRA";
pub const WSL_DISTRO: &str = "Ubuntu-24.04";
pub const WSL_MUSUBI_ROOT: &str = "~/train_runtime/musubi-tuner";
pub const WSL_ENV_PREFIX: &str = "export LD_LIBRARY_PATH=/opt/rocm/lib:/opt/rocm-7.2.2/lib:/usr/local/lib:${LD_LIBRARY_PATH:-}; export HSA_ENABLE_DXG_DETECTION=1; export TORCH_ROCM_AOTRITON_ENABLE_EXPERIMENTAL=1";

#[derive(Clone, Copy)]
struct BackendDefinition {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    best_for: &'static str,
    folder_candidates: &'static [&'static str],
    marker_files: &'static [&'static str],
}

const BACKENDS: &[BackendDefinition] = &[
    BackendDefinition {
        id: "kohya_ss",
        name: "kohya_ss / sd-scripts",
        description: "A common image LoRA training lane for Stable Diffusion style workflows.",
        best_for: "General SD and SDXL image LoRAs.",
        folder_candidates: &["kohya_ss", "kohya-ss", "sd-scripts"],
        marker_files: &[
            "kohya_gui.py",
            "gui.bat",
            "train_network.py",
            "sdxl_train.py",
        ],
    },
    BackendDefinition {
        id: "ai_toolkit",
        name: "AI Toolkit",
        description: "A more modern trainer lane often used for Flux-style and newer LoRA workflows.",
        best_for: "Flux and newer image LoRA pipelines.",
        folder_candidates: &["ai_toolkit", "ai-toolkit"],
        marker_files: &["run.py", "requirements.txt", "toolkit"],
    },
    BackendDefinition {
        id: "onetrainer",
        name: "OneTrainer",
        description: "A guided trainer lane with a friendlier setup model than raw script collections.",
        best_for: "Users who want a packaged trainer experience.",
        folder_candidates: &["onetrainer", "OneTrainer"],
        marker_files: &["OneTrainer.exe", "main.py", "requirements.txt"],
    },
];

#[derive(Clone, Copy)]
struct WanFileDefinition {
    relative_path: &'static str,
    label: &'static str,
    role: &'static str,
    required: bool,
}

const WAN_FILES: &[WanFileDefinition] = &[
    WanFileDefinition {
        relative_path: "models/wan21_t2v_1_3b/dit/Wan2_1-T2V-1_3B_bf16.safetensors",
        label: "Wan 2.1 T2V 1.3B DiT BF16",
        role: "Diffusion model",
        required: true,
    },
    WanFileDefinition {
        relative_path: "models/wan21_t2v_1_3b/dit/wan2.1_t2v_1.3B_fp16.safetensors",
        label: "Wan 2.1 T2V 1.3B DiT FP16 fallback",
        role: "Diffusion model fallback",
        required: false,
    },
    WanFileDefinition {
        relative_path: "models/wan21_t2v_1_3b/t5/models_t5_umt5-xxl-enc-bf16.pth",
        label: "UMT5 XXL text encoder BF16",
        role: "Text encoder",
        required: true,
    },
    WanFileDefinition {
        relative_path: "models/wan21_t2v_1_3b/vae/wan_2.1_vae.safetensors",
        label: "Wan 2.1 VAE",
        role: "VAE",
        required: true,
    },
    WanFileDefinition {
        relative_path: "models/wan21_t2v_1_3b/clip/models_clip_open-clip-xlm-roberta-large-vit-huge-14.pth",
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
    ];
    backends.extend(
        BACKENDS
            .iter()
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
    is_musubi_wan_backend(id) || BACKENDS.iter().any(|backend| backend.id == id)
}

pub fn is_musubi_wan_backend(id: &str) -> bool {
    id == MUSUBI_WAN_BACKEND_ID || id == MUSUBI_WAN_IMAGE_BACKEND_ID
}

pub fn is_musubi_wan_image_backend(id: &str) -> bool {
    id == MUSUBI_WAN_IMAGE_BACKEND_ID
}

pub fn backend_label(id: &str) -> String {
    if id == MUSUBI_WAN_BACKEND_ID {
        return MUSUBI_WAN_LABEL.to_string();
    }
    if id == MUSUBI_WAN_IMAGE_BACKEND_ID {
        return MUSUBI_WAN_IMAGE_LABEL.to_string();
    }

    BACKENDS
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

    let selected_dit_relative_path = select_dit_path(&files);
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
            "Wan 2.1 T2V 1.3B model bundle is present enough for the current Wan/Musubi lanes."
                .to_string(),
        );
    } else {
        notes.push("Wan model bundle is incomplete. The Wan/Musubi lanes need a DiT, T5 encoder, and VAE under models/wan21_t2v_1_3b/.".to_string());
    }

    if let Some(selected_dit) = &selected_dit_relative_path {
        notes.push(format!(
            "Selected DiT for generated plans: {}.",
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
            resolution: 512,
            target_frames: 17,
            source_fps: 16.0,
            batch_size: 1,
            rank: 8,
            epochs: 1,
            learning_rate: 0.0001,
        },
        files,
        notes,
    }
}

pub fn selected_wan_paths(paths: &ProjectPaths) -> Option<WanPathSet> {
    let status = scan_wan_training(paths);
    Some(WanPathSet {
        dit: paths.root.join(status.selected_dit_relative_path?),
        t5: paths
            .root
            .join("models/wan21_t2v_1_3b/t5/models_t5_umt5-xxl-enc-bf16.pth"),
        vae: paths
            .root
            .join("models/wan21_t2v_1_3b/vae/wan_2.1_vae.safetensors"),
    })
}

#[derive(Debug, Clone)]
pub struct WanPathSet {
    pub dit: PathBuf,
    pub t5: PathBuf,
    pub vae: PathBuf,
}

fn musubi_wan_video_backend(status: WanTrainingStatus) -> TrainingBackendSummary {
    TrainingBackendSummary {
        id: MUSUBI_WAN_BACKEND_ID.to_string(),
        name: MUSUBI_WAN_LABEL.to_string(),
        description: "The proven Wan video training lane: Windows app, WSL Ubuntu trainer, ROCm PyTorch, Musubi Tuner, and Wan 2.1 T2V 1.3B.".to_string(),
        best_for: "Wan 2.1 video LoRAs, starting with short T2V motion/style experiments.".to_string(),
        ready: status.ready,
        relative_path: Some(format!("{}: {}", status.wsl_distro, status.wsl_musubi_root)),
        notes: status.notes,
    }
}

fn musubi_wan_image_backend(status: WanTrainingStatus) -> TrainingBackendSummary {
    TrainingBackendSummary {
        id: MUSUBI_WAN_IMAGE_BACKEND_ID.to_string(),
        name: MUSUBI_WAN_IMAGE_LABEL.to_string(),
        description: "A sibling Wan/Musubi lane that trains still-image visual concepts into the same Wan 2.1 T2V 1.3B model family.".to_string(),
        best_for: "Wan visual identity, character, object, and style LoRAs from still images before moving into video refinement.".to_string(),
        ready: status.ready,
        relative_path: Some(format!("{}: {}", status.wsl_distro, status.wsl_musubi_root)),
        notes: status.notes,
    }
}

fn wan_file_status(paths: &ProjectPaths, definition: WanFileDefinition) -> WanModelFileStatus {
    let absolute_path = paths.root.join(definition.relative_path);
    let bytes = std::fs::metadata(&absolute_path)
        .ok()
        .map(|metadata| metadata.len());
    WanModelFileStatus {
        label: definition.label.to_string(),
        role: definition.role.to_string(),
        relative_path: definition.relative_path.to_string(),
        present: absolute_path.exists(),
        required: definition.required,
        bytes,
    }
}

fn select_dit_path(files: &[WanModelFileStatus]) -> Option<String> {
    files
        .iter()
        .find(|file| file.relative_path.contains("bf16") && file.present)
        .or_else(|| {
            files
                .iter()
                .find(|file| file.relative_path.contains("fp16") && file.present)
        })
        .map(|file| file.relative_path.clone())
}

fn run_wsl_check(command: &str) -> bool {
    Command::new("wsl")
        .args(["-d", WSL_DISTRO, "--", "bash", "-lc", command])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn build_summary(paths: &ProjectPaths, definition: &BackendDefinition) -> TrainingBackendSummary {
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

    TrainingBackendSummary {
        id: definition.id.to_string(),
        name: definition.name.to_string(),
        description: definition.description.to_string(),
        best_for: definition.best_for.to_string(),
        ready,
        relative_path,
        notes,
    }
}

fn has_marker(folder: &Path, markers: &[&str]) -> bool {
    markers.iter().any(|marker| folder.join(marker).exists())
}
