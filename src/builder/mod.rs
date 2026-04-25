use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;
use walkdir::WalkDir;

use crate::{
    lane_registry,
    state::ProjectPaths,
    training,
    types::{
        BuilderConceptBlock, BuilderDeleteProjectRequest, BuilderDeleteProjectResponse,
        BuilderPrepareRequest, BuilderPrepareResponse, PreparedProjectOutputSummary,
        PreparedProjectRunCommand, PreparedProjectSummary,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectConceptBlock {
    #[serde(default = "default_project_concept_role")]
    role: String,
    concept_type: String,
    trigger_phrase: String,
    concept_summary: String,
    #[serde(default)]
    training_intent: String,
}

fn default_project_concept_role() -> String {
    "primary".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectSpec {
    project_name: String,
    project_slug: String,
    dataset_slug: String,
    dataset_path: String,
    base_model: String,
    #[serde(default = "default_training_backend_id")]
    training_backend_id: String,
    #[serde(default)]
    backend_selection_manually_overridden: bool,
    trigger_phrase: String,
    concept_summary: String,
    concept_type: String,
    #[serde(default)]
    concept_blocks: Vec<ProjectConceptBlock>,
    training_preset: String,
    caption_strategy: String,
    rank: u32,
    repeats: u32,
    epochs: u32,
    resolution: u32,
    batch_size: u32,
    learning_rate: f32,
    validation_split_percent: u32,
    #[serde(default)]
    generated_training_path: Option<String>,
    #[serde(default)]
    wan_task: Option<String>,
    #[serde(default)]
    target_frames: Option<u32>,
    #[serde(default)]
    source_fps: Option<f32>,
    #[serde(default)]
    frame_extraction: Option<String>,
    #[serde(default)]
    musubi_wsl_distro: Option<String>,
    #[serde(default)]
    musubi_wsl_root: Option<String>,
    created_unix_seconds: u64,
}

pub fn prepare_project(
    paths: &ProjectPaths,
    request: BuilderPrepareRequest,
) -> Result<BuilderPrepareResponse> {
    let project_name = request.project_name.trim();
    if project_name.is_empty() {
        bail!("Project name cannot be blank.");
    }

    if request.dataset_slug.trim().is_empty() {
        bail!("Pick a curated dataset before preparing a project.");
    }

    if request.base_model.trim().is_empty() {
        bail!("Choose a base model before preparing a project.");
    }

    if request.training_backend_id.trim().is_empty() {
        bail!("Choose a training backend target before saving the training plan.");
    }

    if !training::is_known_backend(request.training_backend_id.trim()) {
        bail!("The selected training backend is not recognized by this build.");
    }

    let dataset_path = paths.inputs.join(request.dataset_slug.trim());
    if !dataset_path.exists() {
        bail!("The selected curated dataset could not be found on disk anymore.");
    }

    std::fs::create_dir_all(&paths.project_specs)
        .with_context(|| format!("could not create {}", paths.project_specs.display()))?;

    let project_slug = next_available_project_slug(&paths.project_specs, project_name);
    let project_path = paths.project_specs.join(format!("{}.json", project_slug));
    let created_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let concept_blocks = normalize_concept_blocks(
        request.concept_blocks,
        request.concept_type.trim(),
        request.trigger_phrase.trim(),
        request.concept_summary.trim(),
    );
    let primary_concept = primary_concept_block(
        &concept_blocks,
        request.concept_type.trim(),
        request.trigger_phrase.trim(),
        request.concept_summary.trim(),
    );

    let mut spec = ProjectSpec {
        project_name: project_name.to_string(),
        project_slug: project_slug.clone(),
        dataset_slug: request.dataset_slug.trim().to_string(),
        dataset_path: dataset_path.display().to_string(),
        base_model: request.base_model.trim().to_string(),
        training_backend_id: request.training_backend_id.trim().to_string(),
        backend_selection_manually_overridden: request.backend_selection_manually_overridden,
        trigger_phrase: primary_concept.trigger_phrase.clone(),
        concept_summary: primary_concept.concept_summary.clone(),
        concept_type: primary_concept.concept_type.clone(),
        concept_blocks,
        training_preset: request.training_preset.trim().to_string(),
        caption_strategy: request.caption_strategy.trim().to_string(),
        rank: request.rank,
        repeats: request.repeats,
        epochs: request.epochs,
        resolution: request.resolution,
        batch_size: request.batch_size,
        learning_rate: request.learning_rate,
        validation_split_percent: request.validation_split_percent,
        generated_training_path: None,
        wan_task: None,
        target_frames: None,
        source_fps: None,
        frame_extraction: None,
        musubi_wsl_distro: None,
        musubi_wsl_root: None,
        created_unix_seconds,
    };

    let mut notes = build_prepare_notes(&spec);
    if training::is_musubi_wan_backend(&spec.training_backend_id) {
        let (generated_path, media_count, media_label) =
            generate_wan_training_handoff(paths, &dataset_path, &mut spec)?;
        notes.push(format!(
            "Generated Wan/Musubi handoff files at {}.",
            generated_path
        ));
        if media_count == 0 {
            notes.push(format!(
                "Warning: this dataset currently has no {} files, so the generated Wan metadata file is empty until material is added.",
                media_label
            ));
        } else {
            notes.push(format!(
                "Mapped {} {}{} into the Wan metadata JSONL.",
                media_count,
                media_label,
                if media_count == 1 { "" } else { "s" }
            ));
        }
    }

    let json = serde_json::to_string_pretty(&spec).context("could not serialize project spec")?;
    std::fs::write(&project_path, json)
        .with_context(|| format!("could not write {}", project_path.display()))?;

    Ok(BuilderPrepareResponse {
        project_slug,
        project_path: project_path.display().to_string(),
        generated_training_path: spec.generated_training_path.clone(),
        notes,
    })
}

fn normalize_concept_blocks(
    blocks: Vec<BuilderConceptBlock>,
    legacy_concept_type: &str,
    legacy_trigger_phrase: &str,
    legacy_concept_summary: &str,
) -> Vec<ProjectConceptBlock> {
    let mut normalized = blocks
        .into_iter()
        .map(|block| ProjectConceptBlock {
            role: normalize_concept_role(&block.role),
            concept_type: block.concept_type.trim().to_string(),
            trigger_phrase: block.trigger_phrase.trim().to_string(),
            concept_summary: block.concept_summary.trim().to_string(),
            training_intent: block.training_intent.trim().to_string(),
        })
        .filter(|block| {
            !block.concept_type.is_empty()
                || !block.trigger_phrase.is_empty()
                || !block.concept_summary.is_empty()
                || !block.training_intent.is_empty()
        })
        .collect::<Vec<_>>();

    if normalized.is_empty()
        && (!legacy_concept_type.is_empty()
            || !legacy_trigger_phrase.is_empty()
            || !legacy_concept_summary.is_empty())
    {
        normalized.push(ProjectConceptBlock {
            role: "primary".to_string(),
            concept_type: legacy_concept_type.to_string(),
            trigger_phrase: legacy_trigger_phrase.to_string(),
            concept_summary: legacy_concept_summary.to_string(),
            training_intent: String::new(),
        });
    }

    normalized
}

fn effective_concept_blocks(spec: &ProjectSpec) -> Vec<ProjectConceptBlock> {
    let blocks = normalize_concept_blocks(
        spec.concept_blocks
            .iter()
            .cloned()
            .map(|block| BuilderConceptBlock {
                role: block.role,
                concept_type: block.concept_type,
                trigger_phrase: block.trigger_phrase,
                concept_summary: block.concept_summary,
                training_intent: block.training_intent,
            })
            .collect(),
        spec.concept_type.trim(),
        spec.trigger_phrase.trim(),
        spec.concept_summary.trim(),
    );
    if blocks.is_empty() {
        vec![ProjectConceptBlock {
            role: "primary".to_string(),
            concept_type: "style".to_string(),
            trigger_phrase: String::new(),
            concept_summary: String::new(),
            training_intent: String::new(),
        }]
    } else {
        blocks
    }
}

fn primary_concept_block(
    blocks: &[ProjectConceptBlock],
    legacy_concept_type: &str,
    legacy_trigger_phrase: &str,
    legacy_concept_summary: &str,
) -> ProjectConceptBlock {
    blocks
        .iter()
        .find(|block| block.role == "primary")
        .cloned()
        .or_else(|| blocks.first().cloned())
        .unwrap_or_else(|| ProjectConceptBlock {
            concept_type: if legacy_concept_type.is_empty() {
                "style".to_string()
            } else {
                legacy_concept_type.to_string()
            },
            role: "primary".to_string(),
            trigger_phrase: legacy_trigger_phrase.to_string(),
            concept_summary: legacy_concept_summary.to_string(),
            training_intent: String::new(),
        })
}

fn concept_blocks_for_summary(spec: &ProjectSpec) -> Vec<BuilderConceptBlock> {
    effective_concept_blocks(spec)
        .into_iter()
        .map(|block| BuilderConceptBlock {
            role: block.role,
            concept_type: block.concept_type,
            trigger_phrase: block.trigger_phrase,
            concept_summary: block.concept_summary,
            training_intent: block.training_intent,
        })
        .collect()
}

fn normalize_concept_role(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "primary" => "primary".to_string(),
        "supporting" => "supporting".to_string(),
        "avoid" => "avoid".to_string(),
        _ => "primary".to_string(),
    }
}

pub fn scan_project_specs(paths: &ProjectPaths) -> Result<Vec<PreparedProjectSummary>> {
    if !paths.project_specs.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();
    for entry in WalkDir::new(&paths.project_specs)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
    {
        let path = entry.path();
        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(_) => continue,
        };
        let spec = match serde_json::from_str::<ProjectSpec>(&contents) {
            Ok(spec) => spec,
            Err(_) => continue,
        };

        let generated_training_relative_path = spec.generated_training_path.clone();
        let handoff = build_project_handoff_summary(paths, &spec);

        let concept_blocks = concept_blocks_for_summary(&spec);

        projects.push(PreparedProjectSummary {
            trained_outputs: collect_trained_outputs(paths, &spec.project_slug),
            slug: spec.project_slug,
            project_name: spec.project_name,
            relative_path: path
                .strip_prefix(&paths.root)
                .unwrap_or(path)
                .display()
                .to_string(),
            generated_training_relative_path,
            generated_training_ready: handoff.ready,
            generated_training_notes: handoff.notes,
            generated_training_commands: handoff.commands,
            dataset_slug: spec.dataset_slug,
            base_model: spec.base_model,
            training_backend_id: spec.training_backend_id,
            backend_selection_manually_overridden: spec.backend_selection_manually_overridden,
            trigger_phrase: spec.trigger_phrase,
            concept_summary: spec.concept_summary,
            concept_type: spec.concept_type,
            concept_blocks,
            training_preset: spec.training_preset,
            caption_strategy: spec.caption_strategy,
            resolution: spec.resolution,
            rank: spec.rank,
            repeats: spec.repeats,
            epochs: spec.epochs,
            batch_size: spec.batch_size,
            learning_rate: spec.learning_rate,
            validation_split_percent: spec.validation_split_percent,
            video_rows: handoff.video_rows,
            image_rows: handoff.image_rows,
            created_unix_seconds: Some(spec.created_unix_seconds),
        });
    }

    projects.sort_by(|left, right| {
        right
            .created_unix_seconds
            .unwrap_or(0)
            .cmp(&left.created_unix_seconds.unwrap_or(0))
            .then_with(|| left.slug.cmp(&right.slug))
    });
    Ok(projects)
}

pub fn delete_project(
    paths: &ProjectPaths,
    request: BuilderDeleteProjectRequest,
) -> Result<BuilderDeleteProjectResponse> {
    let project_slug = request.project_slug.trim();
    validate_project_slug(project_slug)?;

    let project_path = paths.project_specs.join(format!("{project_slug}.json"));
    if !project_path.exists() {
        bail!("That saved training plan no longer exists.");
    }

    let contents = std::fs::read_to_string(&project_path)
        .with_context(|| format!("could not read {}", project_path.display()))?;
    let spec = serde_json::from_str::<ProjectSpec>(&contents)
        .context("saved training plan JSON could not be read safely")?;
    if spec.project_slug != project_slug {
        bail!("Saved plan slug mismatch; delete was stopped for safety.");
    }

    let mut removed_paths = Vec::new();
    std::fs::remove_file(&project_path)
        .with_context(|| format!("could not delete {}", project_path.display()))?;
    removed_paths.push(relative_path(&project_path, &paths.root));

    let generated_dir = paths.training_generated.join(project_slug);
    if generated_dir.exists() {
        std::fs::remove_dir_all(&generated_dir)
            .with_context(|| format!("could not delete {}", generated_dir.display()))?;
        removed_paths.push(relative_path(&generated_dir, &paths.root));
    }

    let output_dir = paths.training_outputs.join(project_slug);
    let preserved_paths = if output_dir.exists() {
        vec![relative_path(&output_dir, &paths.root)]
    } else {
        Vec::new()
    };

    let mut notes = vec![
        format!("Deleted saved training plan \"{}\".", spec.project_name),
        "Removed the saved plan and any generated handoff folder only.".to_string(),
    ];
    if preserved_paths.is_empty() {
        notes.push("No trained output folder was present for this plan.".to_string());
    } else {
        notes.push("Trained LoRA outputs were preserved. Delete them manually from outputs/training if you no longer need them.".to_string());
    }

    Ok(BuilderDeleteProjectResponse {
        project_slug: project_slug.to_string(),
        removed_paths,
        preserved_paths,
        notes,
    })
}

fn validate_project_slug(project_slug: &str) -> Result<()> {
    if project_slug.is_empty()
        || project_slug.contains("..")
        || project_slug.contains('/')
        || project_slug.contains('\\')
        || !project_slug
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        bail!("Choose a valid saved training plan first.");
    }
    Ok(())
}

fn relative_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn collect_trained_outputs(
    paths: &ProjectPaths,
    project_slug: &str,
) -> Vec<PreparedProjectOutputSummary> {
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
        .filter_map(|path| {
            let metadata = std::fs::metadata(&path).ok()?;
            Some(PreparedProjectOutputSummary {
                relative_path: path
                    .strip_prefix(&paths.root)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
                bytes: metadata.len(),
                modified_unix_seconds: metadata.modified().ok().and_then(system_time_to_unix),
            })
        })
        .collect::<Vec<_>>();

    outputs.sort_by(|left, right| {
        right
            .modified_unix_seconds
            .unwrap_or(0)
            .cmp(&left.modified_unix_seconds.unwrap_or(0))
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    outputs
}

fn system_time_to_unix(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

#[derive(Debug, Default)]
struct ProjectHandoffSummary {
    ready: bool,
    notes: Vec<String>,
    commands: Vec<PreparedProjectRunCommand>,
    video_rows: Option<usize>,
    image_rows: Option<usize>,
}

fn build_project_handoff_summary(
    paths: &ProjectPaths,
    spec: &ProjectSpec,
) -> ProjectHandoffSummary {
    if !training::is_musubi_wan_backend(&spec.training_backend_id) {
        return ProjectHandoffSummary::default();
    }

    let Some(relative_path) = spec.generated_training_path.as_deref() else {
        return ProjectHandoffSummary {
            notes: vec![
                "No generated Musubi handoff folder is attached to this saved plan yet."
                    .to_string(),
            ],
            ..ProjectHandoffSummary::default()
        };
    };

    let generated_dir = paths.root.join(relative_path);
    let scripts = [
        ("preflight.sh", "Preflight"),
        ("cache_latents.sh", "Cache latents"),
        ("cache_text.sh", "Cache text"),
        ("launch.sh", "Launch training"),
        ("run_all.sh", "Run all"),
    ];

    let mut notes = Vec::new();
    if generated_dir.exists() {
        notes.push(format!("Handoff folder found at {}.", relative_path));
    } else {
        notes.push(format!(
            "Handoff folder was saved as {}, but it is not on disk right now.",
            relative_path
        ));
    }
    notes.push(format!(
        "Backend selection for this saved handoff: {}.",
        backend_selection_mode_label(spec.backend_selection_manually_overridden)
    ));

    let missing_scripts = scripts
        .iter()
        .filter(|(file, _label)| !generated_dir.join(file).exists())
        .map(|(file, _label)| *file)
        .collect::<Vec<_>>();
    if missing_scripts.is_empty() {
        notes.push("All expected Musubi shell scripts are present.".to_string());
    } else {
        notes.push(format!(
            "Missing generated script{}: {}.",
            if missing_scripts.len() == 1 { "" } else { "s" },
            missing_scripts.join(", ")
        ));
    }

    let dataset_kind = WanDatasetKind::from_backend_id(&spec.training_backend_id);
    let row_count = count_jsonl_rows(&generated_dir.join(dataset_kind.metadata_file_name()));
    match row_count {
        Some(0) => notes.push(format!(
            "No {} rows are mapped yet. This plan can be inspected, but training will not be useful until the dataset contains {} files.",
            dataset_kind.media_label(),
            dataset_kind.media_label()
        )),
        Some(count) => notes.push(format!(
            "{} {} row{} mapped into {}.",
            count,
            dataset_kind.media_label(),
            if count == 1 { "" } else { "s" }
            ,
            dataset_kind.metadata_file_name()
        )),
        None => notes.push(format!(
            "{} is missing or could not be read.",
            dataset_kind.metadata_file_name()
        )),
    }

    let ready = generated_dir.exists() && missing_scripts.is_empty() && row_count.unwrap_or(0) > 0;

    let mut commands = Vec::new();
    if generated_dir.exists() && missing_scripts.is_empty() {
        let preflight = sh_quote(&windows_path_to_wsl(&generated_dir.join("preflight.sh")));
        let cache_latents = sh_quote(&windows_path_to_wsl(
            &generated_dir.join("cache_latents.sh"),
        ));
        let cache_text = sh_quote(&windows_path_to_wsl(&generated_dir.join("cache_text.sh")));
        let launch = sh_quote(&windows_path_to_wsl(&generated_dir.join("launch.sh")));
        let run_all = sh_quote(&windows_path_to_wsl(&generated_dir.join("run_all.sh")));

        commands.push(PreparedProjectRunCommand {
            label: "0. Preflight".to_string(),
            command: wsl_command(&format!("bash {preflight}")),
            description: format!(
                "Checks generated files, model paths, Musubi, PyTorch ROCm GPU access, {}metadata, and shell syntax without training.",
                if dataset_kind.is_video() {
                    "ffmpeg/ffprobe, "
                } else {
                    ""
                }
            ),
        });
        commands.push(PreparedProjectRunCommand {
            label: "1. Cache latents".to_string(),
            command: wsl_command(&format!("bash {cache_latents}")),
            description: format!(
                "Prepares {} latent cache files.",
                dataset_kind.media_label()
            ),
        });
        commands.push(PreparedProjectRunCommand {
            label: "2. Cache text".to_string(),
            command: wsl_command(&format!("bash {cache_text}")),
            description: "Prepares text encoder cache files.".to_string(),
        });
        commands.push(PreparedProjectRunCommand {
            label: "3. Train LoRA".to_string(),
            command: wsl_command(&format!("bash {launch}")),
            description: "Starts the actual Musubi training run.".to_string(),
        });
        commands.push(PreparedProjectRunCommand {
            label: "Run all steps".to_string(),
            command: wsl_command(&format!("bash {run_all}")),
            description: "Convenience command once the step-by-step flow is trusted.".to_string(),
        });
    }

    ProjectHandoffSummary {
        ready,
        notes,
        commands,
        video_rows: dataset_kind.is_video().then_some(row_count).flatten(),
        image_rows: dataset_kind.is_image().then_some(row_count).flatten(),
    }
}

fn count_jsonl_rows(path: &Path) -> Option<usize> {
    let contents = std::fs::read_to_string(path).ok()?;
    Some(
        contents
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
    )
}

fn wsl_command(command: &str) -> String {
    format!(
        "wsl -d {} -- bash -lc \"{}\"",
        training::WSL_DISTRO,
        command.replace('"', "\\\"")
    )
}

fn backend_selection_mode_label(manually_overridden: bool) -> &'static str {
    if manually_overridden {
        "manual choice"
    } else {
        "auto-suggested"
    }
}

fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn next_available_project_slug(folder: &Path, project_name: &str) -> String {
    let base_slug = slugify(project_name);
    let mut slug = base_slug.clone();
    let mut counter = 2usize;

    while folder.join(format!("{}.json", slug)).exists() {
        slug = format!("{}-{}", base_slug, counter);
        counter += 1;
    }

    slug
}

fn slugify(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_dash = false;

    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('-');
            last_was_dash = true;
        }
    }

    out.trim_matches('-').to_string()
}

fn default_training_backend_id() -> String {
    "kohya_ss".to_string()
}

fn build_prepare_notes(spec: &ProjectSpec) -> Vec<String> {
    let concept_blocks = effective_concept_blocks(spec);
    let concept_summary = concept_blocks
        .iter()
        .map(|block| {
            let label = match block.role.as_str() {
                "supporting" => format!("supporting {}", block.concept_type),
                "avoid" => format!("avoid {}", block.concept_type),
                _ => block.concept_type.clone(),
            };
            if block.training_intent.trim().is_empty() {
                label
            } else {
                format!("{} ({})", label, block.training_intent)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    vec![
        format!(
            "Prepared project '{}' using dataset '{}'.",
            spec.project_name, spec.dataset_slug
        ),
        format!(
            "Base model: {} | backend target: {} | preset: {}.",
            spec.base_model,
            training::backend_label(&spec.training_backend_id),
            spec.training_preset
        ),
        format!(
            "Concept: {} | training settings: rank {} | repeats {} | epochs {} | {}px | batch size {}.",
            concept_summary, spec.rank, spec.repeats, spec.epochs, spec.resolution, spec.batch_size
        ),
        format!(
            "Validation split: {}% | caption strategy: {}.",
            spec.validation_split_percent, spec.caption_strategy
        ),
    ]
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WanDatasetKind {
    Video,
    Image,
}

impl WanDatasetKind {
    fn from_backend_id(backend_id: &str) -> Self {
        match lane_registry::lane_definition(backend_id)
            .map(|lane| lane.dataset_kind)
            .unwrap_or(lane_registry::TrainingDatasetKind::Video)
        {
            lane_registry::TrainingDatasetKind::Image => Self::Image,
            lane_registry::TrainingDatasetKind::Video => Self::Video,
        }
    }

    fn is_video(self) -> bool {
        self == Self::Video
    }

    fn is_image(self) -> bool {
        self == Self::Image
    }

    fn media_label(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Image => "image",
        }
    }

    fn metadata_file_name(self) -> &'static str {
        match self {
            Self::Video => "video_metadata.jsonl",
            Self::Image => "image_metadata.jsonl",
        }
    }

    fn metadata_path_key(self) -> &'static str {
        match self {
            Self::Video => "video_path",
            Self::Image => "image_path",
        }
    }

    fn jsonl_toml_key(self) -> &'static str {
        match self {
            Self::Video => "video_jsonl_file",
            Self::Image => "image_jsonl_file",
        }
    }

    fn manifest_kind(self) -> &'static str {
        match self {
            Self::Video => "Video",
            Self::Image => "Image",
        }
    }

    fn matches_path(self, path: &Path) -> bool {
        match self {
            Self::Video => is_video_file(path),
            Self::Image => is_image_file(path),
        }
    }
}

fn generate_wan_training_handoff(
    paths: &ProjectPaths,
    dataset_path: &Path,
    spec: &mut ProjectSpec,
) -> Result<(String, usize, &'static str)> {
    let lane = lane_registry::lane_definition(&spec.training_backend_id)
        .context("could not resolve the selected training lane")?;
    let dataset_kind = WanDatasetKind::from_backend_id(&spec.training_backend_id);
    let backend_selection_mode =
        backend_selection_mode_label(spec.backend_selection_manually_overridden);
    let wan_status = training::scan_wan_training(paths);
    if !wan_status.model_bundle_ready {
        bail!("Wan 2.1 model files are not complete enough to generate a Musubi handoff yet.");
    }

    let wan_paths = training::selected_wan_paths(paths)
        .context("could not resolve the selected Wan model paths")?;

    std::fs::create_dir_all(&paths.training_config)
        .with_context(|| format!("could not create {}", paths.training_config.display()))?;
    std::fs::create_dir_all(&paths.training_generated)
        .with_context(|| format!("could not create {}", paths.training_generated.display()))?;
    std::fs::create_dir_all(&paths.training_outputs)
        .with_context(|| format!("could not create {}", paths.training_outputs.display()))?;

    let generated_dir = paths.training_generated.join(&spec.project_slug);
    let output_dir = paths.training_outputs.join(&spec.project_slug);
    let cache_dir = output_dir.join("cache");
    let folders = [
        generated_dir.clone(),
        output_dir.clone(),
        cache_dir.clone(),
        output_dir.join("logs"),
        output_dir.join("samples"),
        output_dir.join("loras"),
        output_dir.join("reports"),
    ];
    for folder in folders {
        std::fs::create_dir_all(&folder)
            .with_context(|| format!("could not create {}", folder.display()))?;
    }

    let target_frames = lane.defaults.target_frames;
    let source_fps = lane.defaults.source_fps;
    let frame_extraction = lane.defaults.frame_extraction.map(str::to_string);
    spec.generated_training_path = Some(
        generated_dir
            .strip_prefix(&paths.root)
            .unwrap_or(&generated_dir)
            .display()
            .to_string(),
    );
    spec.wan_task = Some(lane.task.to_string());
    spec.target_frames = target_frames;
    spec.source_fps = source_fps;
    spec.frame_extraction = frame_extraction.clone();
    spec.musubi_wsl_distro = Some(training::WSL_DISTRO.to_string());
    spec.musubi_wsl_root = Some(training::WSL_MUSUBI_ROOT.to_string());

    let media_rows = collect_media_rows(dataset_path, spec, dataset_kind);
    let metadata_jsonl_path = generated_dir.join(dataset_kind.metadata_file_name());
    let metadata_jsonl = media_rows
        .iter()
        .map(|row| {
            let mut value = serde_json::Map::new();
            value.insert(
                dataset_kind.metadata_path_key().to_string(),
                json!(windows_path_to_wsl(&row.path)),
            );
            value.insert("caption".to_string(), json!(row.caption));
            serde_json::Value::Object(value).to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(
        &metadata_jsonl_path,
        if metadata_jsonl.is_empty() {
            String::new()
        } else {
            format!("{}\n", metadata_jsonl)
        },
    )
    .with_context(|| format!("could not write {}", metadata_jsonl_path.display()))?;

    let dataset_toml_path = generated_dir.join("dataset.toml");
    let dataset_kind_lines = if dataset_kind.is_video() {
        format!(
            r#"target_frames = [{target_frames}]
frame_extraction = "{frame_extraction}"
source_fps = {source_fps:.1}
"#,
            target_frames = target_frames.unwrap_or(lane.defaults.target_frames.unwrap_or(17)),
            frame_extraction = frame_extraction
                .as_deref()
                .unwrap_or(lane.defaults.frame_extraction.unwrap_or("head")),
            source_fps = source_fps.unwrap_or(lane.defaults.source_fps.unwrap_or(16.0)),
        )
    } else {
        String::new()
    };
    let dataset_toml = format!(
        r#"[general]
resolution = [{resolution}, {resolution}]
batch_size = {batch_size}
enable_bucket = true
bucket_no_upscale = false

[[datasets]]
{jsonl_key} = "{metadata_jsonl}"
cache_directory = "{cache_directory}"
num_repeats = {repeats}
{dataset_kind_lines}
"#,
        resolution = spec.resolution,
        batch_size = spec.batch_size.max(1),
        jsonl_key = dataset_kind.jsonl_toml_key(),
        metadata_jsonl = windows_path_to_wsl(&metadata_jsonl_path),
        cache_directory = windows_path_to_wsl(&cache_dir),
        repeats = spec.repeats.max(1),
        dataset_kind_lines = dataset_kind_lines,
    );
    std::fs::write(&dataset_toml_path, dataset_toml)
        .with_context(|| format!("could not write {}", dataset_toml_path.display()))?;

    let sample_prompt_path = generated_dir.join("sample_prompts.txt");
    std::fs::write(&sample_prompt_path, build_sample_prompt(spec))
        .with_context(|| format!("could not write {}", sample_prompt_path.display()))?;

    let script_context = WanScriptContext {
        generated_dir: generated_dir.clone(),
        output_dir: output_dir.clone(),
        dataset_toml_path,
        metadata_jsonl_path,
        dit_path: wan_paths.dit,
        t5_path: wan_paths.t5,
        vae_path: wan_paths.vae,
        project_slug: spec.project_slug.clone(),
        rank: spec.rank.max(4),
        epochs: spec.epochs.max(1),
        learning_rate: spec.learning_rate,
        dataset_kind,
        training_backend_id: spec.training_backend_id.clone(),
        lane_label: lane.label.to_string(),
        backend_selection_mode: backend_selection_mode.to_string(),
    };

    write_script(
        &generated_dir.join("cache_latents.sh"),
        &build_cache_latents_script(&script_context),
    )?;
    write_script(
        &generated_dir.join("cache_text.sh"),
        &build_cache_text_script(&script_context),
    )?;
    write_script(
        &generated_dir.join("launch.sh"),
        &build_launch_script(&script_context),
    )?;
    write_script(
        &generated_dir.join("run_all.sh"),
        &build_run_all_script(&script_context),
    )?;
    write_script(
        &generated_dir.join("preflight.sh"),
        &build_preflight_script(&script_context),
    )?;

    std::fs::write(
        generated_dir.join("README.md"),
        build_generated_readme(&script_context, media_rows.len()),
    )
    .with_context(|| {
        format!(
            "could not write {}",
            generated_dir.join("README.md").display()
        )
    })?;

    let plan_json = json!({
        "project_slug": spec.project_slug,
        "project_name": spec.project_name,
        "dataset_slug": spec.dataset_slug,
        "backend": &spec.training_backend_id,
        "training_backend_id": &spec.training_backend_id,
        "backend_selection_mode": backend_selection_mode,
        "backend_selection_manually_overridden": spec.backend_selection_manually_overridden,
        "lane_label": lane.label,
        "family_id": lane.family_id,
        "task": lane.task,
        "dataset_kind": dataset_kind.media_label(),
        "video_rows": if dataset_kind.is_video() { Some(media_rows.len()) } else { None },
        "image_rows": if dataset_kind.is_image() { Some(media_rows.len()) } else { None },
        "resolution": spec.resolution,
        "target_frames": target_frames,
        "source_fps": source_fps,
        "frame_extraction": frame_extraction,
        "rank": spec.rank,
        "epochs": spec.epochs,
        "learning_rate": spec.learning_rate,
        "wsl_distro": training::WSL_DISTRO,
        "wsl_musubi_root": training::WSL_MUSUBI_ROOT,
        "dataset_toml": windows_path_to_wsl(&script_context.dataset_toml_path),
        "output_dir": windows_path_to_wsl(&script_context.output_dir),
        "notes": [
            "Run preflight.sh first, cache_latents.sh second, cache_text.sh third, launch.sh last.",
            "Generated from Chatty-lora; edit these files if Musubi changes its command flags."
        ]
    });
    std::fs::write(
        generated_dir.join("plan.json"),
        serde_json::to_string_pretty(&plan_json)?,
    )
    .with_context(|| {
        format!(
            "could not write {}",
            generated_dir.join("plan.json").display()
        )
    })?;

    Ok((
        spec.generated_training_path.clone().unwrap_or_default(),
        media_rows.len(),
        dataset_kind.media_label(),
    ))
}

#[derive(Debug)]
struct MediaRow {
    path: PathBuf,
    caption: String,
}

#[derive(Deserialize)]
struct DatasetManifest {
    items: Vec<DatasetManifestItem>,
}

#[derive(Deserialize)]
struct DatasetManifestItem {
    title: String,
    kind: String,
    saved_path: String,
}

fn collect_media_rows(
    dataset_path: &Path,
    spec: &ProjectSpec,
    dataset_kind: WanDatasetKind,
) -> Vec<MediaRow> {
    let manifest_path = dataset_path.join("metadata.json");
    if let Ok(contents) = std::fs::read_to_string(&manifest_path) {
        if let Ok(manifest) = serde_json::from_str::<DatasetManifest>(&contents) {
            let rows = manifest
                .items
                .into_iter()
                .filter(|item| item.kind == dataset_kind.manifest_kind())
                .map(|item| {
                    let path = dataset_path.join(item.saved_path);
                    MediaRow {
                        path,
                        caption: build_caption(spec, &item.title, dataset_kind),
                    }
                })
                .collect::<Vec<_>>();
            if !rows.is_empty() {
                return rows;
            }
        }
    }

    WalkDir::new(dataset_path)
        .min_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| dataset_kind.matches_path(entry.path()))
        .map(|entry| {
            let title = entry
                .path()
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .replace(['-', '_'], " ");
            MediaRow {
                path: entry.path().to_path_buf(),
                caption: build_caption(spec, &title, dataset_kind),
            }
        })
        .collect()
}

fn build_caption(spec: &ProjectSpec, title: &str, dataset_kind: WanDatasetKind) -> String {
    let mut parts = Vec::new();
    let concept_blocks = effective_concept_blocks(spec);

    for trigger in concept_blocks
        .iter()
        .filter(|block| block.role == "primary")
        .map(|block| block.trigger_phrase.trim())
        .filter(|value| !value.is_empty())
    {
        push_unique_part(&mut parts, trigger);
    }
    if !title.trim().is_empty() {
        push_unique_part(&mut parts, title.trim());
    }
    for summary in concept_blocks
        .iter()
        .filter(|block| block.role == "primary")
        .flat_map(|block| [block.concept_summary.trim(), block.training_intent.trim()])
        .filter(|value| !value.is_empty())
    {
        push_unique_part(&mut parts, summary);
    }
    for trigger in concept_blocks
        .iter()
        .filter(|block| block.role == "supporting")
        .map(|block| block.trigger_phrase.trim())
        .filter(|value| !value.is_empty())
    {
        push_unique_part(&mut parts, trigger);
    }
    for summary in concept_blocks
        .iter()
        .filter(|block| block.role == "supporting")
        .flat_map(|block| [block.concept_summary.trim(), block.training_intent.trim()])
        .filter(|value| !value.is_empty())
    {
        push_unique_part(&mut parts, summary);
    }
    if parts.is_empty() {
        format!("training {}", dataset_kind.media_label())
    } else {
        parts.join(", ")
    }
}

fn push_unique_part(parts: &mut Vec<String>, value: &str) {
    if !parts.iter().any(|part| part.eq_ignore_ascii_case(value)) {
        parts.push(value.to_string());
    }
}

fn build_sample_prompt(spec: &ProjectSpec) -> String {
    let prompt = build_caption(
        spec,
        &spec.project_name,
        WanDatasetKind::from_backend_id(&spec.training_backend_id),
    );
    format!("{}\n", prompt)
}

struct WanScriptContext {
    generated_dir: PathBuf,
    output_dir: PathBuf,
    dataset_toml_path: PathBuf,
    metadata_jsonl_path: PathBuf,
    dit_path: PathBuf,
    t5_path: PathBuf,
    vae_path: PathBuf,
    project_slug: String,
    rank: u32,
    epochs: u32,
    learning_rate: f32,
    dataset_kind: WanDatasetKind,
    training_backend_id: String,
    lane_label: String,
    backend_selection_mode: String,
}

fn build_preflight_script(context: &WanScriptContext) -> String {
    let media_label = context.dataset_kind.media_label();
    let media_command_checks = if context.dataset_kind.is_video() {
        "check_command ffmpeg\ncheck_command ffprobe".to_string()
    } else {
        "echo \"Image dataset lane: ffmpeg/ffprobe is not required for still-image preflight.\""
            .to_string()
    };
    let media_probe_python = if context.dataset_kind.is_video() {
        r#"for index, row in enumerate(rows[:3], start=1):
    probe = subprocess.run(
        [
            "ffprobe",
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,avg_frame_rate,nb_frames",
            "-of",
            "json",
            row[path_key],
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    if probe.returncode != 0:
        fail(f"ffprobe could not read {media_label} row {index}: {probe.stderr.strip() or row[path_key]}")
    try:
        probe_json = json.loads(probe.stdout or "{}")
    except json.JSONDecodeError as exc:
        fail(f"ffprobe returned invalid JSON for {media_label} row {index}: {exc}")
    if not probe_json.get("streams"):
        fail(f"ffprobe found no video stream for row {index}: {row[path_key]}")
ok(f"ffprobe read {min(len(rows), 3)} {media_label} row(s)")
"#
        .to_string()
    } else {
        r#"try:
    from PIL import Image
except Exception as exc:
    print(f"WARN: Pillow image decode check unavailable: {exc}", file=sys.stderr)
else:
    for index, row in enumerate(rows[:3], start=1):
        try:
            with Image.open(row[path_key]) as image:
                image.verify()
        except Exception as exc:
            fail(f"Pillow could not verify image row {index}: {exc}")
    ok(f"Pillow verified {min(len(rows), 3)} image row(s)")
"#
        .to_string()
    };

    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

{env_prefix}

echo "== Chatty-lora Wan {media_label} preflight =="

MUSUBI_ROOT="${{MUSUBI_ROOT:-$HOME/train_runtime/musubi-tuner}}"
GENERATED_DIR="{generated_dir}"
OUTPUT_DIR="{output_dir}"
DATASET_CONFIG="{dataset_config}"
MEDIA_METADATA="{media_metadata}"
DIT="{dit}"
T5="{t5}"
VAE="{vae}"

fail() {{
  echo "FAIL: $*" >&2
  exit 1
}}

pass() {{
  echo "OK: $*"
}}

warn() {{
  echo "WARN: $*" >&2
}}

check_file() {{
  local path="$1"
  local label="$2"
  [[ -f "$path" ]] || fail "$label missing: $path"
  pass "$label found"
}}

check_dir() {{
  local path="$1"
  local label="$2"
  [[ -d "$path" ]] || fail "$label missing: $path"
  pass "$label found"
}}

check_command() {{
  local name="$1"
  command -v "$name" >/dev/null 2>&1 || fail "$name command is not available in WSL"
  pass "$name command available"
}}

check_dir "$GENERATED_DIR" "Generated handoff folder"
check_dir "$OUTPUT_DIR" "Training output folder"
check_file "$DATASET_CONFIG" "dataset.toml"
check_file "$MEDIA_METADATA" "{metadata_file_name}"
check_file "$DIT" "Wan DiT"
check_file "$T5" "UMT5 text encoder"
check_file "$VAE" "Wan VAE"

check_file "$GENERATED_DIR/preflight.sh" "preflight.sh"
check_file "$GENERATED_DIR/cache_latents.sh" "cache_latents.sh"
check_file "$GENERATED_DIR/cache_text.sh" "cache_text.sh"
check_file "$GENERATED_DIR/launch.sh" "launch.sh"
check_file "$GENERATED_DIR/run_all.sh" "run_all.sh"

bash -n "$GENERATED_DIR/preflight.sh"
bash -n "$GENERATED_DIR/cache_latents.sh"
bash -n "$GENERATED_DIR/cache_text.sh"
bash -n "$GENERATED_DIR/launch.sh"
bash -n "$GENERATED_DIR/run_all.sh"
pass "Generated shell scripts passed bash syntax checks"

{media_command_checks}

if command -v rocm_agent_enumerator >/dev/null 2>&1; then
  echo "ROCm agents:"
  rocm_agent_enumerator || warn "rocm_agent_enumerator returned a non-zero status"
else
  warn "rocm_agent_enumerator is not available; continuing because PyTorch GPU detection is the stronger check."
fi

check_dir "$MUSUBI_ROOT" "Musubi root"
cd "$MUSUBI_ROOT"
check_file ".venv/bin/python" "Musubi virtualenv Python"
check_file "src/musubi_tuner/wan_cache_latents.py" "Musubi Wan latent cache script"
check_file "src/musubi_tuner/wan_cache_text_encoder_outputs.py" "Musubi Wan text cache script"
check_file "src/musubi_tuner/wan_train_network.py" "Musubi Wan training script"

source .venv/bin/activate

export BABY_DRAGON_DATASET_CONFIG="$DATASET_CONFIG"
export BABY_DRAGON_MEDIA_METADATA="$MEDIA_METADATA"
export BABY_DRAGON_MEDIA_LABEL="{media_label}"
export BABY_DRAGON_MEDIA_PATH_KEY="{media_path_key}"
export BABY_DRAGON_DIT="$DIT"
export BABY_DRAGON_T5="$T5"
export BABY_DRAGON_VAE="$VAE"

python - <<'PY'
import json
import os
import subprocess
import sys

def fail(message):
    print(f"FAIL: {{message}}", file=sys.stderr)
    raise SystemExit(1)

def ok(message):
    print(f"OK: {{message}}")

for env_name, label in [
    ("BABY_DRAGON_DATASET_CONFIG", "dataset config"),
    ("BABY_DRAGON_MEDIA_METADATA", f"{{os.environ.get('BABY_DRAGON_MEDIA_LABEL', 'media')}} metadata"),
    ("BABY_DRAGON_DIT", "Wan DiT"),
    ("BABY_DRAGON_T5", "UMT5 text encoder"),
    ("BABY_DRAGON_VAE", "Wan VAE"),
]:
    path = os.environ.get(env_name, "")
    if not path or not os.path.exists(path):
        fail(f"{{label}} path does not exist: {{path}}")
    ok(f"{{label}} path exists")

media_label = os.environ.get("BABY_DRAGON_MEDIA_LABEL", "media")
path_key = os.environ.get("BABY_DRAGON_MEDIA_PATH_KEY", "media_path")
metadata_path = os.environ["BABY_DRAGON_MEDIA_METADATA"]
rows = []
with open(metadata_path, "r", encoding="utf-8") as handle:
    for line_number, line in enumerate(handle, start=1):
        line = line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as exc:
            fail(f"{{media_label}} metadata line {{line_number}} is not valid JSON: {{exc}}")
        media_path = row.get(path_key)
        caption = row.get("caption")
        if not media_path:
            fail(f"{{media_label}} metadata line {{line_number}} has no {{path_key}}")
        if not os.path.exists(media_path):
            fail(f"{{media_label}} path from line {{line_number}} does not exist: {{media_path}}")
        if not caption:
            fail(f"{{media_label}} metadata line {{line_number}} has no caption")
        rows.append(row)

if not rows:
    fail(f"{{metadata_path}} has no {{media_label}} rows")
ok(f"{{len(rows)}} {{media_label}} metadata row(s) passed path and caption checks")

{media_probe_python}

try:
    import accelerate  # noqa: F401
except Exception as exc:
    fail(f"could not import accelerate from the Musubi venv: {{exc}}")
ok("accelerate import succeeded")

try:
    import torch
except Exception as exc:
    fail(f"could not import torch from the Musubi venv: {{exc}}")

ok(f"torch import succeeded: {{torch.__version__}}")
if not torch.cuda.is_available():
    fail("torch.cuda.is_available() returned False")
ok(f"PyTorch ROCm sees GPU: {{torch.cuda.get_device_name(0)}}")

print(f"Chatty-lora Wan {{media_label}} preflight passed. No cache or training step was run.")
PY
"#,
        env_prefix = training::WSL_ENV_PREFIX,
        media_label = media_label,
        generated_dir = windows_path_to_wsl(&context.generated_dir),
        output_dir = windows_path_to_wsl(&context.output_dir),
        dataset_config = windows_path_to_wsl(&context.dataset_toml_path),
        media_metadata = windows_path_to_wsl(&context.metadata_jsonl_path),
        metadata_file_name = context.dataset_kind.metadata_file_name(),
        media_path_key = context.dataset_kind.metadata_path_key(),
        media_command_checks = media_command_checks,
        media_probe_python = media_probe_python,
        dit = windows_path_to_wsl(&context.dit_path),
        t5 = windows_path_to_wsl(&context.t5_path),
        vae = windows_path_to_wsl(&context.vae_path),
    )
}

fn build_cache_latents_script(context: &WanScriptContext) -> String {
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

{env_prefix}

MUSUBI_ROOT="${{MUSUBI_ROOT:-$HOME/train_runtime/musubi-tuner}}"
cd "$MUSUBI_ROOT"
source .venv/bin/activate

python src/musubi_tuner/wan_cache_latents.py \
  --dataset_config "{dataset_config}" \
  --vae "{vae}" \
  --device cpu \
  --batch_size 1 \
  --skip_existing \
  --vae_cache_cpu \
  --disable_cudnn_backend
"#,
        env_prefix = training::WSL_ENV_PREFIX,
        dataset_config = windows_path_to_wsl(&context.dataset_toml_path),
        vae = windows_path_to_wsl(&context.vae_path),
    )
}

fn build_cache_text_script(context: &WanScriptContext) -> String {
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

{env_prefix}

MUSUBI_ROOT="${{MUSUBI_ROOT:-$HOME/train_runtime/musubi-tuner}}"
cd "$MUSUBI_ROOT"
source .venv/bin/activate

python src/musubi_tuner/wan_cache_text_encoder_outputs.py \
  --dataset_config "{dataset_config}" \
  --t5 "{t5}" \
  --device cpu \
  --batch_size 1 \
  --skip_existing
"#,
        env_prefix = training::WSL_ENV_PREFIX,
        dataset_config = windows_path_to_wsl(&context.dataset_toml_path),
        t5 = windows_path_to_wsl(&context.t5_path),
    )
}

fn build_launch_script(context: &WanScriptContext) -> String {
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

{env_prefix}

MUSUBI_ROOT="${{MUSUBI_ROOT:-$HOME/train_runtime/musubi-tuner}}"
cd "$MUSUBI_ROOT"
source .venv/bin/activate

accelerate launch --num_cpu_threads_per_process 1 --mixed_precision bf16 src/musubi_tuner/wan_train_network.py \
  --task t2v-1.3B \
  --dit "{dit}" \
  --dataset_config "{dataset_config}" \
  --sdpa \
  --split_attn \
  --mixed_precision bf16 \
  --fp8_base \
  --fp8_scaled \
  --optimizer_type AdamW \
  --learning_rate {learning_rate} \
  --gradient_checkpointing \
  --gradient_checkpointing_cpu_offload \
  --blocks_to_swap 20 \
  --img_in_txt_in_offloading \
  --max_data_loader_n_workers 1 \
  --network_module networks.lora_wan \
  --network_dim {rank} \
  --network_alpha {rank} \
  --timestep_sampling shift \
  --discrete_flow_shift 3.0 \
  --max_train_epochs {epochs} \
  --save_every_n_epochs 1 \
  --seed 42 \
  --output_dir "{output_dir}" \
  --output_name "{output_name}"
"#,
        env_prefix = training::WSL_ENV_PREFIX,
        dit = windows_path_to_wsl(&context.dit_path),
        dataset_config = windows_path_to_wsl(&context.dataset_toml_path),
        learning_rate = context.learning_rate,
        rank = context.rank,
        epochs = context.epochs,
        output_dir = windows_path_to_wsl(&context.output_dir.join("loras")),
        output_name = context.project_slug,
    )
}

fn build_run_all_script(context: &WanScriptContext) -> String {
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

bash "{preflight}"
bash "{cache_latents}"
bash "{cache_text}"
bash "{launch}"
"#,
        preflight = windows_path_to_wsl(&context.generated_dir.join("preflight.sh")),
        cache_latents = windows_path_to_wsl(&context.generated_dir.join("cache_latents.sh")),
        cache_text = windows_path_to_wsl(&context.generated_dir.join("cache_text.sh")),
        launch = windows_path_to_wsl(&context.generated_dir.join("launch.sh")),
    )
}

fn build_generated_readme(context: &WanScriptContext, video_count: usize) -> String {
    let media_label = context.dataset_kind.media_label();
    let preflight = sh_quote(&windows_path_to_wsl(
        &context.generated_dir.join("preflight.sh"),
    ));
    let cache_latents = sh_quote(&windows_path_to_wsl(
        &context.generated_dir.join("cache_latents.sh"),
    ));
    let cache_text = sh_quote(&windows_path_to_wsl(
        &context.generated_dir.join("cache_text.sh"),
    ));
    let launch = sh_quote(&windows_path_to_wsl(
        &context.generated_dir.join("launch.sh"),
    ));
    let run_all = sh_quote(&windows_path_to_wsl(
        &context.generated_dir.join("run_all.sh"),
    ));

    format!(
        r#"# Wan 2.1 Musubi {media_label} handoff: {project}

This folder was generated by Chatty-lora for the first Wan 2.1 T2V 1.3B training lane.

Training lane: {lane_label}
Backend id: {training_backend_id}
Backend selection: {backend_selection_mode}

{media_label_title} rows detected: {video_count}

Run from Windows PowerShell if you want to launch the scripts manually:

```powershell
wsl -d Ubuntu-24.04 -- bash -lc "bash {preflight}"
wsl -d Ubuntu-24.04 -- bash -lc "bash {cache_latents}"
wsl -d Ubuntu-24.04 -- bash -lc "bash {cache_text}"
wsl -d Ubuntu-24.04 -- bash -lc "bash {launch}"
```

Or run the full sequence:

```powershell
wsl -d Ubuntu-24.04 -- bash -lc "bash {run_all}"
```

The generated order is:

0. Preflight checks.
1. Cache latents.
2. Cache text encoder outputs.
3. Launch LoRA training.

Low-VRAM notes:

- `cache_latents.sh` runs VAE latent caching on CPU.
- `cache_text.sh` runs T5 text-encoder caching on CPU without fp8.
- `launch.sh` still trains on the GPU, but uses split attention, FP8-scaled Wan weights, input offload, and block swapping to reduce dedicated VRAM pressure.
- This route is slower than a high-VRAM trainer, but it is the conservative proven Chatty-lora path for cautious 8GB AMD/Radeon tests.
- This {media_label} lane uses the same Wan model family as the video lane, so it is meant to teach visual identity/style foundations before later video refinement.

If Musubi changes command flags later, edit the shell scripts in this folder before running.
"#,
        project = context.project_slug,
        media_label = media_label,
        lane_label = context.lane_label,
        training_backend_id = context.training_backend_id,
        backend_selection_mode = context.backend_selection_mode,
        media_label_title = titlecase_ascii(media_label),
        video_count = video_count,
        preflight = preflight,
        cache_latents = cache_latents,
        cache_text = cache_text,
        launch = launch,
        run_all = run_all,
    )
}

fn write_script(path: &Path, contents: &str) -> Result<()> {
    let normalized = contents.replace("\r\n", "\n");
    std::fs::write(path, normalized).with_context(|| format!("could not write {}", path.display()))
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

fn is_video_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref(),
        Some("mp4" | "avi" | "mov" | "mkv" | "webm")
    )
}

fn is_image_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif")
    )
}

fn titlecase_ascii(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
}
