use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Serialize;
use tokio::fs;
use walkdir::WalkDir;

use crate::{
    state::ProjectPaths,
    types::{
        BridgeDatasetImportRequest, DatasetCreateRequest, DatasetCreateResponse,
        LocalDatasetImportRequest, PreviewItem,
    },
};

const USER_AGENT: &str = "Chatty-lora/0.1 (+https://github.com/)";

#[derive(Debug, serde::Deserialize)]
struct BridgeIncomingAssetRecord {
    #[serde(default)]
    label: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    file_name: String,
    #[serde(default)]
    payload_file_name: String,
}

#[derive(Debug)]
struct BridgeImportItem {
    asset_id: String,
    label: String,
    summary: String,
    file_name: String,
    payload_path: PathBuf,
}

pub async fn create_dataset(
    client: &Client,
    paths: &ProjectPaths,
    request: DatasetCreateRequest,
) -> Result<DatasetCreateResponse> {
    let dataset_name = request.dataset_name.trim();
    if dataset_name.is_empty() {
        anyhow::bail!("Give the dataset a name before curating material.");
    }

    let mut unique_items = Vec::new();
    let mut seen = BTreeSet::new();
    for item in request.selected_items {
        if seen.insert(item.key.clone()) {
            unique_items.push(item);
        }
    }

    if unique_items.is_empty() {
        anyhow::bail!("Select at least one preview item before curating a dataset.");
    }

    fs::create_dir_all(&paths.inputs)
        .await
        .with_context(|| format!("could not create {}", paths.inputs.display()))?;

    let dataset_slug = slugify(dataset_name);
    let dataset_dir = next_available_dataset_dir(&paths.inputs, &dataset_slug).await?;
    fs::create_dir_all(&dataset_dir)
        .await
        .with_context(|| format!("could not create {}", dataset_dir.display()))?;

    let mut notes = Vec::new();
    let mut manifest_items = Vec::new();
    let mut saved_items = 0usize;
    let mut failed_items = 0usize;
    let mut image_index = 0usize;
    let mut audio_index = 0usize;
    let mut video_index = 0usize;
    let mut other_index = 0usize;

    for item in unique_items {
        let (kind_folder, index) = match item.kind.as_str() {
            "Image" => {
                image_index += 1;
                ("images", image_index)
            }
            "Audio" => {
                audio_index += 1;
                ("audio", audio_index)
            }
            "Video" => {
                video_index += 1;
                ("video", video_index)
            }
            _ => {
                other_index += 1;
                ("other", other_index)
            }
        };

        let target_dir = dataset_dir.join(kind_folder);
        fs::create_dir_all(&target_dir)
            .await
            .with_context(|| format!("could not create {}", target_dir.display()))?;

        match download_item(client, &item, &target_dir, index).await {
            Ok(saved_path) => {
                let caption_path = match write_caption_sidecar(&saved_path, &item).await {
                    Ok(path) => Some(
                        path.strip_prefix(&dataset_dir)
                            .unwrap_or(&path)
                            .display()
                            .to_string(),
                    ),
                    Err(error) => {
                        notes.push(format!(
                            "Saved {}, but could not write its sidecar caption: {}",
                            item.title, error
                        ));
                        None
                    }
                };
                saved_items += 1;
                manifest_items.push(ManifestItem {
                    key: item.key.clone(),
                    title: item.title.clone(),
                    source_label: item.source_label.clone(),
                    source_page_url: item.source_page_url.clone(),
                    media_url: item.media_url.clone(),
                    license: item.license.clone(),
                    creator: item.creator.clone(),
                    page_number: item.page_number,
                    kind: item.kind.clone(),
                    saved_path: saved_path
                        .strip_prefix(&dataset_dir)
                        .unwrap_or(&saved_path)
                        .display()
                        .to_string(),
                    caption_path,
                });
            }
            Err(error) => {
                failed_items += 1;
                notes.push(format!("Skipped {}: {}", item.title, error));
            }
        }
    }

    let manifest = DatasetManifest {
        dataset_name: dataset_name.to_string(),
        dataset_slug: dataset_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        created_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        saved_items,
        failed_items,
        notes: notes.clone(),
        items: manifest_items,
    };

    let manifest_path = dataset_dir.join("metadata.json");
    let manifest_json =
        serde_json::to_string_pretty(&manifest).context("could not serialize dataset metadata")?;
    fs::write(&manifest_path, manifest_json)
        .await
        .with_context(|| format!("could not write {}", manifest_path.display()))?;

    if saved_items == 0 {
        notes.push(
            "No media files were saved. Check the selected items, licenses, or source availability."
                .to_string(),
        );
    } else {
        notes.push(format!(
            "Curated {} item{} into {}.",
            saved_items,
            if saved_items == 1 { "" } else { "s" },
            dataset_dir.display()
        ));
        notes.push(
            "Wrote simple sidecar captions beside saved media where possible, so Builder lanes have a cleaner starting point."
                .to_string(),
        );
    }

    Ok(DatasetCreateResponse {
        dataset_slug: dataset_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        dataset_path: dataset_dir.display().to_string(),
        manifest_path: manifest_path.display().to_string(),
        saved_items,
        failed_items,
        notes,
    })
}

pub async fn import_local_dataset(
    paths: &ProjectPaths,
    request: LocalDatasetImportRequest,
) -> Result<DatasetCreateResponse> {
    let dataset_name = request.dataset_name.trim();
    if dataset_name.is_empty() {
        anyhow::bail!("Give the cleaned dataset a name first.");
    }

    let source_dir = resolve_input_source_dir(paths, &request.source_folder)?;
    let source_label = source_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let media_files = collect_local_media_files(&source_dir)?;
    if media_files.is_empty() {
        anyhow::bail!(
            "{} does not contain supported image, audio, or video files yet.",
            source_dir.display()
        );
    }

    fs::create_dir_all(&paths.inputs)
        .await
        .with_context(|| format!("could not create {}", paths.inputs.display()))?;

    let dataset_slug = slugify(dataset_name);
    let dataset_dir = next_available_dataset_dir(&paths.inputs, &dataset_slug).await?;
    fs::create_dir_all(&dataset_dir)
        .await
        .with_context(|| format!("could not create {}", dataset_dir.display()))?;

    let mut notes = Vec::new();
    let mut manifest_items = Vec::new();
    let mut saved_items = 0usize;
    let mut failed_items = 0usize;
    let mut image_index = 0usize;
    let mut audio_index = 0usize;
    let mut video_index = 0usize;

    for source_path in media_files {
        let Some(kind) = local_media_kind(&source_path) else {
            continue;
        };
        let (kind_folder, index) = match kind {
            "Image" => {
                image_index += 1;
                ("images", image_index)
            }
            "Audio" => {
                audio_index += 1;
                ("audio", audio_index)
            }
            "Video" => {
                video_index += 1;
                ("video", video_index)
            }
            _ => continue,
        };

        let target_dir = dataset_dir.join(kind_folder);
        fs::create_dir_all(&target_dir)
            .await
            .with_context(|| format!("could not create {}", target_dir.display()))?;

        let title = local_file_title(&source_path);
        let extension = extension_from_path(&source_path);
        let filename = format!(
            "{}-{:04}-{}.{}",
            kind.to_ascii_lowercase(),
            index,
            truncate_slug(&slugify(&title), 48),
            extension
        );
        let target_path = target_dir.join(filename);

        match fs::copy(&source_path, &target_path).await {
            Ok(_) => {
                let caption_path = match write_local_caption_sidecar(
                    &source_path,
                    &target_path,
                    &title,
                    &source_label,
                )
                .await
                {
                    Ok(path) => Some(
                        path.strip_prefix(&dataset_dir)
                            .unwrap_or(&path)
                            .display()
                            .to_string(),
                    ),
                    Err(error) => {
                        notes.push(format!(
                            "Copied {}, but could not write its sidecar caption: {}",
                            source_path.display(),
                            error
                        ));
                        None
                    }
                };

                let original_relative = source_path
                    .strip_prefix(&source_dir)
                    .unwrap_or(&source_path)
                    .display()
                    .to_string();
                saved_items += 1;
                manifest_items.push(ManifestItem {
                    key: format!(
                        "local::{}::{}",
                        request.source_folder.trim(),
                        original_relative
                    ),
                    title,
                    source_label: format!("Local folder: {}", source_label),
                    source_page_url: original_relative.clone(),
                    media_url: original_relative,
                    license: None,
                    creator: None,
                    page_number: 0,
                    kind: kind.to_string(),
                    saved_path: target_path
                        .strip_prefix(&dataset_dir)
                        .unwrap_or(&target_path)
                        .display()
                        .to_string(),
                    caption_path,
                });
            }
            Err(error) => {
                failed_items += 1;
                notes.push(format!("Skipped {}: {}", source_path.display(), error));
            }
        }
    }

    let manifest = DatasetManifest {
        dataset_name: dataset_name.to_string(),
        dataset_slug: dataset_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        created_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        saved_items,
        failed_items,
        notes: notes.clone(),
        items: manifest_items,
    };

    let manifest_path = dataset_dir.join("metadata.json");
    let manifest_json =
        serde_json::to_string_pretty(&manifest).context("could not serialize dataset metadata")?;
    fs::write(&manifest_path, manifest_json)
        .await
        .with_context(|| format!("could not write {}", manifest_path.display()))?;

    if saved_items == 0 {
        notes.push(
            "No local media files were copied. Check the source folder contents and file types."
                .to_string(),
        );
    } else {
        notes.push(format!(
            "Cleaned {} local file{} from {} into {}.",
            saved_items,
            if saved_items == 1 { "" } else { "s" },
            source_label,
            dataset_dir.display()
        ));
        notes.push(
            "Original files were left alone. The cleaned copy has media buckets, normalized filenames, sidecar captions, and metadata.json."
                .to_string(),
        );
    }

    Ok(DatasetCreateResponse {
        dataset_slug: dataset_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        dataset_path: dataset_dir.display().to_string(),
        manifest_path: manifest_path.display().to_string(),
        saved_items,
        failed_items,
        notes,
    })
}

pub async fn import_bridge_dataset(
    paths: &ProjectPaths,
    request: BridgeDatasetImportRequest,
) -> Result<DatasetCreateResponse> {
    let dataset_name = request.dataset_name.trim();
    if dataset_name.is_empty() {
        anyhow::bail!("Give the imported dataset a name first.");
    }
    let lane_id = sanitize_lane_component(&request.lane_id);
    if lane_id.is_empty() {
        anyhow::bail!("Bridge lane id is required.");
    }

    let items = collect_bridge_import_items(paths, &lane_id, &request.asset_ids)?;
    if items.is_empty() {
        anyhow::bail!("Select at least one bridge asset to import.");
    }

    fs::create_dir_all(&paths.inputs)
        .await
        .with_context(|| format!("could not create {}", paths.inputs.display()))?;

    let dataset_slug = slugify(dataset_name);
    let dataset_dir = next_available_dataset_dir(&paths.inputs, &dataset_slug).await?;
    fs::create_dir_all(&dataset_dir)
        .await
        .with_context(|| format!("could not create {}", dataset_dir.display()))?;

    let mut notes = Vec::new();
    let mut manifest_items = Vec::new();
    let mut saved_items = 0usize;
    let mut failed_items = 0usize;
    let mut image_index = 0usize;
    let mut audio_index = 0usize;
    let mut video_index = 0usize;

    for item in items {
        let Some(kind) = local_media_kind(&item.payload_path) else {
            notes.push(format!("Skipped {} because its file type is not supported yet.", item.file_name));
            failed_items += 1;
            continue;
        };

        let (kind_folder, index) = match kind {
            "Image" => {
                image_index += 1;
                ("images", image_index)
            }
            "Audio" => {
                audio_index += 1;
                ("audio", audio_index)
            }
            "Video" => {
                video_index += 1;
                ("video", video_index)
            }
            _ => continue,
        };

        let target_dir = dataset_dir.join(kind_folder);
        fs::create_dir_all(&target_dir)
            .await
            .with_context(|| format!("could not create {}", target_dir.display()))?;

        let title = if item.label.trim().is_empty() {
            local_file_title(&item.payload_path)
        } else {
            clean_caption_text(&item.label)
        };
        let extension = extension_from_path(&item.payload_path);
        let filename = format!(
            "{}-{:04}-{}.{}",
            kind.to_ascii_lowercase(),
            index,
            truncate_slug(&slugify(&title), 48),
            extension
        );
        let target_path = target_dir.join(filename);

        match fs::copy(&item.payload_path, &target_path).await {
            Ok(_) => {
                let caption_path = match write_bridge_caption_sidecar(&target_path, &title, &item.summary).await {
                    Ok(path) => Some(
                        path.strip_prefix(&dataset_dir)
                            .unwrap_or(&path)
                            .display()
                            .to_string(),
                    ),
                    Err(error) => {
                        notes.push(format!(
                            "Copied {}, but could not write its sidecar caption: {}",
                            item.file_name, error
                        ));
                        None
                    }
                };

                saved_items += 1;
                manifest_items.push(ManifestItem {
                    key: format!("bridge::{}::{}", lane_id, item.asset_id),
                    title,
                    source_label: format!("Bridge lane: {}", lane_id),
                    source_page_url: format!("bridge://{}/{}", lane_id, item.file_name),
                    media_url: format!("bridge://{}/{}", lane_id, item.file_name),
                    license: None,
                    creator: None,
                    page_number: 0,
                    kind: kind.to_string(),
                    saved_path: target_path
                        .strip_prefix(&dataset_dir)
                        .unwrap_or(&target_path)
                        .display()
                        .to_string(),
                    caption_path,
                });
            }
            Err(error) => {
                failed_items += 1;
                notes.push(format!("Skipped {}: {}", item.file_name, error));
            }
        }
    }

    let manifest = DatasetManifest {
        dataset_name: dataset_name.to_string(),
        dataset_slug: dataset_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        created_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        saved_items,
        failed_items,
        notes: notes.clone(),
        items: manifest_items,
    };

    let manifest_path = dataset_dir.join("metadata.json");
    let manifest_json =
        serde_json::to_string_pretty(&manifest).context("could not serialize dataset metadata")?;
    fs::write(&manifest_path, manifest_json)
        .await
        .with_context(|| format!("could not write {}", manifest_path.display()))?;

    if saved_items == 0 {
        notes.push(
            "No bridge assets were imported. Check the selected files and try again.".to_string(),
        );
    } else {
        notes.push(format!(
            "Imported {} bridge asset{} from {} into {}.",
            saved_items,
            if saved_items == 1 { "" } else { "s" },
            lane_id,
            dataset_dir.display()
        ));
        notes.push(
            "The bridge originals were left alone until ChattyCog marks them consumed.".to_string(),
        );
    }

    Ok(DatasetCreateResponse {
        dataset_slug: dataset_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        dataset_path: dataset_dir.display().to_string(),
        manifest_path: manifest_path.display().to_string(),
        saved_items,
        failed_items,
        notes,
    })
}

fn resolve_input_source_dir(paths: &ProjectPaths, source_folder: &str) -> Result<PathBuf> {
    let trimmed = source_folder.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Choose an input folder to clean up first.");
    }

    let relative = Path::new(trimmed);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        anyhow::bail!("Input folder must stay inside the local inputs/ folder.");
    }

    let inputs_root = paths
        .inputs
        .canonicalize()
        .with_context(|| format!("could not resolve {}", paths.inputs.display()))?;
    let source_dir = paths
        .inputs
        .join(relative)
        .canonicalize()
        .with_context(|| format!("could not resolve inputs folder {}", trimmed))?;

    if !source_dir.starts_with(&inputs_root) {
        anyhow::bail!("Refusing to import a folder outside inputs/.");
    }
    if !source_dir.is_dir() {
        anyhow::bail!("{} is not a folder.", source_dir.display());
    }

    Ok(source_dir)
}

fn collect_bridge_import_items(
    paths: &ProjectPaths,
    lane_id: &str,
    asset_ids: &[String],
) -> Result<Vec<BridgeImportItem>> {
    if asset_ids.is_empty() {
        return Ok(Vec::new());
    }

    let lane_dir = paths.root.join("bridge").join("incoming_assets").join(lane_id);
    let root = paths
        .root
        .canonicalize()
        .with_context(|| format!("could not resolve {}", paths.root.display()))?;
    let lane_dir = lane_dir
        .canonicalize()
        .with_context(|| format!("could not resolve {}", lane_dir.display()))?;
    if !lane_dir.starts_with(&root) {
        anyhow::bail!("Refusing to import a bridge lane outside the local project root.");
    }

    let requested = asset_ids
        .iter()
        .map(|asset_id| asset_id.trim().to_string())
        .filter(|asset_id| !asset_id.is_empty())
        .collect::<BTreeSet<_>>();

    let mut items = Vec::new();
    for asset_id in requested {
        let record_path = lane_dir.join(format!("{}.json", asset_id));
        let record_bytes = std::fs::read(&record_path)
            .with_context(|| format!("could not read {}", record_path.display()))?;
        let record: BridgeIncomingAssetRecord = serde_json::from_slice(&record_bytes)
            .with_context(|| format!("could not parse {}", record_path.display()))?;
        let payload_path = lane_dir.join(record.payload_file_name.trim());
        if !payload_path.is_file() {
            anyhow::bail!("Bridge payload file is missing for asset {}.", asset_id);
        }
        items.push(BridgeImportItem {
            asset_id,
            label: record.label,
            summary: record.summary,
            file_name: if record.file_name.trim().is_empty() {
                payload_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            } else {
                record.file_name
            },
            payload_path,
        });
    }

    Ok(items)
}

fn sanitize_lane_component(raw: &str) -> String {
    raw.trim()
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch.to_ascii_lowercase(),
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn collect_local_media_files(source_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(source_dir)
        .min_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.file_name().to_string_lossy() != "metadata.json")
    {
        if local_media_kind(entry.path()).is_some() {
            files.push(entry.path().to_path_buf());
        }
    }

    files.sort_by(|left, right| {
        left.display()
            .to_string()
            .to_ascii_lowercase()
            .cmp(&right.display().to_string().to_ascii_lowercase())
    });
    Ok(files)
}

fn local_media_kind(path: &Path) -> Option<&'static str> {
    match extension_from_path(path).as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" => Some("Image"),
        "wav" | "mp3" | "flac" | "ogg" | "m4a" => Some("Audio"),
        "mp4" | "avi" | "mov" | "mkv" | "webm" => Some("Video"),
        _ => None,
    }
}

fn extension_from_path(path: &Path) -> String {
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    if ext.is_empty() || ext.len() > 12 || !ext.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        "bin".to_string()
    } else {
        ext
    }
}

fn local_file_title(path: &Path) -> String {
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .replace(['_', '-', '.'], " ");
    let cleaned = clean_caption_text(&stem);
    if cleaned.is_empty() {
        "local training reference".to_string()
    } else {
        cleaned
    }
}

async fn write_local_caption_sidecar(
    source_path: &Path,
    target_path: &Path,
    title: &str,
    source_label: &str,
) -> Result<PathBuf> {
    let caption_path = target_path.with_extension("txt");
    let caption = match read_existing_sidecar_caption(source_path).await? {
        Some(caption) => caption,
        None => format!(
            "{}, local import from {}",
            clean_caption_text(title),
            source_label
        ),
    };
    fs::write(&caption_path, caption)
        .await
        .with_context(|| format!("could not write {}", caption_path.display()))?;
    Ok(caption_path)
}

async fn write_bridge_caption_sidecar(
    target_path: &Path,
    title: &str,
    summary: &str,
) -> Result<PathBuf> {
    let caption_path = target_path.with_extension("txt");
    let caption = if !clean_caption_text(summary).is_empty() {
        clean_caption_text(summary)
    } else {
        format!("{}, imported from ChattyCog bridge", clean_caption_text(title))
    };
    fs::write(&caption_path, caption)
        .await
        .with_context(|| format!("could not write {}", caption_path.display()))?;
    Ok(caption_path)
}

async fn read_existing_sidecar_caption(source_path: &Path) -> Result<Option<String>> {
    for extension in ["txt", "caption", "md"] {
        let candidate = source_path.with_extension(extension);
        if !candidate.exists() {
            continue;
        }
        let caption = fs::read_to_string(&candidate)
            .await
            .with_context(|| format!("could not read {}", candidate.display()))?;
        let caption = clean_caption_text(&caption);
        if !caption.is_empty() {
            return Ok(Some(caption));
        }
    }
    Ok(None)
}

async fn next_available_dataset_dir(inputs_root: &Path, dataset_slug: &str) -> Result<PathBuf> {
    let base = inputs_root.join(dataset_slug);
    if !base.exists() {
        return Ok(base);
    }

    for suffix in 2..=9999 {
        let candidate = inputs_root.join(format!("{}-{}", dataset_slug, suffix));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "could not find a free dataset folder name for {}",
        dataset_slug
    );
}

async fn download_item(
    client: &Client,
    item: &PreviewItem,
    target_dir: &Path,
    index: usize,
) -> Result<PathBuf> {
    let response = client
        .get(&item.media_url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .with_context(|| format!("download request failed for {}", item.media_url))?
        .error_for_status()
        .with_context(|| format!("download failed for {}", item.media_url))?;

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let bytes = response
        .bytes()
        .await
        .context("could not read downloaded bytes")?;

    let extension = infer_extension(item, content_type.as_deref());
    let filename = format!(
        "{}-{:04}-{}.{}",
        item.kind.to_ascii_lowercase(),
        index,
        truncate_slug(&slugify(&item.title), 48),
        extension
    );
    let path = target_dir.join(filename);
    fs::write(&path, &bytes)
        .await
        .with_context(|| format!("could not write {}", path.display()))?;
    Ok(path)
}

async fn write_caption_sidecar(saved_path: &Path, item: &PreviewItem) -> Result<PathBuf> {
    let caption_path = saved_path.with_extension("txt");
    let caption = build_caption(item);
    fs::write(&caption_path, caption)
        .await
        .with_context(|| format!("could not write {}", caption_path.display()))?;
    Ok(caption_path)
}

fn build_caption(item: &PreviewItem) -> String {
    let mut parts = Vec::new();
    push_caption_part(&mut parts, &item.title);
    if let Some(creator) = &item.creator {
        push_caption_part(&mut parts, creator);
    }
    push_caption_part(&mut parts, &item.source_label);
    if let Some(license) = &item.license {
        push_caption_part(&mut parts, license);
    }

    if parts.is_empty() {
        format!("{} reference", item.kind.to_ascii_lowercase())
    } else {
        parts.join(", ")
    }
}

fn push_caption_part(parts: &mut Vec<String>, raw: &str) {
    let cleaned = clean_caption_text(raw);
    if !cleaned.is_empty() && !parts.iter().any(|part| part.eq_ignore_ascii_case(&cleaned)) {
        parts.push(cleaned);
    }
}

fn clean_caption_text(raw: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for ch in raw.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }

    output
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn infer_extension(item: &PreviewItem, content_type: Option<&str>) -> String {
    if let Some(ext) = extension_from_url(&item.media_url) {
        return ext;
    }
    if let Some(ext) = extension_from_url(&item.source_page_url) {
        return ext;
    }
    if let Some(ext) = extension_from_content_type(content_type) {
        return ext.to_string();
    }

    match item.kind.as_str() {
        "Image" => "jpg".to_string(),
        "Audio" => "wav".to_string(),
        "Video" => "mp4".to_string(),
        _ => "bin".to_string(),
    }
}

fn extension_from_url(url: &str) -> Option<String> {
    let path = url.split('?').next().unwrap_or(url);
    let ext = Path::new(path).extension()?.to_string_lossy().to_string();
    let ext = ext.trim().trim_start_matches('.').to_ascii_lowercase();
    if ext.is_empty() || ext.len() > 6 || !ext.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        None
    } else {
        Some(ext)
    }
}

fn extension_from_content_type(content_type: Option<&str>) -> Option<&'static str> {
    let content_type = content_type?.to_ascii_lowercase();
    Some(match content_type.split(';').next()?.trim() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "audio/mpeg" => "mp3",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/flac" => "flac",
        "audio/ogg" | "audio/vorbis" => "ogg",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        _ => return None,
    })
}

fn slugify(input: &str) -> String {
    let slug = input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|chunk| !chunk.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if slug.is_empty() {
        "item".to_string()
    } else {
        slug
    }
}

fn truncate_slug(slug: &str, max_len: usize) -> String {
    if slug.len() <= max_len {
        slug.to_string()
    } else {
        slug[..max_len].trim_end_matches('-').to_string()
    }
}

#[derive(Serialize)]
struct DatasetManifest {
    dataset_name: String,
    dataset_slug: String,
    created_unix_seconds: u64,
    saved_items: usize,
    failed_items: usize,
    notes: Vec<String>,
    items: Vec<ManifestItem>,
}

#[derive(Serialize)]
struct ManifestItem {
    key: String,
    title: String,
    source_label: String,
    source_page_url: String,
    media_url: String,
    license: Option<String>,
    creator: Option<String>,
    page_number: u32,
    kind: String,
    saved_path: String,
    caption_path: Option<String>,
}
