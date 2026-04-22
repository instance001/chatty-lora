use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::Mutex,
};

use crate::{
    state::ProjectPaths,
    training,
    types::{TrainingLogLine, TrainingRunRequest, TrainingRunStatus, TrainingStageStatus},
};

const MAX_LOG_LINES: usize = 1_200;

static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
struct RunnerStage {
    id: &'static str,
    label: &'static str,
    script_name: &'static str,
}

const FULL_STAGES: &[RunnerStage] = &[
    RunnerStage {
        id: "preflight",
        label: "Preflight",
        script_name: "preflight.sh",
    },
    RunnerStage {
        id: "cache_latents",
        label: "Cache latents",
        script_name: "cache_latents.sh",
    },
    RunnerStage {
        id: "cache_text",
        label: "Cache text",
        script_name: "cache_text.sh",
    },
    RunnerStage {
        id: "train",
        label: "Train LoRA",
        script_name: "launch.sh",
    },
];

#[derive(Debug, Default)]
struct RunnerInner {
    active: bool,
    cancel_requested: bool,
    job_id: u64,
    state: String,
    project_slug: Option<String>,
    mode: Option<String>,
    current_stage: Option<String>,
    message: String,
    started_unix_seconds: Option<u64>,
    ended_unix_seconds: Option<u64>,
    process_id: Option<u32>,
    stages: Vec<TrainingStageStatus>,
    logs: VecDeque<TrainingLogLine>,
    output_files: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectSpecForRun {
    project_slug: String,
    training_backend_id: String,
    generated_training_path: Option<String>,
}

#[derive(Clone, Debug)]
struct ResolvedTrainingRun {
    project_slug: String,
    mode: String,
    generated_dir: PathBuf,
    stages: Vec<RunnerStage>,
}

#[derive(Debug, Default)]
pub struct TrainingRunner {
    inner: Mutex<RunnerInner>,
}

impl TrainingRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn status(&self) -> TrainingRunStatus {
        let inner = self.inner.lock().await;
        status_from_inner(&inner)
    }

    pub async fn start(
        self: Arc<Self>,
        paths: ProjectPaths,
        request: TrainingRunRequest,
    ) -> Result<TrainingRunStatus> {
        let run = resolve_training_run(&paths, request)?;
        let job_id = NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed);

        {
            let mut inner = self.inner.lock().await;
            if inner.active {
                bail!(
                    "A training run is already active. Stop or finish it before starting another."
                );
            }

            inner.active = true;
            inner.cancel_requested = false;
            inner.job_id = job_id;
            inner.state = "running".to_string();
            inner.project_slug = Some(run.project_slug.clone());
            inner.mode = Some(run.mode.clone());
            inner.current_stage = None;
            inner.message = "Training run queued.".to_string();
            inner.started_unix_seconds = Some(unix_now());
            inner.ended_unix_seconds = None;
            inner.process_id = None;
            inner.stages = run
                .stages
                .iter()
                .map(|stage| TrainingStageStatus {
                    id: stage.id.to_string(),
                    label: stage.label.to_string(),
                    state: "pending".to_string(),
                    exit_code: None,
                })
                .collect();
            inner.logs.clear();
            inner.output_files.clear();
        }

        self.push_log("runner", "system", "Starting guided training sequence.")
            .await;

        let runner = Arc::clone(&self);
        tokio::spawn(async move {
            runner.run_sequence(paths, run).await;
        });

        Ok(self.status().await)
    }

    pub async fn stop(&self) -> Result<TrainingRunStatus> {
        let process_id = {
            let mut inner = self.inner.lock().await;
            if !inner.active {
                inner.message = "No active training run to stop.".to_string();
                return Ok(status_from_inner(&inner));
            }
            inner.cancel_requested = true;
            inner.state = "stopping".to_string();
            inner.message =
                "Stop requested. Trying to terminate the active WSL process.".to_string();
            inner.process_id
        };

        self.push_log("runner", "system", "Stop requested from the UI.")
            .await;

        if let Some(pid) = process_id {
            let pid_string = pid.to_string();
            let output = Command::new("taskkill")
                .args(["/PID", &pid_string, "/T", "/F"])
                .output()
                .await
                .context("could not invoke taskkill to stop the training process")?;
            let summary = if output.status.success() {
                format!("taskkill accepted stop request for process {pid}.")
            } else {
                format!(
                    "taskkill returned {} while stopping process {pid}.",
                    output.status
                )
            };
            self.push_log("runner", "system", &summary).await;
        }

        Ok(self.status().await)
    }

    async fn run_sequence(self: Arc<Self>, paths: ProjectPaths, run: ResolvedTrainingRun) {
        for stage in run.stages {
            if self.cancel_requested().await {
                self.finish_cancelled("Training run cancelled before the next stage started.")
                    .await;
                return;
            }

            let script_path = run.generated_dir.join(stage.script_name);
            let result = self.run_stage(&stage, &script_path).await;
            match result {
                Ok(()) => {
                    self.set_stage_state(stage.id, "succeeded", Some(0)).await;
                }
                Err(error) => {
                    if self.cancel_requested().await {
                        self.set_stage_state(stage.id, "cancelled", None).await;
                        self.finish_cancelled("Training run stopped by user.").await;
                    } else {
                        self.set_stage_state(stage.id, "failed", None).await;
                        self.finish_failed(&format!("{} failed: {error:#}", stage.label))
                            .await;
                    }
                    return;
                }
            }
        }

        let outputs = collect_lora_outputs(&paths, &run.project_slug);
        self.finish_succeeded(outputs).await;
    }

    async fn run_stage(self: &Arc<Self>, stage: &RunnerStage, script_path: &Path) -> Result<()> {
        if !script_path.exists() {
            bail!("{} does not exist.", script_path.display());
        }

        self.set_current_stage(stage.id, &format!("Running {}.", stage.label))
            .await;

        let script = sh_quote(&windows_path_to_wsl(script_path));
        let command = format!("bash {script}");
        self.push_log(stage.id, "command", &format!("wsl bash -lc {command}"))
            .await;

        let mut child = Command::new("wsl")
            .args(["-d", training::WSL_DISTRO, "--", "bash", "-lc", &command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("could not start {}", stage.label))?;

        {
            let mut inner = self.inner.lock().await;
            inner.process_id = child.id();
        }

        let mut readers = Vec::new();
        if let Some(stdout) = child.stdout.take() {
            let runner = Arc::clone(self);
            let stage_id = stage.id.to_string();
            readers.push(tokio::spawn(async move {
                runner.read_stream(stage_id, "stdout", stdout).await;
            }));
        }
        if let Some(stderr) = child.stderr.take() {
            let runner = Arc::clone(self);
            let stage_id = stage.id.to_string();
            readers.push(tokio::spawn(async move {
                runner.read_stream(stage_id, "stderr", stderr).await;
            }));
        }

        let status = child
            .wait()
            .await
            .with_context(|| format!("could not wait for {}", stage.label))?;

        for reader in readers {
            let _ = reader.await;
        }

        {
            let mut inner = self.inner.lock().await;
            inner.process_id = None;
        }

        let exit_code = status.code();
        if status.success() {
            self.push_log(
                stage.id,
                "system",
                &format!("{} finished successfully.", stage.label),
            )
            .await;
            self.set_stage_state(stage.id, "succeeded", exit_code).await;
            Ok(())
        } else if self.cancel_requested().await {
            bail!("stopped by user")
        } else {
            self.set_stage_state(stage.id, "failed", exit_code).await;
            bail!("process exited with {}", status)
        }
    }

    async fn read_stream<R>(&self, stage_id: String, stream: &'static str, reader: R)
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            self.push_log(&stage_id, stream, &line).await;
        }
    }

    async fn push_log(&self, stage_id: &str, stream: &str, line: &str) {
        let mut inner = self.inner.lock().await;
        inner.logs.push_back(TrainingLogLine {
            unix_seconds: unix_now(),
            stage_id: stage_id.to_string(),
            stream: stream.to_string(),
            line: line.to_string(),
        });
        while inner.logs.len() > MAX_LOG_LINES {
            inner.logs.pop_front();
        }
    }

    async fn set_current_stage(&self, stage_id: &str, message: &str) {
        let mut inner = self.inner.lock().await;
        inner.current_stage = Some(stage_id.to_string());
        inner.message = message.to_string();
        if let Some(stage) = inner.stages.iter_mut().find(|stage| stage.id == stage_id) {
            stage.state = "running".to_string();
            stage.exit_code = None;
        }
    }

    async fn set_stage_state(&self, stage_id: &str, state: &str, exit_code: Option<i32>) {
        let mut inner = self.inner.lock().await;
        if let Some(stage) = inner.stages.iter_mut().find(|stage| stage.id == stage_id) {
            stage.state = state.to_string();
            stage.exit_code = exit_code;
        }
    }

    async fn cancel_requested(&self) -> bool {
        self.inner.lock().await.cancel_requested
    }

    async fn finish_succeeded(&self, output_files: Vec<String>) {
        let mut inner = self.inner.lock().await;
        inner.active = false;
        inner.cancel_requested = false;
        inner.state = "succeeded".to_string();
        inner.current_stage = None;
        inner.message = if output_files.is_empty() {
            "Training sequence finished, but no LoRA output file was found yet.".to_string()
        } else {
            format!(
                "Training sequence finished. {} LoRA output file{} found.",
                output_files.len(),
                if output_files.len() == 1 { "" } else { "s" }
            )
        };
        inner.ended_unix_seconds = Some(unix_now());
        inner.process_id = None;
        inner.output_files = output_files;
    }

    async fn finish_failed(&self, message: &str) {
        let mut inner = self.inner.lock().await;
        inner.active = false;
        inner.cancel_requested = false;
        inner.state = "failed".to_string();
        inner.message = message.to_string();
        inner.ended_unix_seconds = Some(unix_now());
        inner.process_id = None;
    }

    async fn finish_cancelled(&self, message: &str) {
        let mut inner = self.inner.lock().await;
        inner.active = false;
        inner.cancel_requested = false;
        inner.state = "cancelled".to_string();
        inner.message = message.to_string();
        inner.ended_unix_seconds = Some(unix_now());
        inner.process_id = None;
    }
}

fn resolve_training_run(
    paths: &ProjectPaths,
    request: TrainingRunRequest,
) -> Result<ResolvedTrainingRun> {
    let project_slug = request.project_slug.trim();
    if project_slug.is_empty()
        || project_slug.contains('/')
        || project_slug.contains('\\')
        || project_slug.contains("..")
    {
        bail!("Choose a valid saved training plan first.");
    }

    let project_path = paths.project_specs.join(format!("{project_slug}.json"));
    let contents = std::fs::read_to_string(&project_path)
        .with_context(|| format!("could not read {}", project_path.display()))?;
    let spec: ProjectSpecForRun = serde_json::from_str(&contents)
        .with_context(|| format!("could not parse {}", project_path.display()))?;

    if !training::is_musubi_wan_backend(&spec.training_backend_id) {
        bail!("Only the Wan/Musubi backends can run from the UI in this build.");
    }

    let Some(relative_path) = spec.generated_training_path else {
        bail!("This saved plan does not have a generated Wan/Musubi handoff folder.");
    };

    let generated_dir = paths.root.join(&relative_path);
    if !generated_dir.exists() {
        bail!("The generated handoff folder is missing: {relative_path}.");
    }

    let mode = request.mode.trim();
    let mode = if mode.is_empty() { "full" } else { mode };
    let stages = stages_for_mode(mode)?;
    for stage in &stages {
        let script_path = generated_dir.join(stage.script_name);
        if !script_path.exists() {
            bail!(
                "The generated handoff is missing {}. Regenerate the training plan before running.",
                stage.script_name
            );
        }
    }

    Ok(ResolvedTrainingRun {
        project_slug: spec.project_slug,
        mode: mode.to_string(),
        generated_dir,
        stages,
    })
}

fn stages_for_mode(mode: &str) -> Result<Vec<RunnerStage>> {
    if mode == "full" {
        return Ok(FULL_STAGES.to_vec());
    }

    let stage = FULL_STAGES
        .iter()
        .find(|stage| stage.id == mode)
        .cloned()
        .with_context(|| format!("Unknown training run mode: {mode}"))?;
    Ok(vec![stage])
}

fn collect_lora_outputs(paths: &ProjectPaths, project_slug: &str) -> Vec<String> {
    let lora_dir = paths.training_outputs.join(project_slug).join("loras");
    let Ok(entries) = std::fs::read_dir(&lora_dir) else {
        return Vec::new();
    };

    let mut outputs = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("safetensors"))
                .unwrap_or(false)
        })
        .map(|path| {
            path.strip_prefix(&paths.root)
                .unwrap_or(&path)
                .display()
                .to_string()
        })
        .collect::<Vec<_>>();
    outputs.sort();
    outputs
}

fn status_from_inner(inner: &RunnerInner) -> TrainingRunStatus {
    TrainingRunStatus {
        job_id: inner.job_id,
        state: if inner.state.is_empty() {
            "idle".to_string()
        } else {
            inner.state.clone()
        },
        project_slug: inner.project_slug.clone(),
        mode: inner.mode.clone(),
        current_stage: inner.current_stage.clone(),
        message: if inner.message.is_empty() {
            "No training run has been started yet.".to_string()
        } else {
            inner.message.clone()
        },
        started_unix_seconds: inner.started_unix_seconds,
        ended_unix_seconds: inner.ended_unix_seconds,
        process_id: inner.process_id,
        stages: inner.stages.clone(),
        logs: inner.logs.iter().cloned().collect(),
        output_files: inner.output_files.clone(),
    }
}

fn windows_path_to_wsl(path: &Path) -> String {
    let path = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
        .replace('\\', "/");
    let path = path
        .strip_prefix("//?/")
        .or_else(|| path.strip_prefix("/?/"))
        .unwrap_or(&path)
        .to_string();
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        let rest = path[2..].trim_start_matches('/');
        format!("/mnt/{}/{}", drive, rest)
    } else {
        path
    }
}

fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
