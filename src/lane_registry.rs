use crate::backend_registry;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrainingDatasetKind {
    Video,
    Image,
}

impl TrainingDatasetKind {
    pub fn media_label(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Image => "image",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LaneDefaults {
    pub resolution: u32,
    pub target_frames: Option<u32>,
    pub source_fps: Option<f32>,
    pub batch_size: u32,
    pub rank: u32,
    pub epochs: u32,
    pub learning_rate: f32,
    pub frame_extraction: Option<&'static str>,
    pub blocks_to_swap: u32,
    pub route_label: &'static str,
    pub route_summary: &'static str,
    pub hardware_note: &'static str,
    pub exploratory: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct TrainingLaneDefinition {
    pub backend_id: &'static str,
    pub label: &'static str,
    pub task: &'static str,
    pub family_id: &'static str,
    pub dataset_kind: TrainingDatasetKind,
    pub defaults: LaneDefaults,
}

pub const TRAINING_LANES: &[TrainingLaneDefinition] = &[
    TrainingLaneDefinition {
        backend_id: backend_registry::MUSUBI_WAN_BACKEND_ID,
        label: "Wan video lane",
        task: "t2v-1.3B",
        family_id: "wan",
        dataset_kind: TrainingDatasetKind::Video,
        defaults: LaneDefaults {
            resolution: 512,
            target_frames: Some(17),
            source_fps: Some(16.0),
            batch_size: 1,
            rank: 8,
            epochs: 1,
            learning_rate: 0.0001,
            frame_extraction: Some("head"),
            blocks_to_swap: 20,
            route_label: "Designed for cautious 8GB AMD tests",
            route_summary: "This is slower than a big-VRAM setup, but it proved the 512px, 17-frame, rank 8 Wan video smoke test end to end.",
            hardware_note: "Conservative consumer-GPU starter lane.",
            exploratory: false,
        },
    },
    TrainingLaneDefinition {
        backend_id: backend_registry::MUSUBI_WAN_IMAGE_BACKEND_ID,
        label: "Wan image visual lane",
        task: "t2v-1.3B",
        family_id: "wan",
        dataset_kind: TrainingDatasetKind::Image,
        defaults: LaneDefaults {
            resolution: 512,
            target_frames: None,
            source_fps: None,
            batch_size: 1,
            rank: 8,
            epochs: 1,
            learning_rate: 0.0001,
            frame_extraction: None,
            blocks_to_swap: 20,
            route_label: "Designed for cautious 8GB AMD tests",
            route_summary: "This image visual lane reuses the same cautious low-VRAM Musubi route as the first 1.3B video path.",
            hardware_note: "Conservative consumer-GPU starter lane.",
            exploratory: false,
        },
    },
    TrainingLaneDefinition {
        backend_id: backend_registry::MUSUBI_WAN_14B_BACKEND_ID,
        label: "Wan 14B video lane",
        task: "t2v-14B",
        family_id: "wan",
        dataset_kind: TrainingDatasetKind::Video,
        defaults: LaneDefaults {
            resolution: 320,
            target_frames: Some(9),
            source_fps: Some(10.0),
            batch_size: 1,
            rank: 2,
            epochs: 1,
            learning_rate: 0.0001,
            frame_extraction: Some("head"),
            blocks_to_swap: 38,
            route_label: "Extreme squeeze experimental route",
            route_summary: "This lane now follows the only 14B path that reached live training on the current WSL plus ROCm test rig: BF16-loaded weights, very low resolution, fewer frames, tiny rank, and maximum block swap. It is still experimental and may OOM before completing a step on 32GB-class systems.",
            hardware_note: "Treat Wan 14B as a stronger-hardware experiment. On this project's current 8GB Radeon plus 32GB system-RAM box, even the squeezed route can exhaust WSL RAM plus swap after training starts.",
            exploratory: true,
        },
    },
];

pub fn lane_definition(backend_id: &str) -> Option<&'static TrainingLaneDefinition> {
    TRAINING_LANES
        .iter()
        .find(|lane| lane.backend_id == backend_id)
}
