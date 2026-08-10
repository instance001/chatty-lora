use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::runner::TrainingRunner;
use crate::types::SystemTelemetrySnapshot;

pub const FIRST_RUN_DIRS: &[&str] = &[
    "inputs",
    "outputs",
    "outputs/training",
    "models",
    "models/wan",
    "models/wan/dependencies",
    "models/wan/dependencies/clip",
    "models/wan/dependencies/dit",
    "models/wan/dependencies/t5",
    "models/wan/dependencies/vae",
    "models/wan/diffusers",
    "models/wan/gguf",
    "models/wan21_t2v_1_3b",
    "models/wan21_t2v_1_3b/clip",
    "models/wan21_t2v_1_3b/dit",
    "models/wan21_t2v_1_3b/t5",
    "models/wan21_t2v_1_3b/vae",
    "models/flux",
    "models/flux/dependencies",
    "models/flux/gguf",
    "models/ai_assistant",
    "models/ai_assistant/dependencies",
    "models/ai_assistant/gguf",
    "models/audio",
    "models/audio/dependencies",
    "models/audio/gguf",
    "runtime",
    "runtime/musubi_tuner",
    "runtime/musubi-tuner",
    "runtime/ai_toolkit",
    "runtime/ai-toolkit",
    "runtime/kohya_ss",
    "runtime/kohya-ss",
    "runtime/sd-scripts",
    "runtime/onetrainer",
    "runtime/OneTrainer",
    "defaults",
    "config",
    "config/projects",
    "config/training",
    "config/training/generated",
    "config/source-fixes",
    "config/source-fixes/applied",
    "config/source-fixes/backups",
    "config/source-fixes/proposals",
    "bridge",
    "bridge/incoming_assets",
    "bridge/incoming_assets/dataset_candidates",
];

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
        std::fs::create_dir_all(&self.root)
            .with_context(|| format!("mkdir {}", self.root.display()))?;
        for relative in FIRST_RUN_DIRS {
            let dir = first_run_dir(&self.root, relative);
            std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
        }
        Ok(())
    }
}

pub fn first_run_dir(root: &std::path::Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for component in relative.split('/') {
        path.push(component);
    }
    path
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

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{FIRST_RUN_DIRS, ProjectPaths, first_run_dir};

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("chatty-lora-state-{label}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn project_paths(root: PathBuf) -> ProjectPaths {
        ProjectPaths {
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
        }
    }

    #[test]
    fn ensure_first_run_dirs_bootstraps_binary_layout() {
        let root = temp_dir("first-run-layout");
        let paths = project_paths(root.clone());

        paths.ensure_first_run_dirs().unwrap();

        assert!(paths.root.is_dir());
        assert!(paths.inputs.is_dir());
        assert!(paths.outputs.is_dir());
        assert!(paths.models.is_dir());
        assert!(paths.runtime.is_dir());
        assert!(paths.defaults.is_dir());
        assert!(paths.config.is_dir());
        assert!(paths.project_specs.is_dir());
        assert!(paths.training_config.is_dir());
        assert!(paths.training_generated.is_dir());
        assert!(paths.training_outputs.is_dir());
        assert!(paths.site_fix_notes.is_dir());

        for relative in FIRST_RUN_DIRS {
            let dir = first_run_dir(&root, relative);
            assert!(
                dir.is_dir(),
                "expected first-run folder {} to exist",
                dir.display()
            );
        }

        fs::remove_dir_all(root).unwrap();
    }
}
