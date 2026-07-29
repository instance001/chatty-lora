use std::path::PathBuf;

use anyhow::{Context, Result};

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

impl ProjectPaths {
    pub fn resolve() -> Result<Self> {
        let root = resolve_app_root()?;
        let paths = Self {
            inputs: root.join("inputs"),
            outputs: root.join("outputs"),
            models: root.join("models"),
            runtime: root.join("runtime"),
            defaults: root.join("defaults"),
            config: root.join("config"),
            project_specs: root.join("config").join("projects"),
            training_config: root.join("config").join("training"),
            training_generated: root.join("config").join("training").join("generated"),
            training_outputs: root.join("outputs").join("training"),
            site_fix_notes: root.join("config").join("source-fixes"),
            root,
        };
        paths.ensure_first_run_dirs()?;
        Ok(paths)
    }

    pub fn ensure_first_run_dirs(&self) -> Result<()> {
        let dirs = [
            &self.inputs,
            &self.outputs,
            &self.models,
            &self.models.join("wan"),
            &self.models.join("wan").join("dependencies"),
            &self.models.join("wan").join("dependencies").join("clip"),
            &self.models.join("wan").join("dependencies").join("dit"),
            &self.models.join("wan").join("dependencies").join("t5"),
            &self.models.join("wan").join("dependencies").join("vae"),
            &self.models.join("wan").join("gguf"),
            &self.models.join("flux"),
            &self.models.join("flux").join("dependencies"),
            &self.models.join("flux").join("gguf"),
            &self.models.join("ai_assistant"),
            &self.models.join("ai_assistant").join("gguf"),
            &self.models.join("audio"),
            &self.models.join("audio").join("dependencies"),
            &self.models.join("audio").join("gguf"),
            &self.runtime,
            &self.config,
            &self.project_specs,
            &self.training_config,
            &self.training_generated,
            &self.training_outputs,
            &self.site_fix_notes,
            &self.site_fix_notes.join("applied"),
            &self.site_fix_notes.join("backups"),
            &self.root.join("bridge"),
            &self.root.join("bridge").join("incoming_assets"),
            &self
                .root
                .join("bridge")
                .join("incoming_assets")
                .join("dataset_candidates"),
        ];

        for dir in dirs {
            std::fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
        }
        Ok(())
    }
}

fn resolve_app_root() -> Result<PathBuf> {
    if let Ok(value) = std::env::var("CHATTY_LORA_BASE_PATH") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        for dir in exe.ancestors().filter(|path| path.is_dir()) {
            if dir.join("Cargo.toml").is_file()
                && dir.join("src").is_dir()
                && dir.join("static").is_dir()
            {
                return Ok(dir.to_path_buf());
            }
        }

        if let Some(dir) = exe.parent() {
            return Ok(dir.to_path_buf());
        }
    }

    std::env::current_dir().context("could not determine app root")
}
