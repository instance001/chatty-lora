use crate::model_registry;

pub const MUSUBI_WAN_BACKEND_ID: &str = "musubi_wan21_t2v_1_3b";
pub const MUSUBI_WAN_IMAGE_BACKEND_ID: &str = "musubi_wan21_t2v_1_3b_image";
pub const MUSUBI_WAN_LABEL: &str = "Musubi Tuner / Wan 2.1 T2V 1.3B";
pub const MUSUBI_WAN_IMAGE_LABEL: &str = "Musubi Tuner / Wan 2.1 T2V 1.3B / Image visual LoRA";

#[derive(Clone, Copy)]
pub struct BackendDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub best_for: &'static str,
    pub compatible_family_ids: &'static [&'static str],
    pub folder_candidates: &'static [&'static str],
    pub marker_files: &'static [&'static str],
}

pub const BACKENDS: &[BackendDefinition] = &[
    BackendDefinition {
        id: MUSUBI_WAN_BACKEND_ID,
        name: MUSUBI_WAN_LABEL,
        description: "The proven Wan video training lane: Windows app, WSL Ubuntu trainer, ROCm PyTorch, Musubi Tuner, and Wan 2.1 T2V 1.3B.",
        best_for: "Wan 2.1 video LoRAs, starting with short T2V motion/style experiments.",
        compatible_family_ids: &[model_registry::WAN_FAMILY_ID],
        folder_candidates: &["musubi_tuner", "musubi-tuner"],
        marker_files: &[
            "src/musubi_tuner/wan_train_network.py",
            "src/musubi_tuner/wan_cache_latents.py",
            "src/musubi_tuner/wan_cache_text_encoder_outputs.py",
        ],
    },
    BackendDefinition {
        id: MUSUBI_WAN_IMAGE_BACKEND_ID,
        name: MUSUBI_WAN_IMAGE_LABEL,
        description: "A sibling Wan/Musubi lane that trains still-image visual concepts into the same Wan 2.1 T2V 1.3B model family.",
        best_for: "Wan visual identity, character, object, and style LoRAs from still images before moving into video refinement.",
        compatible_family_ids: &[model_registry::WAN_FAMILY_ID],
        folder_candidates: &["musubi_tuner", "musubi-tuner"],
        marker_files: &[
            "src/musubi_tuner/wan_train_network.py",
            "src/musubi_tuner/wan_cache_latents.py",
            "src/musubi_tuner/wan_cache_text_encoder_outputs.py",
        ],
    },
    BackendDefinition {
        id: "kohya_ss",
        name: "kohya_ss / sd-scripts",
        description: "A common image LoRA training lane for Stable Diffusion style workflows.",
        best_for: "General SD and SDXL image LoRAs.",
        compatible_family_ids: &[model_registry::FLUX_FAMILY_ID],
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
        compatible_family_ids: &[model_registry::FLUX_FAMILY_ID],
        folder_candidates: &["ai_toolkit", "ai-toolkit"],
        marker_files: &["run.py", "requirements.txt", "toolkit"],
    },
    BackendDefinition {
        id: "onetrainer",
        name: "OneTrainer",
        description: "A guided trainer lane with a friendlier setup model than raw script collections.",
        best_for: "Users who want a packaged trainer experience.",
        compatible_family_ids: &[
            model_registry::WAN_FAMILY_ID,
            model_registry::FLUX_FAMILY_ID,
            model_registry::AUDIO_FAMILY_ID,
        ],
        folder_candidates: &["onetrainer", "OneTrainer"],
        marker_files: &["OneTrainer.exe", "main.py", "requirements.txt"],
    },
];

pub fn backend_definition(id: &str) -> Option<&'static BackendDefinition> {
    BACKENDS.iter().find(|backend| backend.id == id)
}

pub fn compatible_family_labels(definition: &BackendDefinition) -> Vec<String> {
    definition
        .compatible_family_ids
        .iter()
        .filter_map(|family_id| {
            model_registry::family_definition(family_id).map(|family| family.label.to_string())
        })
        .collect()
}
