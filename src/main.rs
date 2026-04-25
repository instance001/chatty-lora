mod backend_registry;
mod builder;
mod datasets;
mod helper;
mod lane_registry;
mod model_registry;
mod runner;
mod sources;
mod state;
mod training;
mod types;

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use axum::{
    Json, Router,
    extract::State,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use serde::Deserialize;
use state::{AppState, ProjectPaths};
use tokio::{fs, process::Command as TokioCommand, sync::RwLock, time::timeout};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::info;
use types::{
    BaseModelOption, BuilderDeleteProjectRequest, BuilderPanel, BuilderPrepareRequest,
    DashboardResponse, DatasetCreateRequest, DatasetPreflightSummary, DatasetSummary,
    DatasetVideoSummary, FolderSummary, HelperQueryRequest, LibraryItem, LocalDatasetImportRequest,
    MaterialPanel, ModelFamilySummary, ModelItem, ModelSummary, OpenLocalPathRequest,
    OpenLocalPathResponse, RuntimeSummary, SearchPreviewRequest, SourceFixApplyPreviewRequest,
    SourceFixApplyRequest, SourceFixOpenRequest, SourceFixProposalSaveRequest,
    SourceFixProposeRequest, SourceFixSaveRequest, SourceRegistryUpdateRequest,
    SystemTelemetrySnapshot, TrainingRunRequest,
};
use walkdir::WalkDir;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let root = std::env::current_dir().context("could not determine project root")?;
    let paths = ProjectPaths {
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

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .context("could not build HTTP client")?;

    let system_telemetry = Arc::new(RwLock::new(initial_system_telemetry()));
    spawn_system_telemetry_sampler(system_telemetry.clone());

    let state = Arc::new(AppState {
        paths,
        http,
        training_runner: Arc::new(runner::TrainingRunner::new()),
        system_telemetry,
    });
    let app = Router::new()
        .route("/api/dashboard", get(get_dashboard))
        .route("/api/sources", get(get_sources).post(save_sources))
        .route("/api/search/preview", post(preview_search))
        .route("/api/datasets/create", post(create_dataset))
        .route("/api/datasets/import-local", post(import_local_dataset))
        .route("/api/builder/prepare", post(prepare_builder_project))
        .route("/api/builder/delete", post(delete_builder_project))
        .route("/api/training/status", get(get_training_status))
        .route("/api/training/run", post(start_training_run))
        .route("/api/training/stop", post(stop_training_run))
        .route("/api/telemetry/system", get(system_telemetry_status))
        .route("/api/open-local-path", post(open_local_path))
        .route("/api/helper/query", post(query_helper))
        .route("/api/source-fix/open", post(open_source_fix_shell))
        .route("/api/source-fix/propose", post(propose_source_fix_shell))
        .route(
            "/api/source-fix/proposal-save",
            post(save_source_fix_proposal),
        )
        .route(
            "/api/source-fix/apply-preview",
            post(preview_source_fix_apply),
        )
        .route("/api/source-fix/apply", post(apply_source_fix))
        .route("/api/source-fix/save", post(save_source_fix_shell))
        .route("/", get(index))
        .nest_service("/static", ServeDir::new("static"))
        .fallback(get(index))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = "127.0.0.1:7879".parse().unwrap();
    info!("Chatty-lora is running at http://{}", addr);
    println!("Chatty-lora is running at http://{}", addr);

    if std::env::var("CHATTY_LORA_NO_BROWSER").ok().as_deref() != Some("1") {
        let _ = webbrowser::open(&format!("http://{}", addr));
    }

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("failed to bind server")?;
    axum::serve(listener, app)
        .await
        .context("server exited unexpectedly")?;
    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "chatty_lora=info,tower_http=info".into()),
        )
        .try_init();
}

fn initial_system_telemetry() -> SystemTelemetrySnapshot {
    SystemTelemetrySnapshot {
        supported: cfg!(target_os = "windows"),
        label: "ECG Window".to_string(),
        note: if cfg!(target_os = "windows") {
            "CPU and GPU heartbeat view for split cache/training workloads.".to_string()
        } else {
            "ECG Window sampling is currently available on Windows only.".to_string()
        },
        cpu_label: "CPU".to_string(),
        gpu_label: "GPU".to_string(),
        current_cpu_percent: 0.0,
        current_gpu_percent: 0.0,
        cpu_history: Vec::new(),
        gpu_history: Vec::new(),
    }
}

#[derive(Debug, Deserialize)]
struct WindowsSystemTelemetryProbe {
    #[serde(default)]
    cpu: f32,
    #[serde(default)]
    gpu: f32,
}

fn spawn_system_telemetry_sampler(target: Arc<RwLock<SystemTelemetrySnapshot>>) {
    tokio::spawn(async move {
        if !cfg!(target_os = "windows") {
            return;
        }

        if let Ok(label) = query_windows_gpu_label().await {
            let mut guard = target.write().await;
            guard.gpu_label = label;
        }

        let mut cpu_history = VecDeque::with_capacity(90);
        let mut gpu_history = VecDeque::with_capacity(90);
        let mut last_error_note = None::<String>;

        loop {
            match sample_windows_system_activity_percent().await {
                Ok(sample) => {
                    push_history_sample(&mut cpu_history, sample.cpu);
                    push_history_sample(&mut gpu_history, sample.gpu);

                    let mut guard = target.write().await;
                    guard.supported = true;
                    guard.current_cpu_percent = clamp_percent(sample.cpu);
                    guard.current_gpu_percent = clamp_percent(sample.gpu);
                    guard.cpu_history = cpu_history.iter().copied().collect();
                    guard.gpu_history = gpu_history.iter().copied().collect();
                    if last_error_note.take().is_some() {
                        guard.note =
                            "CPU and GPU heartbeat view for split cache/training workloads."
                                .to_string();
                    }
                }
                Err(error) => {
                    let note = format!(
                        "ECG Window uses Windows performance counters. Sampling is temporarily unavailable: {error}"
                    );
                    if last_error_note.as_deref() != Some(note.as_str()) {
                        let mut guard = target.write().await;
                        guard.note = note.clone();
                        last_error_note = Some(note);
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(1800)).await;
        }
    });
}

fn push_history_sample(history: &mut VecDeque<f32>, value: f32) {
    if history.len() >= 90 {
        history.pop_front();
    }
    history.push_back(clamp_percent(value));
}

async fn query_windows_gpu_label() -> Result<String> {
    let script = r#"
$adapter = Get-CimInstance Win32_VideoController | Select-Object -First 1 -ExpandProperty Name
if ([string]::IsNullOrWhiteSpace($adapter)) { 'GPU' } else { $adapter.Trim() }
"#;
    let output = timeout(
        Duration::from_secs(4),
        TokioCommand::new("powershell.exe")
            .args(["-NoProfile", "-Command", script])
            .output(),
    )
    .await
    .context("timed out querying Windows GPU adapter label")?
    .context("failed to launch PowerShell for GPU adapter query")?;

    if !output.status.success() {
        bail!("PowerShell GPU adapter query failed");
    }

    let label = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if label.is_empty() {
        Ok("GPU".to_string())
    } else {
        Ok(label)
    }
}

async fn sample_windows_system_activity_percent() -> Result<WindowsSystemTelemetryProbe> {
    let script = r#"
$gpuValues = @(
  Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine |
    Where-Object { $_.Name -match 'engtype_(3D|Compute|Video|Copy)' } |
    ForEach-Object { [double]$_.UtilizationPercentage }
)
$gpu = if ($gpuValues.Count -eq 0) {
  0
} else {
  [math]::Round((($gpuValues | Measure-Object -Maximum).Maximum), 1)
}
$cpuValue = Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor |
  Where-Object { $_.Name -eq '_Total' } |
  Select-Object -First 1 -ExpandProperty PercentProcessorTime
if ($null -eq $cpuValue) { $cpuValue = 0 }
[pscustomobject]@{
  cpu = [math]::Round([double]$cpuValue, 1)
  gpu = [math]::Round([double]$gpu, 1)
} | ConvertTo-Json -Compress
"#;

    let output = timeout(
        Duration::from_secs(6),
        TokioCommand::new("powershell.exe")
            .args(["-NoProfile", "-Command", script])
            .output(),
    )
    .await
    .context("timed out querying Windows ECG Window counters")?
    .context("failed to launch PowerShell for ECG Window query")?;

    if !output.status.success() {
        bail!("PowerShell ECG Window query failed");
    }

    serde_json::from_slice::<WindowsSystemTelemetryProbe>(&output.stdout)
        .context("could not parse ECG Window CPU/GPU sample")
}

fn clamp_percent(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        0.0
    }
}

async fn index() -> impl IntoResponse {
    match fs::read_to_string("static/index.html").await {
        Ok(contents) => Html(contents).into_response(),
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Missing static/index.html",
        )
            .into_response(),
    }
}

async fn get_dashboard(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match build_dashboard(&state.paths) {
        Ok(dashboard) => Json(dashboard).into_response(),
        Err(error) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": error.to_string()
            })),
        )
            .into_response(),
    }
}

async fn get_sources(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match sources::load_registry_payload(&state.paths) {
        Ok(payload) => Json(payload).into_response(),
        Err(error) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn save_sources(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SourceRegistryUpdateRequest>,
) -> impl IntoResponse {
    match sources::registry::save_sources(&state.paths, request) {
        Ok(()) => match sources::load_registry_payload(&state.paths) {
            Ok(payload) => Json(payload).into_response(),
            Err(error) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": error.to_string() })),
            )
                .into_response(),
        },
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn preview_search(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SearchPreviewRequest>,
) -> impl IntoResponse {
    match sources::search_preview(&state.http, &state.paths, request).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn create_dataset(
    State(state): State<Arc<AppState>>,
    Json(request): Json<DatasetCreateRequest>,
) -> impl IntoResponse {
    match datasets::create_dataset(&state.http, &state.paths, request).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn import_local_dataset(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LocalDatasetImportRequest>,
) -> impl IntoResponse {
    match datasets::import_local_dataset(&state.paths, request).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn prepare_builder_project(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BuilderPrepareRequest>,
) -> impl IntoResponse {
    match builder::prepare_project(&state.paths, request) {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn delete_builder_project(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BuilderDeleteProjectRequest>,
) -> impl IntoResponse {
    let active_status = state.training_runner.status().await;
    if matches!(active_status.state.as_str(), "running" | "stopping")
        && active_status.project_slug.as_deref() == Some(request.project_slug.trim())
    {
        return (
            axum::http::StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "Stop this training run before deleting its saved plan."
            })),
        )
            .into_response();
    }

    match builder::delete_project(&state.paths, request) {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn get_training_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.training_runner.status().await).into_response()
}

async fn start_training_run(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TrainingRunRequest>,
) -> impl IntoResponse {
    match state
        .training_runner
        .clone()
        .start(state.paths.clone(), request)
        .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn stop_training_run(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.training_runner.stop().await {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn system_telemetry_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.system_telemetry.read().await.clone()).into_response()
}

async fn open_local_path(
    State(state): State<Arc<AppState>>,
    Json(request): Json<OpenLocalPathRequest>,
) -> impl IntoResponse {
    match open_project_path(&state.paths, &request.relative_path) {
        Ok(opened_path) => Json(OpenLocalPathResponse { opened_path }).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn query_helper(Json(request): Json<HelperQueryRequest>) -> impl IntoResponse {
    Json(helper::answer(request)).into_response()
}

async fn open_source_fix_shell(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SourceFixOpenRequest>,
) -> impl IntoResponse {
    match sources::site_fix::open_shell(&state.paths, request) {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn save_source_fix_shell(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SourceFixSaveRequest>,
) -> impl IntoResponse {
    match sources::site_fix::save_shell(&state.paths, request) {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn propose_source_fix_shell(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SourceFixProposeRequest>,
) -> impl IntoResponse {
    match sources::site_fix::propose_fix(&state.http, &state.paths, request).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn save_source_fix_proposal(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SourceFixProposalSaveRequest>,
) -> impl IntoResponse {
    match sources::site_fix::save_proposal_snapshot(&state.paths, request) {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn preview_source_fix_apply(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SourceFixApplyPreviewRequest>,
) -> impl IntoResponse {
    match sources::site_fix::preview_apply(&state.paths, request) {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn apply_source_fix(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SourceFixApplyRequest>,
) -> impl IntoResponse {
    match sources::site_fix::apply_fix(&state.paths, request) {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

fn build_dashboard(paths: &ProjectPaths) -> Result<DashboardResponse> {
    let (input_summary, input_files) = scan_media_folder(&paths.inputs)?;
    let (output_summary, output_files) = scan_media_folder(&paths.outputs)?;
    let model_summary = scan_models(&paths.models)?;
    let runtime_summary = scan_runtime(paths);
    let source_registry = sources::load_registry_payload(paths)?;
    let site_fix_summaries = sources::site_fix_summaries(paths)?;
    let curated_datasets = scan_curated_datasets(&paths.inputs)?;
    let recommended_dataset_slug = curated_datasets.first().map(|dataset| dataset.slug.clone());
    let prepared_projects = builder::scan_project_specs(paths)?;
    let wan_training = training::scan_wan_training(paths);
    let training_backends = training::scan_backends_with_wan_status(paths, wan_training.clone());
    let recommended_training_backend_id = training::recommended_backend_id(&training_backends);

    let mut base_model_options = Vec::new();
    if wan_training.model_bundle_ready {
        base_model_options.push(BaseModelOption {
            value: "Wan 2.1 T2V 1.3B bundle".to_string(),
            label: "Wan 2.1 T2V 1.3B bundle".to_string(),
            family_id: model_registry::WAN_FAMILY_ID.to_string(),
            family_label: "Wan".to_string(),
            detail: "Resolved from the Wan dependency bundle used by the Musubi training lanes."
                .to_string(),
        });
    }
    base_model_options.extend(model_summary.families.iter().flat_map(|family| {
        if !model_registry::include_family_in_training_base_model_picker(&family.id) {
            return Vec::new().into_iter();
        }

        family
            .items
            .iter()
            .filter(move |item| item.kind == "GGUF")
            .filter(move |_item| {
                model_registry::include_family_in_training_base_model_picker(&family.id)
            })
            .map(move |item| BaseModelOption {
                value: item.name.clone(),
                label: item.name.clone(),
                family_id: family.id.clone(),
                family_label: family.label.clone(),
                detail: item.relative_path.clone(),
            })
            .collect::<Vec<_>>()
            .into_iter()
    }));

    Ok(DashboardResponse {
        app_name: "Chatty-lora",
        project_root: paths.root.display().to_string(),
        materials: MaterialPanel {
            input_summary,
            output_summary,
            input_files,
            output_files,
            model_summary,
            runtime_summary,
            source_registry,
            site_fix_summaries,
        },
        builder: BuilderPanel {
            project_name_suggestion: recommended_dataset_slug
                .as_deref()
                .map(|slug| format!("{}-lora", slug))
                .unwrap_or_else(|| "coastal-kookaburra-lora".to_string()),
            base_model_options,
            training_backends,
            recommended_training_backend_id,
            wan_training,
            recommended_dataset_slug,
            curated_datasets,
            prepared_projects,
            status_lines: vec![
                "Rust server chassis is ready.".to_string(),
                "Source registry and batched preview search are now wired.".to_string(),
                "Wan/Musubi plans now generate WSL handoff scripts and can run through the guided app runner.".to_string(),
            ],
            starter_notes: vec![
                "Use the Materials page to select polite sources, search in 3-page batches, and preview material before curation."
                    .to_string(),
                "Use the Builder page to pick a curated dataset, choose the Wan/Musubi backend, shape the starter settings, and save a reusable local training plan."
                    .to_string(),
                "Saved Wan plans show an app runner plus manual fallback commands, so the first run stays visible and reversible.".to_string(),
                model_registry::family_layout_note(),
                "This sister tool is standalone and does not share live code paths with Chatty-art."
                    .to_string(),
            ],
        },
    })
}

fn open_project_path(paths: &ProjectPaths, relative_path: &str) -> Result<String> {
    let trimmed = relative_path.trim();
    if trimmed.is_empty() {
        bail!("No local path was provided.");
    }

    let requested_path = paths.root.join(trimmed);
    let target_path = requested_path
        .canonicalize()
        .with_context(|| format!("could not resolve {}", requested_path.display()))?;
    let root_path = paths
        .root
        .canonicalize()
        .with_context(|| format!("could not resolve {}", paths.root.display()))?;

    if !target_path.starts_with(&root_path) {
        bail!("Refusing to open a path outside the Chatty-lora project folder.");
    }

    if target_path.is_file() {
        Command::new("explorer")
            .arg(format!("/select,{}", target_path.display()))
            .spawn()
            .context("could not ask Windows Explorer to select the file")?;
    } else {
        Command::new("explorer")
            .arg(&target_path)
            .spawn()
            .context("could not ask Windows Explorer to open the folder")?;
    }

    Ok(target_path.display().to_string())
}

fn scan_media_folder(folder: &Path) -> Result<(FolderSummary, Vec<LibraryItem>)> {
    if !folder.exists() {
        return Ok((FolderSummary::default(), Vec::new()));
    }

    let mut summary = FolderSummary::default();
    let mut items = Vec::new();

    for entry in WalkDir::new(folder)
        .min_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.file_name().to_string_lossy() != ".gitkeep")
    {
        let path = entry.path();
        let metadata = entry.metadata().ok();
        let bytes = metadata.map(|meta| meta.len()).unwrap_or(0);
        let kind = classify_media_kind(path);

        summary.total += 1;
        match kind.as_str() {
            "Image" => summary.images += 1,
            "Audio" => summary.audio += 1,
            "Video" => summary.video += 1,
            "Text" => summary.text += 1,
            _ => summary.other += 1,
        }

        items.push(LibraryItem {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            relative_path: path
                .strip_prefix(folder)
                .unwrap_or(path)
                .display()
                .to_string(),
            kind,
            bytes,
        });
    }

    items.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });
    Ok((summary, items))
}

fn scan_models(folder: &Path) -> Result<ModelSummary> {
    if !folder.exists() {
        return Ok(ModelSummary {
            total: 0,
            gguf: 0,
            safetensors: 0,
            checkpoints: 0,
            other: 0,
            families: Vec::new(),
            items: Vec::new(),
        });
    }

    let mut summary = ModelSummary {
        total: 0,
        gguf: 0,
        safetensors: 0,
        checkpoints: 0,
        other: 0,
        families: Vec::new(),
        items: Vec::new(),
    };
    let mut families = BTreeMap::<String, ModelFamilySummary>::new();

    for entry in WalkDir::new(folder)
        .min_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.file_name().to_string_lossy() != ".gitkeep")
    {
        let path = entry.path();
        let kind = classify_model_kind(path);
        let relative_path = path
            .strip_prefix(folder)
            .unwrap_or(path)
            .display()
            .to_string();
        let (family_id, family_label, family_purpose, family_root) =
            classify_model_family(&relative_path);
        summary.total += 1;
        match kind.as_str() {
            "GGUF" => summary.gguf += 1,
            "Safetensors" => summary.safetensors += 1,
            "Checkpoint" => summary.checkpoints += 1,
            _ => summary.other += 1,
        }

        let item = ModelItem {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            kind: kind.clone(),
            family_id: family_id.to_string(),
            family_label: family_label.to_string(),
            relative_path,
        };

        summary.items.push(item.clone());

        let family_summary =
            families
                .entry(family_id.to_string())
                .or_insert_with(|| ModelFamilySummary {
                    id: family_id.to_string(),
                    label: family_label.to_string(),
                    purpose: family_purpose.to_string(),
                    included_in_training_base_model_picker:
                        model_registry::include_family_in_training_base_model_picker(family_id),
                    relative_root: family_root.to_string(),
                    total: 0,
                    gguf: 0,
                    safetensors: 0,
                    checkpoints: 0,
                    other: 0,
                    items: Vec::new(),
                });
        family_summary.total += 1;
        match kind.as_str() {
            "GGUF" => family_summary.gguf += 1,
            "Safetensors" => family_summary.safetensors += 1,
            "Checkpoint" => family_summary.checkpoints += 1,
            _ => family_summary.other += 1,
        }
        family_summary.items.push(item);
    }

    summary.items.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });
    for family in families.values_mut() {
        family.items.sort_by(|a, b| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        });
    }
    summary.families = families.into_values().collect();
    Ok(summary)
}

fn scan_curated_datasets(folder: &Path) -> Result<Vec<DatasetSummary>> {
    if !folder.exists() {
        return Ok(Vec::new());
    }

    let mut datasets = Vec::new();
    for entry in
        std::fs::read_dir(folder).with_context(|| format!("could not read {}", folder.display()))?
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }

        let manifest_path = path.join("metadata.json");
        let manifest = if manifest_path.exists() {
            std::fs::read_to_string(&manifest_path)
                .ok()
                .and_then(|contents| serde_json::from_str::<DatasetManifestRecord>(&contents).ok())
        } else {
            None
        };

        let mut images = 0usize;
        let mut audio = 0usize;
        let mut video = 0usize;
        let mut other = 0usize;
        let mut total_files = 0usize;
        let mut caption_files = 0usize;
        let mut video_paths = Vec::new();

        for file in WalkDir::new(&path)
            .min_depth(1)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| {
                let name = entry.file_name().to_string_lossy();
                name != ".gitkeep" && name != "metadata.json"
            })
        {
            total_files += 1;
            if is_caption_file(file.path()) {
                caption_files += 1;
            }

            match classify_media_kind(file.path()).as_str() {
                "Image" => images += 1,
                "Audio" => audio += 1,
                "Video" => {
                    video += 1;
                    video_paths.push(file.path().to_path_buf());
                }
                _ => other += 1,
            }
        }

        let source_count = manifest
            .as_ref()
            .map(|manifest| {
                manifest
                    .items
                    .iter()
                    .map(|item| item.source_label.clone())
                    .collect::<BTreeSet<_>>()
                    .len()
            })
            .unwrap_or(0);

        let display_name = manifest
            .as_ref()
            .map(|manifest| manifest.dataset_name.clone())
            .unwrap_or_else(|| {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .replace('-', " ")
            });

        let created_unix_seconds = manifest
            .as_ref()
            .map(|manifest| manifest.created_unix_seconds);
        let slug = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let preflight = build_dataset_preflight(
            &path,
            total_files,
            images,
            audio,
            video,
            other,
            source_count,
            manifest.is_some(),
            caption_files,
            &video_paths,
        );

        datasets.push(DatasetSummary {
            slug: slug.clone(),
            display_name,
            relative_path: path
                .strip_prefix(folder)
                .unwrap_or(&path)
                .display()
                .to_string(),
            total_files,
            images,
            audio,
            video,
            other,
            source_count,
            manifest_present: manifest.is_some(),
            created_unix_seconds,
            preflight,
        });
    }

    datasets.sort_by(|left, right| {
        right
            .created_unix_seconds
            .unwrap_or(0)
            .cmp(&left.created_unix_seconds.unwrap_or(0))
            .then_with(|| left.slug.cmp(&right.slug))
    });
    Ok(datasets)
}

const DATASET_VIDEO_PROBE_LIMIT: usize = 16;

fn build_dataset_preflight(
    dataset_path: &Path,
    total_files: usize,
    images: usize,
    audio: usize,
    video: usize,
    other: usize,
    source_count: usize,
    manifest_present: bool,
    caption_files: usize,
    video_paths: &[PathBuf],
) -> DatasetPreflightSummary {
    let mut notes = Vec::new();
    let mut status = "ok".to_string();
    let (label, badge) = if total_files == 0 {
        status = "blocked".to_string();
        notes.push(
            "This folder is empty, so there is nothing for the Wan trainer to learn from yet."
                .to_string(),
        );
        ("Empty dataset", "blocked")
    } else if video == 0 {
        status = "blocked".to_string();
        notes.push(
            "The first supported Wan lane needs video clips. Images and audio are useful later, but this trainer will ignore them for now."
                .to_string(),
        );
        ("No Wan video rows", "blocked")
    } else if video < 4 {
        status = "caution".to_string();
        notes.push(
            "One to three clips can test the wiring, but the result will probably overfit or fail to learn the concept cleanly."
                .to_string(),
        );
        ("Very thin video set", "thin")
    } else if video == 4 {
        notes.push(
            "Four clips matches the current Wan video smoke-test shape: enough to prove the pipeline, still thin for a serious LoRA."
                .to_string(),
        );
        ("Smoke-test ready", "smoke test")
    } else if video <= 12 {
        notes.push(
            "This is a small focused set, which is usually the friendliest starting point for the Wan video lane."
                .to_string(),
        );
        ("Small focused set", "good starter")
    } else {
        notes.push(
            "This is a larger dataset. Review it for duplicates or mixed concepts before spending a long run on it."
                .to_string(),
        );
        ("Larger set", "review")
    };

    if !manifest_present {
        notes.push(
            "No curation manifest was found. Hand-added files are fine, but source labels and original URLs will be limited."
                .to_string(),
        );
    } else if source_count == 0 {
        notes.push(
            "The curation manifest exists, but it does not list source labels yet.".to_string(),
        );
    }

    if caption_files == 0 {
        notes.push(
            "No sidecar caption files were found. The generated Wan plan will lean on the trigger phrase, filename, and concept summary."
                .to_string(),
        );
    }

    if images > 0 || audio > 0 {
        notes.push(format!(
            "This folder also contains {} image file(s) and {} audio file(s). The current Wan video lane will leave those alone.",
            images, audio
        ));
    }

    if other > 0 && other != caption_files {
        notes.push(format!(
            "{} uncategorized file(s) were found. That is okay, but they will not become Wan video rows.",
            other.saturating_sub(caption_files)
        ));
    }

    let mut video_probe_available = false;
    let mut video_details = Vec::new();
    let mut failed_probe_count = 0usize;
    for video_path in video_paths.iter().take(DATASET_VIDEO_PROBE_LIMIT) {
        match probe_video(video_path, dataset_path) {
            Ok(detail) => {
                video_probe_available = true;
                video_details.push(detail);
            }
            Err(VideoProbeError::MissingCommand) => {
                notes.push(
                    "ffprobe was not available, so Chatty-lora could not read clip durations or resolutions in Windows. The generated WSL preflight still performs deeper checks before training."
                        .to_string(),
                );
                break;
            }
            Err(VideoProbeError::Failed) => {
                video_probe_available = true;
                failed_probe_count += 1;
            }
        }
    }

    if failed_probe_count > 0 {
        notes.push(format!(
            "Could not inspect {} video file(s). If training fails during caching, check whether those clips play normally.",
            failed_probe_count
        ));
    }

    if video_paths.len() > DATASET_VIDEO_PROBE_LIMIT {
        notes.push(format!(
            "Only the first {} video clip(s) were probed to keep the dashboard quick.",
            DATASET_VIDEO_PROBE_LIMIT
        ));
    }

    let durations: Vec<f32> = video_details
        .iter()
        .filter_map(|detail| detail.duration_seconds)
        .collect();
    let total_duration_seconds =
        (!durations.is_empty()).then(|| round_seconds(durations.iter().copied().sum::<f32>()));
    let min_duration_seconds = durations
        .iter()
        .copied()
        .reduce(f32::min)
        .map(round_seconds);
    let max_duration_seconds = durations
        .iter()
        .copied()
        .reduce(f32::max)
        .map(round_seconds);

    if let Some(total_duration) = total_duration_seconds {
        if total_duration < 20.0 && status == "ok" {
            status = "caution".to_string();
            notes.push(
                "Total probed video time is under 20 seconds. Fine for a smoke test, thin for a LoRA you expect to generalize."
                    .to_string(),
            );
        }
    }

    if let Some(min_duration) = min_duration_seconds {
        if min_duration < 2.0 {
            notes.push(
                "At least one probed clip is under 2 seconds. Very short clips may teach motion poorly."
                    .to_string(),
            );
        }
    }

    if let Some(max_duration) = max_duration_seconds {
        if max_duration > 30.0 {
            notes.push(
                "At least one probed clip is over 30 seconds. Long clips can slow VAE and text caching."
                    .to_string(),
            );
        }
    }

    let resolution_summary = summarize_video_resolutions(&video_details);

    DatasetPreflightSummary {
        status,
        label: label.to_string(),
        badge: badge.to_string(),
        notes,
        caption_files,
        video_probe_available,
        probed_video_count: video_details.len(),
        total_duration_seconds,
        min_duration_seconds,
        max_duration_seconds,
        resolution_summary,
        video_details,
    }
}

fn probe_video(
    path: &Path,
    dataset_path: &Path,
) -> std::result::Result<DatasetVideoSummary, VideoProbeError> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=width,height,avg_frame_rate:format=duration")
        .arg("-of")
        .arg("json")
        .arg(path)
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                VideoProbeError::MissingCommand
            } else {
                VideoProbeError::Failed
            }
        })?;

    if !output.status.success() {
        return Err(VideoProbeError::Failed);
    }

    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|_| VideoProbeError::Failed)?;
    let stream = value
        .get("streams")
        .and_then(|streams| streams.as_array())
        .and_then(|streams| streams.first());

    let width = stream
        .and_then(|stream| stream.get("width"))
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok());
    let height = stream
        .and_then(|stream| stream.get("height"))
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok());
    let fps = stream
        .and_then(|stream| stream.get("avg_frame_rate"))
        .and_then(|value| value.as_str())
        .and_then(parse_ratio);
    let duration_seconds = value
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse::<f32>().ok())
        .map(round_seconds);

    Ok(DatasetVideoSummary {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        relative_path: path
            .strip_prefix(dataset_path)
            .unwrap_or(path)
            .display()
            .to_string(),
        duration_seconds,
        width,
        height,
        fps,
    })
}

#[derive(Debug)]
enum VideoProbeError {
    MissingCommand,
    Failed,
}

fn summarize_video_resolutions(video_details: &[DatasetVideoSummary]) -> Vec<String> {
    let mut counts = BTreeMap::new();
    for detail in video_details {
        let (Some(width), Some(height)) = (detail.width, detail.height) else {
            continue;
        };
        *counts.entry(format!("{width}x{height}")).or_insert(0usize) += 1;
    }

    counts
        .into_iter()
        .map(|(resolution, count)| format!("{resolution} x{count}"))
        .collect()
}

fn parse_ratio(value: &str) -> Option<f32> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "0/0" {
        return None;
    }

    if let Some((numerator, denominator)) = trimmed.split_once('/') {
        let numerator = numerator.parse::<f32>().ok()?;
        let denominator = denominator.parse::<f32>().ok()?;
        if denominator <= 0.0 {
            return None;
        }
        return Some(round_seconds(numerator / denominator));
    }

    trimmed.parse::<f32>().ok().map(round_seconds)
}

fn round_seconds(value: f32) -> f32 {
    (value * 100.0).round() / 100.0
}

fn is_caption_file(path: &Path) -> bool {
    matches!(extension(path).as_deref(), Some("txt" | "caption" | "md"))
}

fn scan_runtime(paths: &ProjectPaths) -> RuntimeSummary {
    let llama_cli_ready = paths.runtime.join("llama-cli.exe").exists();
    let vulkan_runtime_ready = paths.runtime.join("ggml-vulkan.dll").exists();
    let diffusion_runtime_present = paths.root.join("diffuse_runtime").exists();

    let mut notes = Vec::new();
    if llama_cli_ready && vulkan_runtime_ready {
        notes.push(
            "Bundled llama Vulkan runtime looks ready for the future helper sidecar.".to_string(),
        );
    } else {
        notes.push(
            "Bundled llama Vulkan runtime looks incomplete. Check runtime/ if local inference is expected."
                .to_string(),
        );
    }

    if diffusion_runtime_present {
        notes.push(
            "A diffusion runtime folder is present, but this LoRA builder chassis is not using it yet."
                .to_string(),
        );
    } else {
        notes.push(
            "No diffusion runtime is installed in this sister tool yet. That is fine for the current dashboard phase."
                .to_string(),
        );
    }

    RuntimeSummary {
        llama_cli_ready,
        vulkan_runtime_ready,
        diffusion_runtime_present,
        notes,
    }
}

fn classify_media_kind(path: &Path) -> String {
    match extension(path).as_deref() {
        Some("png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif") => "Image".to_string(),
        Some("wav" | "mp3" | "flac" | "ogg" | "m4a") => "Audio".to_string(),
        Some("mp4" | "avi" | "mov" | "mkv" | "webm") => "Video".to_string(),
        Some("txt" | "md" | "csv" | "json" | "caption") => "Text".to_string(),
        _ => "Other".to_string(),
    }
}

#[derive(serde::Deserialize)]
struct DatasetManifestRecord {
    dataset_name: String,
    created_unix_seconds: u64,
    items: Vec<DatasetManifestItemRecord>,
}

#[derive(serde::Deserialize)]
struct DatasetManifestItemRecord {
    source_label: String,
}

fn classify_model_kind(path: &Path) -> String {
    match extension(path).as_deref() {
        Some("gguf") => "GGUF".to_string(),
        Some("safetensors") => "Safetensors".to_string(),
        Some("ckpt" | "pt" | "pth" | "bin") => "Checkpoint".to_string(),
        _ => "Other".to_string(),
    }
}

fn classify_model_family(
    relative_path: &str,
) -> (&'static str, &'static str, &'static str, &'static str) {
    let normalized = relative_path.replace('\\', "/");
    let first_segment = normalized.split('/').next().unwrap_or_default();
    if let Some(family) = model_registry::MODEL_FAMILIES.iter().find(|family| {
        family
            .relative_root
            .strip_prefix("models/")
            .map(|segment| segment == first_segment)
            .unwrap_or(false)
    }) {
        return (
            family.id,
            family.label,
            family.purpose,
            family.relative_root,
        );
    }

    (
        "unsorted",
        "Unsorted",
        "Model files that are not in a known family bucket yet.",
        "models/",
    )
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}
