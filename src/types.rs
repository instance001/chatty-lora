use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct DashboardResponse {
    pub app_name: &'static str,
    pub project_root: String,
    pub materials: MaterialPanel,
    pub builder: BuilderPanel,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemTelemetrySnapshot {
    pub supported: bool,
    pub label: String,
    pub note: String,
    pub cpu_label: String,
    pub gpu_label: String,
    pub current_cpu_percent: f32,
    pub current_gpu_percent: f32,
    pub cpu_history: Vec<f32>,
    pub gpu_history: Vec<f32>,
}

#[derive(Debug, Deserialize)]
pub struct HelperQueryRequest {
    pub page: String,
    pub question: String,
    pub materials: Option<MaterialsHelperContext>,
    pub builder: Option<BuilderHelperContext>,
}

#[derive(Debug, Deserialize)]
pub struct MaterialsHelperContext {
    pub search_query: String,
    #[serde(default)]
    pub media_kinds: Vec<String>,
    pub enabled_source_names: Vec<String>,
    pub selected_preview_count: usize,
    pub preview_batch_loaded: bool,
    pub input_file_count: usize,
    pub output_file_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct BuilderHelperContext {
    pub selected_dataset_slug: Option<String>,
    pub selected_dataset_file_count: Option<usize>,
    pub selected_dataset_image_count: Option<usize>,
    pub selected_dataset_audio_count: Option<usize>,
    pub selected_dataset_video_count: Option<usize>,
    pub prepared_project_count: usize,
    pub project_name: String,
    pub base_model: String,
    pub training_backend_id: String,
    pub concept_type: String,
    pub training_preset: String,
    pub caption_strategy: String,
    pub rank: Option<u32>,
    pub repeats: Option<u32>,
    pub epochs: Option<u32>,
    pub resolution: Option<u32>,
    pub batch_size: Option<u32>,
    pub learning_rate: Option<f32>,
    pub validation_split_percent: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct HelperQueryResponse {
    pub page: String,
    pub answer: String,
    pub context_title: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MaterialPanel {
    pub input_summary: FolderSummary,
    pub output_summary: FolderSummary,
    pub input_files: Vec<LibraryItem>,
    pub output_files: Vec<LibraryItem>,
    pub model_summary: ModelSummary,
    pub runtime_summary: RuntimeSummary,
    pub source_registry: SourceRegistryPayload,
    pub site_fix_summaries: Vec<SourceFixSummary>,
}

#[derive(Debug, Serialize)]
pub struct BuilderPanel {
    pub project_name_suggestion: String,
    pub base_model_options: Vec<String>,
    pub training_backends: Vec<TrainingBackendSummary>,
    pub recommended_training_backend_id: Option<String>,
    pub wan_training: WanTrainingStatus,
    pub recommended_dataset_slug: Option<String>,
    pub curated_datasets: Vec<DatasetSummary>,
    pub prepared_projects: Vec<PreparedProjectSummary>,
    pub status_lines: Vec<String>,
    pub starter_notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DatasetSummary {
    pub slug: String,
    pub display_name: String,
    pub relative_path: String,
    pub total_files: usize,
    pub images: usize,
    pub audio: usize,
    pub video: usize,
    pub other: usize,
    pub source_count: usize,
    pub manifest_present: bool,
    pub created_unix_seconds: Option<u64>,
    pub preflight: DatasetPreflightSummary,
}

#[derive(Debug, Serialize)]
pub struct DatasetPreflightSummary {
    pub status: String,
    pub label: String,
    pub badge: String,
    pub notes: Vec<String>,
    pub caption_files: usize,
    pub video_probe_available: bool,
    pub probed_video_count: usize,
    pub total_duration_seconds: Option<f32>,
    pub min_duration_seconds: Option<f32>,
    pub max_duration_seconds: Option<f32>,
    pub resolution_summary: Vec<String>,
    pub video_details: Vec<DatasetVideoSummary>,
}

#[derive(Debug, Serialize)]
pub struct DatasetVideoSummary {
    pub name: String,
    pub relative_path: String,
    pub duration_seconds: Option<f32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f32>,
}

#[derive(Debug, Serialize)]
pub struct PreparedProjectSummary {
    pub slug: String,
    pub project_name: String,
    pub relative_path: String,
    pub generated_training_relative_path: Option<String>,
    pub generated_training_ready: bool,
    pub generated_training_notes: Vec<String>,
    pub generated_training_commands: Vec<PreparedProjectRunCommand>,
    pub trained_outputs: Vec<PreparedProjectOutputSummary>,
    pub video_rows: Option<usize>,
    pub image_rows: Option<usize>,
    pub dataset_slug: String,
    pub base_model: String,
    pub training_backend_id: String,
    pub trigger_phrase: String,
    pub concept_summary: String,
    pub concept_type: String,
    pub training_preset: String,
    pub caption_strategy: String,
    pub resolution: u32,
    pub rank: u32,
    pub repeats: u32,
    pub epochs: u32,
    pub batch_size: u32,
    pub learning_rate: f32,
    pub validation_split_percent: u32,
    pub created_unix_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct PreparedProjectRunCommand {
    pub label: String,
    pub command: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct PreparedProjectOutputSummary {
    pub relative_path: String,
    pub bytes: u64,
    pub modified_unix_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct TrainingBackendSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub best_for: String,
    pub ready: bool,
    pub relative_path: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct WanTrainingStatus {
    pub id: String,
    pub label: String,
    pub ready: bool,
    pub model_bundle_ready: bool,
    pub trainer_ready: bool,
    pub wsl_ready: bool,
    pub wsl_distro: String,
    pub wsl_musubi_root: String,
    pub selected_dit_relative_path: Option<String>,
    pub recommended_defaults: WanTrainingDefaults,
    pub files: Vec<WanModelFileStatus>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct WanTrainingDefaults {
    pub resolution: u32,
    pub target_frames: u32,
    pub source_fps: f32,
    pub batch_size: u32,
    pub rank: u32,
    pub epochs: u32,
    pub learning_rate: f32,
}

#[derive(Debug, Serialize, Clone)]
pub struct WanModelFileStatus {
    pub label: String,
    pub role: String,
    pub relative_path: String,
    pub present: bool,
    pub required: bool,
    pub bytes: Option<u64>,
}

#[derive(Debug, Serialize, Default)]
pub struct FolderSummary {
    pub total: usize,
    pub images: usize,
    pub audio: usize,
    pub video: usize,
    pub text: usize,
    pub other: usize,
}

#[derive(Debug, Serialize)]
pub struct ModelSummary {
    pub total: usize,
    pub gguf: usize,
    pub safetensors: usize,
    pub checkpoints: usize,
    pub other: usize,
    pub items: Vec<ModelItem>,
}

#[derive(Debug, Serialize)]
pub struct RuntimeSummary {
    pub llama_cli_ready: bool,
    pub vulkan_runtime_ready: bool,
    pub diffusion_runtime_present: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct LibraryItem {
    pub name: String,
    pub relative_path: String,
    pub kind: String,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct ModelItem {
    pub name: String,
    pub kind: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEntry {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub adapter_kind: String,
    pub media_kind: String,
    pub enabled: bool,
    pub user_added: bool,
    pub crawl_delay_ms: u64,
    pub pages_per_batch: u32,
    pub respect_robots_txt: bool,
    pub notes: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site_profile: Option<GenericGalleryProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericGalleryProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_selector: Option<String>,
    pub media_selector: String,
    pub media_attribute: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_attribute: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_attribute: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_attribute: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SourceRegistryPayload {
    pub total: usize,
    pub enabled: usize,
    pub custom: usize,
    pub search_ready: usize,
    pub sources: Vec<SourceEntry>,
}

#[derive(Debug, Serialize)]
pub struct SourceFixSummary {
    pub source_id: String,
    pub source_name: String,
    pub adapter_kind: String,
    pub adapter_ready: bool,
    pub adapter_file_path: String,
    pub note_relative_path: String,
    pub note_present: bool,
}

#[derive(Debug, Deserialize)]
pub struct SourceRegistryUpdateRequest {
    pub sources: Vec<SourceEntry>,
}

#[derive(Debug, Deserialize)]
pub struct SearchPreviewRequest {
    pub query: String,
    pub selected_source_ids: Vec<String>,
    #[serde(default)]
    pub media_kinds: Vec<String>,
    pub batch_index: u32,
}

#[derive(Debug, Deserialize)]
pub struct DatasetCreateRequest {
    pub dataset_name: String,
    pub selected_items: Vec<PreviewItem>,
}

#[derive(Debug, Deserialize)]
pub struct LocalDatasetImportRequest {
    pub source_folder: String,
    pub dataset_name: String,
}

#[derive(Debug, Serialize)]
pub struct DatasetCreateResponse {
    pub dataset_slug: String,
    pub dataset_path: String,
    pub manifest_path: String,
    pub saved_items: usize,
    pub failed_items: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SourceFixOpenRequest {
    pub source_id: String,
}

#[derive(Debug, Serialize)]
pub struct SourceFixOpenResponse {
    pub source_id: String,
    pub source_name: String,
    pub adapter_kind: String,
    pub adapter_ready: bool,
    pub adapter_file_path: String,
    pub note_relative_path: String,
    pub existing_note: String,
    pub scope_note: String,
    pub starter_steps: Vec<String>,
    pub proposal_history: Vec<SourceFixProposalHistoryItem>,
    pub apply_history: Vec<SourceFixAppliedHistoryItem>,
}

#[derive(Debug, Deserialize)]
pub struct SourceFixSaveRequest {
    pub source_id: String,
    pub issue_summary: String,
    pub reproduction_notes: String,
    pub patch_notes: String,
}

#[derive(Debug, Serialize)]
pub struct SourceFixSaveResponse {
    pub source_id: String,
    pub saved_relative_path: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SourceFixProposeRequest {
    pub source_id: String,
    pub issue_summary: String,
    pub reproduction_notes: String,
    pub patch_notes: String,
}

#[derive(Debug, Serialize)]
pub struct SourceFixProposalResponse {
    pub source_id: String,
    pub source_name: String,
    pub adapter_file_path: String,
    pub proposal_title: String,
    pub confidence_label: String,
    pub analysis_points: Vec<String>,
    pub proposed_patch: String,
    pub review_checklist: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SourceFixProposalHistoryItem {
    pub title: String,
    pub relative_path: String,
    pub saved_unix_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct SourceFixAppliedHistoryItem {
    pub title: String,
    pub relative_path: String,
    pub saved_unix_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct SourceFixProposalSaveRequest {
    pub source_id: String,
    pub proposal_title: String,
    pub confidence_label: String,
    pub analysis_points: Vec<String>,
    pub proposed_patch: String,
    pub review_checklist: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SourceFixProposalSaveResponse {
    pub source_id: String,
    pub saved_relative_path: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SourceFixApplyPreviewRequest {
    pub source_id: String,
    pub issue_summary: String,
    pub reproduction_notes: String,
    pub patch_notes: String,
}

#[derive(Debug, Serialize)]
pub struct SourceFixApplyPreviewResponse {
    pub source_id: String,
    pub source_name: String,
    pub adapter_file_path: String,
    pub backup_relative_path: String,
    pub review_title: String,
    pub apply_notes: Vec<String>,
    pub diff_lines: Vec<String>,
    pub before_excerpt: String,
    pub after_excerpt: String,
}

#[derive(Debug, Deserialize)]
pub struct SourceFixApplyRequest {
    pub source_id: String,
    pub issue_summary: String,
    pub reproduction_notes: String,
    pub patch_notes: String,
}

#[derive(Debug, Serialize)]
pub struct SourceFixApplyResponse {
    pub source_id: String,
    pub source_name: String,
    pub adapter_file_path: String,
    pub applied_relative_path: String,
    pub backup_relative_path: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct BuilderPrepareRequest {
    pub project_name: String,
    pub dataset_slug: String,
    pub base_model: String,
    pub training_backend_id: String,
    pub trigger_phrase: String,
    pub concept_summary: String,
    pub concept_type: String,
    pub training_preset: String,
    pub caption_strategy: String,
    pub rank: u32,
    pub repeats: u32,
    pub epochs: u32,
    pub resolution: u32,
    pub batch_size: u32,
    pub learning_rate: f32,
    pub validation_split_percent: u32,
}

#[derive(Debug, Serialize)]
pub struct BuilderPrepareResponse {
    pub project_slug: String,
    pub project_path: String,
    pub generated_training_path: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct BuilderDeleteProjectRequest {
    pub project_slug: String,
}

#[derive(Debug, Serialize)]
pub struct BuilderDeleteProjectResponse {
    pub project_slug: String,
    pub removed_paths: Vec<String>,
    pub preserved_paths: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TrainingRunRequest {
    pub project_slug: String,
    #[serde(default = "default_training_run_mode")]
    pub mode: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenLocalPathRequest {
    pub relative_path: String,
}

#[derive(Debug, Serialize)]
pub struct OpenLocalPathResponse {
    pub opened_path: String,
}

fn default_training_run_mode() -> String {
    "full".to_string()
}

#[derive(Debug, Serialize, Clone)]
pub struct TrainingRunStatus {
    pub job_id: u64,
    pub state: String,
    pub project_slug: Option<String>,
    pub mode: Option<String>,
    pub current_stage: Option<String>,
    pub message: String,
    pub started_unix_seconds: Option<u64>,
    pub ended_unix_seconds: Option<u64>,
    pub process_id: Option<u32>,
    pub stages: Vec<TrainingStageStatus>,
    pub logs: Vec<TrainingLogLine>,
    pub output_files: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TrainingStageStatus {
    pub id: String,
    pub label: String,
    pub state: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TrainingLogLine {
    pub unix_seconds: u64,
    pub stage_id: String,
    pub stream: String,
    pub line: String,
}

#[derive(Debug, Serialize)]
pub struct SearchPreviewResponse {
    pub query: String,
    pub batch_index: u32,
    pub page_window_start: u32,
    pub page_window_end: u32,
    pub source_batches: Vec<SourcePreviewBatch>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SourcePreviewBatch {
    pub source_id: String,
    pub source_name: String,
    pub media_kind: String,
    pub note: String,
    pub has_more: bool,
    pub pages: Vec<PagePreviewGroup>,
}

#[derive(Debug, Serialize)]
pub struct PagePreviewGroup {
    pub page_number: u32,
    pub items: Vec<PreviewItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewItem {
    pub key: String,
    pub title: String,
    pub thumb_url: Option<String>,
    pub preview_url: Option<String>,
    pub media_url: String,
    pub source_page_url: String,
    pub license: Option<String>,
    pub creator: Option<String>,
    pub source_label: String,
    pub page_number: u32,
    pub kind: String,
}
