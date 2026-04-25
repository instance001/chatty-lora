pub const WAN_FAMILY_ID: &str = "wan";
pub const FLUX_FAMILY_ID: &str = "flux";
pub const AI_ASSISTANT_FAMILY_ID: &str = "ai_assistant";
pub const AUDIO_FAMILY_ID: &str = "audio";

#[derive(Debug, Clone, Copy)]
pub struct ModelFamilyDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub purpose: &'static str,
    pub include_in_training_base_model_picker: bool,
    pub relative_root: &'static str,
    pub gguf_relative_root: &'static str,
    pub dependency_relative_root: &'static str,
    pub legacy_relative_roots: &'static [&'static str],
}

pub const MODEL_FAMILIES: &[ModelFamilyDefinition] = &[
    ModelFamilyDefinition {
        id: WAN_FAMILY_ID,
        label: "Wan",
        purpose: "Wan model family for training lanes, GGUF inference experiments, and future sibling variants.",
        include_in_training_base_model_picker: true,
        relative_root: "models/wan",
        gguf_relative_root: "models/wan/gguf",
        dependency_relative_root: "models/wan/dependencies",
        legacy_relative_roots: &["models/wan21_t2v_1_3b"],
    },
    ModelFamilyDefinition {
        id: FLUX_FAMILY_ID,
        label: "Flux",
        purpose: "Flux model family for future image-generation and LoRA workflows.",
        include_in_training_base_model_picker: true,
        relative_root: "models/flux",
        gguf_relative_root: "models/flux/gguf",
        dependency_relative_root: "models/flux/dependencies",
        legacy_relative_roots: &[],
    },
    ModelFamilyDefinition {
        id: AI_ASSISTANT_FAMILY_ID,
        label: "AI Assistant",
        purpose: "Local helper models that support app features like assistant chat, source triage, and bug-testing workflows.",
        include_in_training_base_model_picker: false,
        relative_root: "models/ai_assistant",
        gguf_relative_root: "models/ai_assistant/gguf",
        dependency_relative_root: "models/ai_assistant/dependencies",
        legacy_relative_roots: &[],
    },
    ModelFamilyDefinition {
        id: AUDIO_FAMILY_ID,
        label: "Audio",
        purpose: "Future audio-model family bucket for transcription, tagging, and audio training support.",
        include_in_training_base_model_picker: true,
        relative_root: "models/audio",
        gguf_relative_root: "models/audio/gguf",
        dependency_relative_root: "models/audio/dependencies",
        legacy_relative_roots: &[],
    },
];

pub fn family_definition(family_id: &str) -> Option<&'static ModelFamilyDefinition> {
    MODEL_FAMILIES.iter().find(|family| family.id == family_id)
}

pub fn family_gguf_relative_root(family_id: &str) -> Option<&'static str> {
    family_definition(family_id).map(|family| family.gguf_relative_root)
}

pub fn family_dependency_relative_root(family_id: &str) -> Option<&'static str> {
    family_definition(family_id).map(|family| family.dependency_relative_root)
}

pub fn family_legacy_relative_roots(family_id: &str) -> &'static [&'static str] {
    family_definition(family_id)
        .map(|family| family.legacy_relative_roots)
        .unwrap_or(&[])
}

pub fn family_layout_note() -> String {
    let families = MODEL_FAMILIES
        .iter()
        .map(|family| {
            format!(
                "{} -> {} ({})",
                family.label, family.relative_root, family.purpose
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Model families now live in dedicated buckets: {}.",
        families
    )
}

pub fn include_family_in_training_base_model_picker(family_id: &str) -> bool {
    family_definition(family_id)
        .map(|family| family.include_in_training_base_model_picker)
        .unwrap_or(false)
}
