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
        },
    },
];

pub fn lane_definition(backend_id: &str) -> Option<&'static TrainingLaneDefinition> {
    TRAINING_LANES
        .iter()
        .find(|lane| lane.backend_id == backend_id)
}
