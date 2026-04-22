use std::path::PathBuf;

use crate::runner::TrainingRunner;
use crate::types::SystemTelemetrySnapshot;

#[derive(Clone)]
pub struct AppState {
    pub paths: ProjectPaths,
    pub http: reqwest::Client,
    pub training_runner: std::sync::Arc<TrainingRunner>,
    pub system_telemetry: std::sync::Arc<tokio::sync::RwLock<SystemTelemetrySnapshot>>,
}

#[derive(Clone)]
pub struct ProjectPaths {
    pub root: PathBuf,
    pub inputs: PathBuf,
    pub outputs: PathBuf,
    pub models: PathBuf,
    pub runtime: PathBuf,
    pub defaults: PathBuf,
    pub config: PathBuf,
    pub project_specs: PathBuf,
    pub training_config: PathBuf,
    pub training_generated: PathBuf,
    pub training_outputs: PathBuf,
    pub site_fix_notes: PathBuf,
}
