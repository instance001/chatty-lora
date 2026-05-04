use std::{collections::BTreeMap, fs};

use anyhow::{Context, Result, anyhow};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::Deserialize;

use crate::{
    state::ProjectPaths,
    types::{
        GenericGalleryProfile, SourceEntry, SourceFixAppliedHistoryItem,
        SourceFixApplyPreviewRequest, SourceFixApplyPreviewResponse, SourceFixApplyRequest,
        SourceFixApplyResponse, SourceFixOpenRequest, SourceFixOpenResponse,
        SourceFixProposalHistoryItem, SourceFixProposalResponse, SourceFixProposalSaveRequest,
        SourceFixProposalSaveResponse, SourceFixProposeRequest, SourceFixSaveRequest,
        SourceFixSaveResponse, SourceFixSummary, SourceRegistryUpdateRequest,
    },
};

use super::{adapters, registry};

const DEFAULT_GENERIC_PROFILE_JSON: &str = r#"{
  "item_selector": ".gallery-card",
  "media_selector": "img",
  "media_attribute": "src",
  "title_selector": "img",
  "title_attribute": "alt",
  "thumbnail_selector": "img",
  "thumbnail_attribute": "src",
  "link_selector": "a",
  "link_attribute": "href",
  "thumbnail_url_template": "https://example.com/thumbs/{basename}",
  "title_template": "{title}",
  "source_page_url_template": "{source_page_url}"
}"#;

#[derive(Debug, Default, Clone)]
struct GenericSiteInspection {
    analysis_points: Vec<String>,
    profile_json: Option<String>,
}

pub fn summaries(paths: &ProjectPaths) -> Result<Vec<SourceFixSummary>> {
    let sources = registry::load_sources(paths)?;
    let mut items = sources
        .into_iter()
        .map(|source| {
            let note_path = note_path(paths, &source.id);
            SourceFixSummary {
                source_id: source.id.clone(),
                source_name: source.name,
                adapter_kind: source.adapter_kind.clone(),
                adapter_ready: adapters::adapter_ready(&source.adapter_kind),
                adapter_file_path: adapters::adapter_file_path(&source.adapter_kind).to_string(),
                note_relative_path: relative_note_path(&note_path, &paths.root),
                note_present: note_path.exists(),
            }
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        left.source_name
            .to_ascii_lowercase()
            .cmp(&right.source_name.to_ascii_lowercase())
    });
    Ok(items)
}

pub fn open_shell(
    paths: &ProjectPaths,
    request: SourceFixOpenRequest,
) -> Result<SourceFixOpenResponse> {
    let source = registry::load_sources(paths)?
        .into_iter()
        .find(|source| source.id == request.source_id)
        .ok_or_else(|| anyhow!("unknown source id: {}", request.source_id))?;

    let note_path = note_path(paths, &source.id);
    let existing_note = if note_path.exists() {
        fs::read_to_string(&note_path)
            .with_context(|| format!("could not read {}", note_path.display()))?
    } else {
        String::new()
    };

    Ok(SourceFixOpenResponse {
        source_id: source.id.clone(),
        source_name: source.name.clone(),
        adapter_kind: source.adapter_kind.clone(),
        adapter_ready: adapters::adapter_ready(&source.adapter_kind),
        adapter_file_path: adapters::adapter_file_path(&source.adapter_kind).to_string(),
        note_relative_path: relative_note_path(&note_path, &paths.root),
        existing_note,
        scope_note: scope_note(&source.adapter_kind),
        starter_steps: starter_steps(&source.name, &source.adapter_kind),
        proposal_history: proposal_history(paths, &source.id)?,
        apply_history: apply_history(paths, &source.id)?,
    })
}

pub fn save_shell(
    paths: &ProjectPaths,
    request: SourceFixSaveRequest,
) -> Result<SourceFixSaveResponse> {
    let source = registry::load_sources(paths)?
        .into_iter()
        .find(|source| source.id == request.source_id)
        .ok_or_else(|| anyhow!("unknown source id: {}", request.source_id))?;

    fs::create_dir_all(&paths.site_fix_notes)
        .with_context(|| format!("could not create {}", paths.site_fix_notes.display()))?;
    let note_path = note_path(paths, &source.id);
    let note_contents = format!(
        "# Site Fix Brief: {}\n\n## Source ID\n{}\n\n## Adapter Kind\n{}\n\n## Adapter File Scope\n{}\n\n## Issue Summary\n{}\n\n## Reproduction Notes\n{}\n\n## Patch Notes\n{}\n",
        source.name,
        source.id,
        source.adapter_kind,
        adapters::adapter_file_path(&source.adapter_kind),
        request.issue_summary.trim(),
        request.reproduction_notes.trim(),
        request.patch_notes.trim(),
    );

    fs::write(&note_path, note_contents)
        .with_context(|| format!("could not write {}", note_path.display()))?;

    Ok(SourceFixSaveResponse {
        source_id: source.id,
        saved_relative_path: relative_note_path(&note_path, &paths.root),
        notes: vec![
            "Scoped site-fix brief saved locally.".to_string(),
            "This note is for the selected source only and should not be used to rewrite crawler core logic."
                .to_string(),
        ],
    })
}

pub async fn propose_fix(
    client: &Client,
    paths: &ProjectPaths,
    request: SourceFixProposeRequest,
) -> Result<SourceFixProposalResponse> {
    let source = registry::load_sources(paths)?
        .into_iter()
        .find(|source| source.id == request.source_id)
        .ok_or_else(|| anyhow!("unknown source id: {}", request.source_id))?;

    let adapter_rel = adapters::adapter_file_path(&source.adapter_kind);
    let adapter_path = paths.root.join(adapter_rel);
    let adapter_source = fs::read_to_string(&adapter_path).unwrap_or_default();
    let profile = classify_issue(
        &source.adapter_kind,
        &request.issue_summary,
        &request.reproduction_notes,
        &request.patch_notes,
    );
    let touched_symbols = target_symbols(&source.adapter_kind, &adapter_source, profile.key);

    let mut analysis_points = vec![
        format!(
            "Keep the patch scoped to {} and leave the shared crawl engine untouched.",
            adapter_rel
        ),
        format!("The current issue reads most like {}.", profile.description),
        format!(
            "Most relevant touch points in this adapter: {}.",
            if touched_symbols.is_empty() {
                "search entrypoint and the preview item mapping".to_string()
            } else {
                touched_symbols.join(", ")
            }
        ),
        if request.patch_notes.trim().is_empty() {
            "No manual patch notes were provided yet, so this draft leans on the issue summary and reproduction notes."
                .to_string()
        } else {
            "The manual patch notes were folded in as hints, but this is still a review draft rather than an applied edit."
                .to_string()
        },
    ];

    let site_inspection = if source.adapter_kind == "generic_gallery_html" {
        match inspect_generic_gallery_source(client, &source, &request).await {
            Ok(inspection) => inspection,
            Err(error) => GenericSiteInspection {
                analysis_points: vec![format!(
                    "Automatic site inspection could not complete: {}. The manual profile draft is still available below.",
                    friendly_site_probe_error(&error.to_string())
                )],
                profile_json: None,
            },
        }
    } else {
        GenericSiteInspection::default()
    };
    analysis_points.extend(site_inspection.analysis_points.clone());

    let review_checklist = if source.adapter_kind == "generic_gallery_html" {
        vec![
            "Confirm the proposal only updates this source's URL template and/or selector profile in config/sources.json."
                .to_string(),
            "Re-run one concrete failing search term before expanding the fix.".to_string(),
            "Check that the connection fix returns real media URLs without changing crawl pacing or dataset naming."
                .to_string(),
        ]
    } else {
        vec![
            "Confirm the proposal only touches the selected adapter file.".to_string(),
            "Re-run one concrete failing search term before expanding the fix.".to_string(),
            "Check that the patch improves previews without changing crawl pacing or dataset naming."
                .to_string(),
        ]
    };

    Ok(SourceFixProposalResponse {
        source_id: source.id,
        source_name: source.name,
        adapter_file_path: adapter_rel.to_string(),
        proposal_title: format!("Scoped adapter fix draft for {}", profile.short_label),
        confidence_label: profile.confidence.to_string(),
        analysis_points,
        proposed_patch: build_patch_sketch(
            &source.adapter_kind,
            adapter_rel,
            &request,
            profile.key,
            &touched_symbols,
            site_inspection.profile_json.as_deref(),
        ),
        review_checklist,
    })
}

pub fn save_proposal_snapshot(
    paths: &ProjectPaths,
    request: SourceFixProposalSaveRequest,
) -> Result<SourceFixProposalSaveResponse> {
    let source = registry::load_sources(paths)?
        .into_iter()
        .find(|source| source.id == request.source_id)
        .ok_or_else(|| anyhow!("unknown source id: {}", request.source_id))?;

    let proposal_dir = proposal_dir(paths, &source.id);
    fs::create_dir_all(&proposal_dir)
        .with_context(|| format!("could not create {}", proposal_dir.display()))?;

    let saved_unix_seconds = unix_seconds();
    let proposal_slug = slugify(&request.proposal_title);
    let proposal_path = proposal_dir.join(format!(
        "{}-{}.md",
        saved_unix_seconds,
        if proposal_slug.is_empty() {
            "scoped-fix-proposal".to_string()
        } else {
            proposal_slug
        }
    ));

    let analysis = if request.analysis_points.is_empty() {
        "- No analysis points were supplied.\n".to_string()
    } else {
        request
            .analysis_points
            .iter()
            .map(|item| format!("- {}\n", item.trim()))
            .collect::<String>()
    };
    let checklist = if request.review_checklist.is_empty() {
        "- No checklist items were supplied.\n".to_string()
    } else {
        request
            .review_checklist
            .iter()
            .map(|item| format!("- {}\n", item.trim()))
            .collect::<String>()
    };

    let contents = format!(
        "# Scoped Proposal: {title}\n\n## Source ID\n{source_id}\n\n## Source Name\n{source_name}\n\n## Confidence\n{confidence}\n\n## Saved Unix Seconds\n{saved_unix_seconds}\n\n## Analysis\n{analysis}\n\n## Proposed Patch\n{patch}\n\n## Review Checklist\n{checklist}",
        title = request.proposal_title.trim(),
        source_id = source.id,
        source_name = source.name,
        confidence = request.confidence_label.trim(),
        saved_unix_seconds = saved_unix_seconds,
        analysis = analysis.trim_end(),
        patch = request.proposed_patch.trim(),
        checklist = checklist.trim_end(),
    );

    fs::write(&proposal_path, contents)
        .with_context(|| format!("could not write {}", proposal_path.display()))?;

    Ok(SourceFixProposalSaveResponse {
        source_id: request.source_id,
        saved_relative_path: relative_note_path(&proposal_path, &paths.root),
        notes: vec![
            "Scoped proposal snapshot saved locally.".to_string(),
            "This is a review artifact only. It does not edit the adapter file.".to_string(),
        ],
    })
}

pub fn preview_apply(
    paths: &ProjectPaths,
    request: SourceFixApplyPreviewRequest,
) -> Result<SourceFixApplyPreviewResponse> {
    let source = registry::load_sources(paths)?
        .into_iter()
        .find(|source| source.id == request.source_id)
        .ok_or_else(|| anyhow!("unknown source id: {}", request.source_id))?;

    if source.adapter_kind == "generic_gallery_html" {
        return preview_generic_gallery_profile_apply(paths, &source, &request);
    }

    let adapter_rel = adapters::adapter_file_path(&source.adapter_kind);
    let adapter_path = paths.root.join(adapter_rel);
    let original = fs::read_to_string(&adapter_path)
        .with_context(|| format!("could not read {}", adapter_path.display()))?;

    let profile = classify_issue(
        &source.adapter_kind,
        &request.issue_summary,
        &request.reproduction_notes,
        &request.patch_notes,
    );
    let (updated, apply_notes) =
        build_applied_source(&source.adapter_kind, profile.key, &original)?;
    if updated == original {
        return Err(anyhow!(
            "no scoped adapter change was generated for {} yet",
            adapter_rel
        ));
    }

    let preview = build_preview_bundle(
        &original,
        &updated,
        preview_backup_relative_path(paths, &source.id, adapter_rel),
    );

    Ok(SourceFixApplyPreviewResponse {
        source_id: source.id,
        source_name: source.name,
        adapter_file_path: adapter_rel.to_string(),
        backup_relative_path: preview.backup_relative_path,
        review_title: format!("Scoped apply review for {}", profile.short_label),
        apply_notes,
        diff_lines: preview.diff_lines,
        before_excerpt: preview.before_excerpt,
        after_excerpt: preview.after_excerpt,
    })
}

pub fn apply_fix(
    paths: &ProjectPaths,
    request: SourceFixApplyRequest,
) -> Result<SourceFixApplyResponse> {
    let source = registry::load_sources(paths)?
        .into_iter()
        .find(|source| source.id == request.source_id)
        .ok_or_else(|| anyhow!("unknown source id: {}", request.source_id))?;

    if source.adapter_kind == "generic_gallery_html" {
        return apply_generic_gallery_profile(paths, &source, &request);
    }

    let adapter_rel = adapters::adapter_file_path(&source.adapter_kind);
    let adapter_path = paths.root.join(adapter_rel);
    let original = fs::read_to_string(&adapter_path)
        .with_context(|| format!("could not read {}", adapter_path.display()))?;

    let profile = classify_issue(
        &source.adapter_kind,
        &request.issue_summary,
        &request.reproduction_notes,
        &request.patch_notes,
    );
    let (updated, mut apply_notes) =
        build_applied_source(&source.adapter_kind, profile.key, &original)?;
    if updated == original {
        return Err(anyhow!(
            "no scoped adapter change was generated for {} yet",
            adapter_rel
        ));
    }

    let backup_path = write_backup(paths, &source.id, adapter_rel, &original)?;
    fs::write(&adapter_path, updated)
        .with_context(|| format!("could not write {}", adapter_path.display()))?;
    let apply_record_path = save_apply_record(
        paths,
        &source.id,
        &source.name,
        adapter_rel,
        profile.short_label,
        &request,
        &backup_path,
    )?;

    apply_notes.push("Backup written before the adapter file was changed.".to_string());
    apply_notes.push("Only the selected adapter file was touched.".to_string());
    apply_notes.push("A per-source apply record was saved for later review.".to_string());

    Ok(SourceFixApplyResponse {
        source_id: source.id,
        source_name: source.name,
        adapter_file_path: adapter_rel.to_string(),
        applied_relative_path: relative_note_path(&adapter_path, &paths.root),
        backup_relative_path: relative_note_path(&backup_path, &paths.root),
        notes: {
            apply_notes.push(format!(
                "Apply record: {}",
                relative_note_path(&apply_record_path, &paths.root)
            ));
            apply_notes
        },
    })
}

fn note_path(paths: &ProjectPaths, source_id: &str) -> std::path::PathBuf {
    paths.site_fix_notes.join(format!("{}.md", source_id))
}

fn proposal_dir(paths: &ProjectPaths, source_id: &str) -> std::path::PathBuf {
    paths.site_fix_notes.join("proposals").join(source_id)
}

fn apply_dir(paths: &ProjectPaths, source_id: &str) -> std::path::PathBuf {
    paths.site_fix_notes.join("applied").join(source_id)
}

fn proposal_history(
    paths: &ProjectPaths,
    source_id: &str,
) -> Result<Vec<SourceFixProposalHistoryItem>> {
    collect_history_items(&proposal_dir(paths, source_id), &paths.root).map(|items| {
        items
            .into_iter()
            .map(|item| SourceFixProposalHistoryItem {
                title: item.title,
                relative_path: item.relative_path,
                saved_unix_seconds: item.saved_unix_seconds,
            })
            .collect()
    })
}

fn apply_history(
    paths: &ProjectPaths,
    source_id: &str,
) -> Result<Vec<SourceFixAppliedHistoryItem>> {
    collect_history_items(&apply_dir(paths, source_id), &paths.root).map(|items| {
        items
            .into_iter()
            .map(|item| SourceFixAppliedHistoryItem {
                title: item.title,
                relative_path: item.relative_path,
                saved_unix_seconds: item.saved_unix_seconds,
            })
            .collect()
    })
}

struct HistoryItem {
    title: String,
    relative_path: String,
    saved_unix_seconds: u64,
}

fn collect_history_items(
    dir: &std::path::Path,
    root: &std::path::Path,
) -> Result<Vec<HistoryItem>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("could not read {}", dir.display()))? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let relative_path = relative_note_path(&path, root);
        let file_name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        let (saved_unix_seconds, fallback_title) = match file_name.split_once('-') {
            Some((stamp, title)) => (stamp.parse::<u64>().unwrap_or(0), title.replace('-', " ")),
            None => (0, file_name.replace('-', " ")),
        };

        let title = fs::read_to_string(&path)
            .ok()
            .and_then(|contents| extract_markdown_heading(&contents))
            .unwrap_or_else(|| {
                if fallback_title.is_empty() {
                    "Scoped history item".to_string()
                } else {
                    fallback_title
                }
            });

        items.push(HistoryItem {
            title,
            relative_path,
            saved_unix_seconds,
        });
    }

    items.sort_by(|left, right| {
        right
            .saved_unix_seconds
            .cmp(&left.saved_unix_seconds)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });

    Ok(items)
}

struct PreviewBundle {
    backup_relative_path: String,
    diff_lines: Vec<String>,
    before_excerpt: String,
    after_excerpt: String,
}

fn build_preview_bundle(
    original: &str,
    updated: &str,
    backup_relative_path: String,
) -> PreviewBundle {
    let diff_lines = diff_summary(original, updated, 48);
    let (before_excerpt, after_excerpt) = excerpt_pair(original, updated, 4);
    PreviewBundle {
        backup_relative_path,
        diff_lines,
        before_excerpt,
        after_excerpt,
    }
}

fn preview_backup_relative_path(
    paths: &ProjectPaths,
    source_id: &str,
    adapter_rel: &str,
) -> String {
    relative_note_path(
        &backup_path(paths, source_id, adapter_rel, unix_seconds()),
        &paths.root,
    )
}

fn backup_path(
    paths: &ProjectPaths,
    source_id: &str,
    adapter_rel: &str,
    timestamp: u64,
) -> std::path::PathBuf {
    let adapter_name = std::path::Path::new(adapter_rel)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("adapter.rs");
    paths
        .site_fix_notes
        .join("backups")
        .join(source_id)
        .join(format!("{}-{}.bak", timestamp, adapter_name))
}

fn write_backup(
    paths: &ProjectPaths,
    source_id: &str,
    adapter_rel: &str,
    original: &str,
) -> Result<std::path::PathBuf> {
    let backup_path = backup_path(paths, source_id, adapter_rel, unix_seconds());
    if let Some(parent) = backup_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("could not create {}", parent.display()))?;
    }
    fs::write(&backup_path, original)
        .with_context(|| format!("could not write {}", backup_path.display()))?;
    Ok(backup_path)
}

fn save_apply_record(
    paths: &ProjectPaths,
    source_id: &str,
    source_name: &str,
    adapter_rel: &str,
    short_label: &str,
    request: &SourceFixApplyRequest,
    backup_path: &std::path::Path,
) -> Result<std::path::PathBuf> {
    let apply_dir = apply_dir(paths, source_id);
    fs::create_dir_all(&apply_dir)
        .with_context(|| format!("could not create {}", apply_dir.display()))?;

    let timestamp = unix_seconds();
    let title = format!("Applied scoped adapter fix for {}", short_label);
    let record_path = apply_dir.join(format!("{}-{}.md", timestamp, slugify(&title)));
    let contents = format!(
        "# {title}\n\n## Source ID\n{source_id}\n\n## Source Name\n{source_name}\n\n## Adapter File\n{adapter_rel}\n\n## Backup Path\n{backup_rel}\n\n## Saved Unix Seconds\n{timestamp}\n\n## Issue Summary\n{issue_summary}\n\n## Reproduction Notes\n{reproduction_notes}\n\n## Patch Notes\n{patch_notes}\n",
        title = title,
        source_id = source_id,
        source_name = source_name,
        adapter_rel = adapter_rel,
        backup_rel = relative_note_path(backup_path, &paths.root),
        timestamp = timestamp,
        issue_summary = request.issue_summary.trim(),
        reproduction_notes = request.reproduction_notes.trim(),
        patch_notes = request.patch_notes.trim(),
    );

    fs::write(&record_path, contents)
        .with_context(|| format!("could not write {}", record_path.display()))?;
    Ok(record_path)
}

fn build_applied_source(
    adapter_kind: &str,
    issue_key: &str,
    original: &str,
) -> Result<(String, Vec<String>)> {
    match adapter_kind {
        "openverse_images" | "openverse_audio" => apply_openverse_patch(issue_key, original),
        "wikimedia_commons" => apply_wikimedia_patch(issue_key, original),
        "generic_gallery_html" => Err(anyhow!(
            "the generic gallery adapter is searchable now, but automatic site-specific patch application is not available for it yet"
        )),
        other => Err(anyhow!(
            "no scoped auto-apply path exists for adapter kind {}",
            other
        )),
    }
}

fn preview_generic_gallery_profile_apply(
    paths: &ProjectPaths,
    source: &SourceEntry,
    request: &SourceFixApplyPreviewRequest,
) -> Result<SourceFixApplyPreviewResponse> {
    let connection_fix = build_generic_gallery_connection_fix(
        &request.issue_summary,
        &request.reproduction_notes,
        &request.patch_notes,
    )?;
    let (original, updated) =
        source_registry_preview_with_profile(paths, &source.id, connection_fix.clone())?;
    if updated == original {
        return Err(anyhow!(
            "the generated source profile did not change config/sources.json"
        ));
    }

    let target_rel = "config/sources.json";
    let preview = build_preview_bundle(
        &original,
        &updated,
        preview_backup_relative_path(paths, &source.id, target_rel),
    );

    Ok(SourceFixApplyPreviewResponse {
        source_id: source.id.clone(),
        source_name: source.name.clone(),
        adapter_file_path: target_rel.to_string(),
        backup_relative_path: preview.backup_relative_path,
        review_title: "Scoped source connection profile review".to_string(),
        apply_notes: generic_connection_apply_notes(&connection_fix),
        diff_lines: preview.diff_lines,
        before_excerpt: preview.before_excerpt,
        after_excerpt: preview.after_excerpt,
    })
}

fn apply_generic_gallery_profile(
    paths: &ProjectPaths,
    source: &SourceEntry,
    request: &SourceFixApplyRequest,
) -> Result<SourceFixApplyResponse> {
    let connection_fix = build_generic_gallery_connection_fix(
        &request.issue_summary,
        &request.reproduction_notes,
        &request.patch_notes,
    )?;
    let (sources, original, updated) =
        source_registry_update_with_connection_fix(paths, &source.id, connection_fix.clone())?;
    if updated == original {
        return Err(anyhow!(
            "the generated source profile did not change config/sources.json"
        ));
    }

    let target_rel = "config/sources.json";
    let backup_path = write_backup(paths, &source.id, target_rel, &original)?;
    registry::save_sources(paths, SourceRegistryUpdateRequest { sources })?;
    let apply_record_path = save_apply_record(
        paths,
        &source.id,
        &source.name,
        target_rel,
        "generic gallery source profile",
        request,
        &backup_path,
    )?;

    let mut notes = generic_connection_apply_notes(&connection_fix);
    notes.push("Backup written before config/sources.json changed.".to_string());
    notes.push("No Rust adapter file or crawler-core code was edited.".to_string());
    notes.push(format!(
        "Apply record: {}",
        relative_note_path(&apply_record_path, &paths.root)
    ));

    Ok(SourceFixApplyResponse {
        source_id: source.id.clone(),
        source_name: source.name.clone(),
        adapter_file_path: target_rel.to_string(),
        applied_relative_path: target_rel.to_string(),
        backup_relative_path: relative_note_path(&backup_path, &paths.root),
        notes,
    })
}

fn source_registry_preview_with_profile(
    paths: &ProjectPaths,
    source_id: &str,
    connection_fix: GenericGalleryConnectionFix,
) -> Result<(String, String)> {
    let (_sources, original, updated) =
        source_registry_update_with_connection_fix(paths, source_id, connection_fix)?;
    Ok((original, updated))
}

fn source_registry_update_with_connection_fix(
    paths: &ProjectPaths,
    source_id: &str,
    connection_fix: GenericGalleryConnectionFix,
) -> Result<(Vec<SourceEntry>, String, String)> {
    let mut sources = registry::load_sources(paths)?;
    let original = serde_json::to_string_pretty(&sources)
        .context("could not serialize current source registry")?;
    let source = sources
        .iter_mut()
        .find(|source| source.id == source_id)
        .ok_or_else(|| anyhow!("unknown source id: {}", source_id))?;
    if let Some(base_url_template) = connection_fix.base_url_template {
        source.base_url = base_url_template;
    }
    if let Some(profile) = connection_fix.profile {
        source.site_profile = Some(profile);
    }
    source.enabled = true;
    if !source.notes.contains("Source-specific connection fix") {
        source.notes = format!(
            "{} Source-specific connection fix is active.",
            source.notes.trim()
        )
        .trim()
        .to_string();
    }
    let updated = serde_json::to_string_pretty(&sources)
        .context("could not serialize updated source registry")?;
    Ok((sources, original, updated))
}

#[derive(Debug, Clone)]
struct GenericGalleryConnectionFix {
    profile: Option<GenericGalleryProfile>,
    base_url_template: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct GenericGalleryProfileDraft {
    base_url_template: Option<String>,
    item_selector: Option<String>,
    media_selector: Option<String>,
    media_attribute: Option<String>,
    title_selector: Option<String>,
    title_attribute: Option<String>,
    thumbnail_selector: Option<String>,
    thumbnail_attribute: Option<String>,
    link_selector: Option<String>,
    link_attribute: Option<String>,
    media_url_template: Option<String>,
    thumbnail_url_template: Option<String>,
    title_template: Option<String>,
    source_page_url_template: Option<String>,
}

fn build_generic_gallery_connection_fix(
    issue_summary: &str,
    reproduction_notes: &str,
    patch_notes: &str,
) -> Result<GenericGalleryConnectionFix> {
    let draft = generic_gallery_profile_draft(issue_summary, reproduction_notes, patch_notes)
        .ok_or_else(|| {
            anyhow!(
                "generic gallery apply needs a source profile or URL template in Patch notes. Paste JSON with media_selector and/or base_url_template."
            )
        })?;

    let base_url_template = clean_optional_field(draft.base_url_template.clone())
        .map(|value| validate_url_template("base_url_template", &value))
        .transpose()?;
    let has_profile = draft
        .media_selector
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let profile = if has_profile {
        Some(generic_gallery_profile_from_draft(draft)?)
    } else {
        None
    };

    if profile.is_none() && base_url_template.is_none() {
        return Err(anyhow!(
            "generic gallery apply needs at least media_selector or base_url_template in Patch notes."
        ));
    }

    Ok(GenericGalleryConnectionFix {
        profile,
        base_url_template,
    })
}

fn generic_gallery_profile_draft(
    issue_summary: &str,
    reproduction_notes: &str,
    patch_notes: &str,
) -> Option<GenericGalleryProfileDraft> {
    let combined = format!(
        "{}\n{}\n{}",
        issue_summary.trim(),
        reproduction_notes.trim(),
        patch_notes.trim()
    );

    let draft = extract_json_object(&combined)
        .and_then(|json| serde_json::from_str::<GenericGalleryProfileDraft>(&json).ok())
        .or_else(|| parse_profile_key_values(&combined));

    draft
}

fn generic_gallery_profile_from_draft(
    draft: GenericGalleryProfileDraft,
) -> Result<GenericGalleryProfile> {
    let media_selector = clean_required_field(draft.media_selector, "media_selector")?;
    validate_selector("media_selector", &media_selector)?;
    let media_attribute = clean_optional_field(draft.media_attribute)
        .unwrap_or_else(|| infer_media_attribute(&media_selector).to_string());
    validate_attribute("media_attribute", &media_attribute)?;

    validate_optional_selector("item_selector", draft.item_selector.as_deref())?;
    validate_optional_selector("title_selector", draft.title_selector.as_deref())?;
    validate_optional_selector("thumbnail_selector", draft.thumbnail_selector.as_deref())?;
    validate_optional_selector("link_selector", draft.link_selector.as_deref())?;

    let title_attribute = clean_optional_attribute("title_attribute", draft.title_attribute)?;
    let thumbnail_attribute =
        clean_optional_attribute("thumbnail_attribute", draft.thumbnail_attribute)?;
    let link_attribute = clean_optional_attribute("link_attribute", draft.link_attribute)?
        .or_else(|| draft.link_selector.as_ref().map(|_| "href".to_string()));
    let media_url_template =
        clean_optional_template("media_url_template", draft.media_url_template)?;
    let thumbnail_url_template =
        clean_optional_template("thumbnail_url_template", draft.thumbnail_url_template)?;
    let title_template = clean_optional_template("title_template", draft.title_template)?;
    let source_page_url_template =
        clean_optional_template("source_page_url_template", draft.source_page_url_template)?;

    Ok(GenericGalleryProfile {
        item_selector: clean_optional_field(draft.item_selector),
        media_selector,
        media_attribute,
        title_selector: clean_optional_field(draft.title_selector),
        title_attribute,
        thumbnail_selector: clean_optional_field(draft.thumbnail_selector),
        thumbnail_attribute,
        link_selector: clean_optional_field(draft.link_selector),
        link_attribute,
        media_url_template,
        thumbnail_url_template,
        title_template,
        source_page_url_template,
    })
}

fn extract_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let mut depth = 0usize;
    for (offset, ch) in text[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(text[start..=start + offset].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_profile_key_values(text: &str) -> Option<GenericGalleryProfileDraft> {
    let mut values = BTreeMap::new();
    for line in text.lines() {
        let trimmed = line
            .trim()
            .trim_start_matches('-')
            .trim()
            .trim_matches('`')
            .trim();
        let Some((key, value)) = trimmed.split_once(':').or_else(|| trimmed.split_once('=')) else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase().replace('-', "_");
        let value = value
            .trim()
            .trim_matches(',')
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        if !value.is_empty() {
            values.insert(key, value);
        }
    }

    if values.is_empty() {
        return None;
    }

    Some(GenericGalleryProfileDraft {
        base_url_template: remove_first(
            &mut values,
            &[
                "base_url_template",
                "url_template",
                "source_url_template",
                "source_url",
                "base_url",
                "search_url",
            ],
        ),
        item_selector: remove_first(
            &mut values,
            &["item_selector", "card_selector", "container_selector"],
        ),
        media_selector: remove_first(
            &mut values,
            &[
                "media_selector",
                "file_selector",
                "image_selector",
                "video_selector",
                "audio_selector",
            ],
        ),
        media_attribute: remove_first(
            &mut values,
            &[
                "media_attribute",
                "media_attr",
                "file_attribute",
                "image_attribute",
                "src_attribute",
            ],
        ),
        title_selector: remove_first(
            &mut values,
            &["title_selector", "caption_selector", "name_selector"],
        ),
        title_attribute: remove_first(
            &mut values,
            &["title_attribute", "title_attr", "caption_attribute"],
        ),
        thumbnail_selector: remove_first(
            &mut values,
            &["thumbnail_selector", "thumb_selector", "preview_selector"],
        ),
        thumbnail_attribute: remove_first(
            &mut values,
            &[
                "thumbnail_attribute",
                "thumbnail_attr",
                "thumb_attribute",
                "thumb_attr",
            ],
        ),
        link_selector: remove_first(
            &mut values,
            &["link_selector", "page_selector", "source_page_selector"],
        ),
        link_attribute: remove_first(
            &mut values,
            &["link_attribute", "link_attr", "page_attribute"],
        ),
        media_url_template: remove_first(
            &mut values,
            &[
                "media_url_template",
                "media_template",
                "download_url_template",
            ],
        ),
        thumbnail_url_template: remove_first(
            &mut values,
            &[
                "thumbnail_url_template",
                "thumb_url_template",
                "poster_url_template",
            ],
        ),
        title_template: remove_first(&mut values, &["title_template", "name_template"]),
        source_page_url_template: remove_first(
            &mut values,
            &[
                "source_page_url_template",
                "detail_url_template",
                "page_url_template",
            ],
        ),
    })
}

fn remove_first(values: &mut BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| values.remove(*key))
}

fn clean_required_field(value: Option<String>, label: &str) -> Result<String> {
    clean_optional_field(value)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{} is required for a generic gallery source profile", label))
}

fn clean_optional_field(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn clean_optional_attribute(label: &str, value: Option<String>) -> Result<Option<String>> {
    let Some(value) = clean_optional_field(value) else {
        return Ok(None);
    };
    validate_attribute(label, &value)?;
    Ok(Some(value))
}

fn clean_optional_template(label: &str, value: Option<String>) -> Result<Option<String>> {
    let Some(value) = clean_optional_field(value) else {
        return Ok(None);
    };
    validate_template(label, &value)?;
    Ok(Some(value))
}

fn validate_optional_selector(label: &str, value: Option<&str>) -> Result<()> {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        validate_selector(label, value)?;
    }
    Ok(())
}

fn validate_selector(label: &str, value: &str) -> Result<()> {
    Selector::parse(value).map(|_| ()).map_err(|_| {
        anyhow!(
            "{} must be a valid CSS selector. \"{}\" could not be parsed.",
            label,
            value
        )
    })
}

fn validate_attribute(label: &str, value: &str) -> Result<()> {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':'))
        || value.eq_ignore_ascii_case("text")
    {
        Ok(())
    } else {
        Err(anyhow!(
            "{} must be a simple attribute name such as src, href, data-src, srcset, poster, or text.",
            label
        ))
    }
}

fn validate_url_template(label: &str, value: &str) -> Result<String> {
    let cleaned = value.trim().to_string();
    if cleaned.is_empty() {
        return Err(anyhow!("{} cannot be empty.", label));
    }
    let probe_url = cleaned.replace("{query}", "sample").replace("{page}", "1");
    Url::parse(&probe_url).map_err(|_| {
        anyhow!(
            "{} must be a valid absolute URL template, for example https://example.com/gallery/page/{{page}}.",
            label
        )
    })?;
    Ok(cleaned)
}

fn validate_template(label: &str, value: &str) -> Result<()> {
    let cleaned = value.trim();
    if cleaned.is_empty() {
        return Err(anyhow!("{} cannot be empty.", label));
    }
    if cleaned.contains('\n') || cleaned.contains('\r') {
        return Err(anyhow!(
            "{} must stay on one line so it can be stored safely in the source profile.",
            label
        ));
    }
    Ok(())
}

fn generic_connection_apply_notes(connection_fix: &GenericGalleryConnectionFix) -> Vec<String> {
    let mut notes = Vec::new();
    if connection_fix.base_url_template.is_some() {
        notes.push(
            "This updates the selected source URL/template in config/sources.json.".to_string(),
        );
    }
    if connection_fix.profile.is_some() {
        notes.push(
            "This applies a validated selector profile to this one source entry.".to_string(),
        );
        notes.push("The generic gallery adapter will use this profile on the next search before falling back to broad HTML scanning.".to_string());
    }
    notes.push("No Rust adapter file or crawler-core code will be edited.".to_string());
    notes
}

fn infer_media_attribute(media_selector: &str) -> &'static str {
    let selector = media_selector.to_ascii_lowercase();
    if selector.contains("a") && !selector.contains("img") && !selector.contains("video") {
        "href"
    } else if selector.contains("source")
        || selector.contains("img")
        || selector.contains("video")
        || selector.contains("audio")
    {
        "src"
    } else {
        "href"
    }
}

fn apply_openverse_patch(issue_key: &str, original: &str) -> Result<(String, Vec<String>)> {
    match issue_key {
        "pagination_drift" => {
            let old = "        has_more |= payload.page_count.map(|count| page < count).unwrap_or(false);";
            let new = "        has_more |= payload.page_count.map(|count| page < count).unwrap_or(false)\n            || payload.results.len() as u32 == PREVIEW_PAGE_SIZE;";
            if !original.contains(old) {
                return Err(anyhow!(
                    "could not find the Openverse pagination block to patch"
                ));
            }
            Ok((
                original.replacen(old, new, 1),
                vec![
                    "Added a fallback has-more check when Openverse page counts drift or disappear.".to_string(),
                    "Kept the change local to the Openverse adapter request loop.".to_string(),
                ],
            ))
        }
        "thumbnail_drift" | "media_url_drift" | "empty_results" | "metadata_drift"
        | "generic_drift" => {
            let old = r#"        let items = payload
            .results
            .into_iter()
            .map(|item| {
                let title = item
                    .title
                    .filter(|title| !title.trim().is_empty())
                    .unwrap_or_else(|| format!("Untitled {}", if image_mode { "image" } else { "audio" }));
                let media_url = item
                    .url
                    .clone()
                    .unwrap_or_else(|| item.foreign_landing_url.clone().unwrap_or_default());
                let preview_url = if image_mode {
                    None
                } else {
                    item.url.clone()
                };
                let thumb_url = if image_mode { item.thumbnail.clone() } else { None };

                PreviewItem {
                    key: format!(
                        "{}::{}::{}",
                        source.id,
                        page,
                        item.id.unwrap_or_else(|| title.clone())
                    ),
                    title,
                    thumb_url,
                    preview_url,
                    media_url: media_url.clone(),
                    source_page_url: item.foreign_landing_url.clone().unwrap_or(media_url),
                    license: item.license.map(|license| {
                        if let Some(version) = item.license_version {
                            format!("{} {}", license, version)
                        } else {
                            license
                        }
                    }),
                    creator: item.creator,
                    source_label: source.name.clone(),
                    page_number: page,
                    kind: if image_mode {
                        "Image".to_string()
                    } else {
                        "Audio".to_string()
                    },
                }
            })
            .collect();
"#;
            let new = r#"        let items = payload
            .results
            .into_iter()
            .filter_map(|item| {
                let title = item
                    .title
                    .filter(|title| !title.trim().is_empty())
                    .unwrap_or_else(|| format!("Untitled {}", if image_mode { "image" } else { "audio" }));
                let media_url = item
                    .url
                    .clone()
                    .or_else(|| item.foreign_landing_url.clone())
                    .unwrap_or_default();
                if media_url.trim().is_empty() {
                    return None;
                }
                let preview_url = if image_mode {
                    None
                } else {
                    item.url.clone().or_else(|| item.foreign_landing_url.clone())
                };
                let thumb_url = if image_mode {
                    item.thumbnail.clone().or_else(|| item.url.clone())
                } else {
                    None
                };

                Some(PreviewItem {
                    key: format!(
                        "{}::{}::{}",
                        source.id,
                        page,
                        item.id.unwrap_or_else(|| title.clone())
                    ),
                    title,
                    thumb_url,
                    preview_url,
                    media_url: media_url.clone(),
                    source_page_url: item.foreign_landing_url.clone().unwrap_or_else(|| media_url.clone()),
                    license: item.license.map(|license| {
                        if let Some(version) = item.license_version {
                            format!("{} {}", license, version)
                        } else {
                            license
                        }
                    }),
                    creator: item.creator,
                    source_label: source.name.clone(),
                    page_number: page,
                    kind: if image_mode {
                        "Image".to_string()
                    } else {
                        "Audio".to_string()
                    },
                })
            })
            .collect();
"#;
            if !original.contains(old) {
                return Err(anyhow!(
                    "could not find the Openverse preview mapping block to patch"
                ));
            }
            Ok((
                original.replacen(old, new, 1),
                vec![
                    "Hardened Openverse preview mapping so broken rows get skipped instead of poisoning the whole page.".to_string(),
                    "Added thumbnail and source-page fallbacks inside the Openverse adapter only.".to_string(),
                ],
            ))
        }
        other => Err(anyhow!(
            "no scoped Openverse patch is defined for issue key {}",
            other
        )),
    }
}

fn apply_wikimedia_patch(issue_key: &str, original: &str) -> Result<(String, Vec<String>)> {
    match issue_key {
        "pagination_drift" => {
            let original_with_continue = if original.contains("struct WikimediaResponse {\n    #[serde(default)]\n    query: Option<WikimediaQuery>,\n}") {
                original.replacen(
                    "struct WikimediaResponse {\n    #[serde(default)]\n    query: Option<WikimediaQuery>,\n}",
                    "struct WikimediaResponse {\n    #[serde(default)]\n    query: Option<WikimediaQuery>,\n    #[serde(default, rename = \"continue\")]\n    continue_token: Option<serde_json::Value>,\n}",
                    1,
                )
            } else {
                original.to_string()
            };
            let old = "        has_more |= raw_pages.len() as u32 == PREVIEW_PAGE_SIZE;";
            let new = "        has_more |= payload.continue_token.is_some() || raw_pages.len() as u32 == PREVIEW_PAGE_SIZE;";
            if !original_with_continue.contains(old) {
                return Err(anyhow!(
                    "could not find the Wikimedia pagination block to patch"
                ));
            }
            Ok((
                original_with_continue.replacen(old, new, 1),
                vec![
                    "Added a Wikimedia continue-token fallback for has-more detection.".to_string(),
                    "Kept the change local to the Wikimedia adapter response parsing.".to_string(),
                ],
            ))
        }
        "thumbnail_drift" | "media_url_drift" | "metadata_drift" | "empty_results"
        | "generic_drift" => {
            let helper_old = "#[derive(Debug, Deserialize)]\nstruct WikimediaResponse {";
            let helper_new = r#"fn clean_wikimedia_field(value: String) -> String {
    let mut cleaned = String::with_capacity(value.len());
    let mut inside_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => cleaned.push(ch),
            _ => {}
        }
    }
    cleaned
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
        .trim()
        .to_string()
}

#[derive(Debug, Deserialize)]
struct WikimediaResponse {"#;
            let with_helper = if original.contains("fn clean_wikimedia_field(") {
                original.to_string()
            } else if original.contains(helper_old) {
                original.replacen(helper_old, helper_new, 1)
            } else {
                return Err(anyhow!(
                    "could not find the Wikimedia response block to anchor a helper"
                ));
            };

            let old = r#"        let items = raw_pages
            .into_iter()
            .filter_map(|page_data| {
                let image = page_data.imageinfo.and_then(|mut info| info.drain(..).next())?;
                let ext = image.extmetadata.unwrap_or_default();
                let license = ext
                    .get("LicenseShortName")
                    .and_then(|field| field.value.clone());
                let creator = ext.get("Artist").and_then(|field| field.value.clone());

                Some(PreviewItem {
                    key: format!("{}::{}::{}", source.id, page, page_data.title),
                    title: page_data.title,
                    thumb_url: image.thumburl.or_else(|| image.url.clone()),
                    preview_url: None,
                    media_url: image.url.clone().unwrap_or_default(),
                    source_page_url: image.descriptionurl.unwrap_or_else(|| image.url.unwrap_or_default()),
                    license,
                    creator,
                    source_label: source.name.clone(),
                    page_number: page,
                    kind: "Image".to_string(),
                })
            })
            .collect();
"#;
            let new = r#"        let items = raw_pages
            .into_iter()
            .filter_map(|page_data| {
                let image = page_data.imageinfo.and_then(|mut info| info.drain(..).next())?;
                let ext = image.extmetadata.unwrap_or_default();
                let media_url = image.url.clone().unwrap_or_default();
                if media_url.trim().is_empty() {
                    return None;
                }
                let license = ext
                    .get("LicenseShortName")
                    .and_then(|field| field.value.clone())
                    .or_else(|| ext.get("UsageTerms").and_then(|field| field.value.clone()))
                    .map(clean_wikimedia_field);
                let creator = ext
                    .get("Artist")
                    .and_then(|field| field.value.clone())
                    .or_else(|| ext.get("Credit").and_then(|field| field.value.clone()))
                    .map(clean_wikimedia_field);

                Some(PreviewItem {
                    key: format!("{}::{}::{}", source.id, page, page_data.title),
                    title: page_data.title,
                    thumb_url: image.thumburl.or_else(|| image.url.clone()),
                    preview_url: None,
                    media_url: media_url.clone(),
                    source_page_url: image.descriptionurl.unwrap_or_else(|| media_url.clone()),
                    license,
                    creator,
                    source_label: source.name.clone(),
                    page_number: page,
                    kind: "Image".to_string(),
                })
            })
            .collect();
"#;
            if !with_helper.contains(old) {
                return Err(anyhow!(
                    "could not find the Wikimedia preview mapping block to patch"
                ));
            }
            Ok((
                with_helper.replacen(old, new, 1),
                vec![
                    "Hardened Wikimedia preview mapping so rows without a real media URL get skipped cleanly.".to_string(),
                    "Expanded license and creator fallbacks and stripped simple HTML markup locally in the Wikimedia adapter.".to_string(),
                ],
            ))
        }
        other => Err(anyhow!(
            "no scoped Wikimedia patch is defined for issue key {}",
            other
        )),
    }
}

fn diff_summary(original: &str, updated: &str, max_lines: usize) -> Vec<String> {
    let original_lines = original.lines().collect::<Vec<_>>();
    let updated_lines = updated.lines().collect::<Vec<_>>();
    let (start, original_end, updated_end) = differing_window(&original_lines, &updated_lines);

    let before_block = &original_lines[start..original_end];
    let after_block = &updated_lines[start..updated_end];
    let before_len = before_block.len();
    let after_len = after_block.len();
    let max_block = before_len.max(after_len).min(max_lines);
    let mut diff = Vec::new();

    for index in 0..max_block {
        if let Some(line) = before_block.get(index) {
            diff.push(format!("- {}", line));
        }
        if let Some(line) = after_block.get(index) {
            diff.push(format!("+ {}", line));
        }
    }

    if before_len.max(after_len) > max_block {
        diff.push(format!(
            "... {} more changed line(s) hidden for brevity ...",
            before_len.max(after_len) - max_block
        ));
    }

    diff
}

fn excerpt_pair(original: &str, updated: &str, context_lines: usize) -> (String, String) {
    let original_lines = original.lines().collect::<Vec<_>>();
    let updated_lines = updated.lines().collect::<Vec<_>>();
    let (start, original_end, updated_end) = differing_window(&original_lines, &updated_lines);

    let start_with_context = start.saturating_sub(context_lines);
    let original_stop = (original_end + context_lines).min(original_lines.len());
    let updated_stop = (updated_end + context_lines).min(updated_lines.len());

    (
        numbered_excerpt(
            &original_lines[start_with_context..original_stop],
            start_with_context + 1,
        ),
        numbered_excerpt(
            &updated_lines[start_with_context..updated_stop],
            start_with_context + 1,
        ),
    )
}

fn differing_window<'a>(original: &[&'a str], updated: &[&'a str]) -> (usize, usize, usize) {
    let mut prefix = 0usize;
    while prefix < original.len() && prefix < updated.len() && original[prefix] == updated[prefix] {
        prefix += 1;
    }

    let mut original_suffix = original.len();
    let mut updated_suffix = updated.len();
    while original_suffix > prefix
        && updated_suffix > prefix
        && original[original_suffix - 1] == updated[updated_suffix - 1]
    {
        original_suffix -= 1;
        updated_suffix -= 1;
    }

    (prefix, original_suffix, updated_suffix)
}

fn numbered_excerpt(lines: &[&str], start_line: usize) -> String {
    lines
        .iter()
        .enumerate()
        .map(|(offset, line)| format!("{:>4} | {}", start_line + offset, line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn relative_note_path(path: &std::path::Path, root: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn extract_markdown_heading(contents: &str) -> Option<String> {
    contents
        .lines()
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_start_matches("# ").trim().to_string())
}

fn slugify(value: &str) -> String {
    value
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
        .join("-")
}

fn unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn scope_note(adapter_kind: &str) -> String {
    let adapter_file = adapters::adapter_file_path(adapter_kind);
    if adapter_kind == "generic_gallery_html" {
        return format!(
            "Patch scope is limited to this source's selector profile inside config/sources.json. The shared generic adapter ({}) reads that profile on search, but crawler core and unrelated sources stay out of bounds.",
            adapter_file
        );
    }

    if adapters::adapter_ready(adapter_kind) {
        format!(
            "Patch scope is limited to {} for this source family. The shared crawl engine, polite pacing, and dataset curation code stay out of bounds.",
            adapter_file
        )
    } else {
        format!(
            "This source is not fully implemented yet, so the safe patch scope is the adapter scaffold file {} plus this source's local note. Do not expand changes into unrelated source adapters or crawler core.",
            adapter_file
        )
    }
}

fn starter_steps(source_name: &str, adapter_kind: &str) -> Vec<String> {
    vec![
        format!(
            "Reproduce the problem on {} with one concrete search term before changing anything.",
            source_name
        ),
        "Capture what broke: empty results, wrong media URLs, broken thumbs, or pagination drift."
            .to_string(),
        format!(
            "Keep any future AI bugfix scoped to the adapter file for this source family: {}.",
            adapters::adapter_file_path(adapter_kind)
        ),
        if adapter_kind == "generic_gallery_html" {
            "For generic gallery sources, paste a selector-profile JSON into Patch notes before generating the apply review."
                .to_string()
        } else {
            "Generate a review preview before applying so the backup path and changed lines are visible."
                .to_string()
        },
    ]
}

async fn inspect_generic_gallery_source(
    client: &Client,
    source: &SourceEntry,
    request: &SourceFixProposeRequest,
) -> Result<GenericSiteInspection> {
    let probe_url = probe_url_from_template(&source.base_url, request)?;
    let html = client
        .get(probe_url.clone())
        .header(reqwest::header::USER_AGENT, adapters::USER_AGENT)
        .send()
        .await
        .with_context(|| format!("could not fetch {}", probe_url))?
        .error_for_status()
        .with_context(|| format!("{} returned an error for {}", source.name, probe_url))?
        .text()
        .await
        .with_context(|| format!("could not read {} as HTML", probe_url))?;

    let document = Html::parse_document(&html);
    let page_template = detect_page_template(&probe_url, &document);
    let profile = infer_generic_profile_from_document(&document, &source.media_kind);
    let mut analysis_points = vec![format!(
        "Inspected {} as the first-page sample for this generic source.",
        probe_url
    )];

    match page_template.as_ref() {
        Some(template) => analysis_points.push(format!(
            "Found a likely pagination URL template: {}.",
            template
        )),
        None => analysis_points.push(
            "No obvious next/page link was found in the sampled HTML, so URL paging may need a manual template or source profile."
                .to_string(),
        ),
    }

    match profile.item_selector.as_ref() {
        Some(selector) => analysis_points.push(format!(
            "Found repeated media containers that look compatible with selector `{}`.",
            selector
        )),
        None => analysis_points.push(
            "No strong repeated card container was detected, so the draft keeps selectors broad and relies on the URL/listing template to land on the right page."
                .to_string(),
        ),
    }

    let profile_json = Some(generic_profile_json(page_template.as_deref(), &profile)?);
    Ok(GenericSiteInspection {
        analysis_points,
        profile_json,
    })
}

fn probe_url_from_template(base_url: &str, request: &SourceFixProposeRequest) -> Result<Url> {
    let query = likely_probe_query(request);
    let expanded = base_url
        .replace("{query}", &encode_url_component(&query))
        .replace("{page}", "1");
    Url::parse(&expanded).with_context(|| format!("invalid source URL: {}", base_url))
}

fn likely_probe_query(request: &SourceFixProposeRequest) -> String {
    [
        request.reproduction_notes.as_str(),
        request.issue_summary.as_str(),
        request.patch_notes.as_str(),
    ]
    .iter()
    .find_map(|value| first_quoted_text(value))
    .unwrap_or_else(|| "sample".to_string())
}

fn first_quoted_text(value: &str) -> Option<String> {
    let start = value.find('"').or_else(|| value.find('\''))?;
    let quote = value[start..].chars().next()?;
    let rest = &value[start + quote.len_utf8()..];
    let end = rest.find(quote)?;
    let found = rest[..end].trim();
    (!found.is_empty()).then(|| found.to_string())
}

fn detect_page_template(page_url: &Url, document: &Html) -> Option<String> {
    let selector = Selector::parse("a[href], link[href]").ok()?;
    let mut best: Option<(i32, Url)> = None;

    for element in document.select(&selector) {
        let Some(raw_href) = element.value().attr("href") else {
            continue;
        };
        let Some(candidate) = same_site_join(page_url, raw_href) else {
            continue;
        };
        let descriptor = [
            element.value().attr("rel").unwrap_or_default(),
            element.value().attr("aria-label").unwrap_or_default(),
            element.value().attr("title").unwrap_or_default(),
            &element.text().collect::<Vec<_>>().join(" "),
            candidate.as_str(),
        ]
        .join(" ")
        .to_ascii_lowercase();
        let score = pagination_link_score(&descriptor);
        if score <= 0 {
            continue;
        }
        if best
            .as_ref()
            .map(|(best_score, _)| score > *best_score)
            .unwrap_or(true)
        {
            best = Some((score, candidate));
        }
    }

    best.and_then(|(_, url)| url_template_from_pagination_url(&url))
}

fn pagination_link_score(descriptor: &str) -> i32 {
    let mut score = 0;
    for token in ["rel=\"next", " next", "older", "more", "page 2", "page=2"] {
        if descriptor.contains(token) {
            score += 4;
        }
    }
    for token in ["page", "paged", "pagination", "/p/", "?p=", "&p="] {
        if descriptor.contains(token) {
            score += 2;
        }
    }
    if descriptor
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|token| token == "2")
    {
        score += 1;
    }
    score
}

fn url_template_from_pagination_url(url: &Url) -> Option<String> {
    let url_string = url.to_string();
    for key in ["page", "paged", "p"] {
        for marker in [format!("?{}=", key), format!("&{}=", key)] {
            if let Some(start) = url_string.find(&marker) {
                let value_start = start + marker.len();
                let value_end = url_string[value_start..]
                    .find('&')
                    .map(|offset| value_start + offset)
                    .unwrap_or(url_string.len());
                let value = &url_string[value_start..value_end];
                if !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit()) {
                    let mut output = url_string.clone();
                    output.replace_range(value_start..value_end, "{page}");
                    return Some(output);
                }
            }
        }
    }

    replace_last_numeric_path_run(&url_string)
}

fn replace_last_numeric_path_run(url: &str) -> Option<String> {
    let query_start = url.find('?').unwrap_or(url.len());
    let path = &url[..query_start];
    let bytes = path.as_bytes();
    let mut best_run = None;
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_digit() {
            cursor += 1;
            continue;
        }
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        let end = cursor;
        let before = path[..start].to_ascii_lowercase();
        let near_page_word = before
            .rsplit(['/', '-', '_'])
            .next()
            .map(|chunk| chunk.contains("page") || chunk == "p")
            .unwrap_or(false);
        if near_page_word || path[start..end].parse::<u32>().ok().unwrap_or(0) > 1 {
            best_run = Some((start, end));
        }
    }

    let (start, end) = best_run?;
    let mut output = url.to_string();
    output.replace_range(start..end, "{page}");
    Some(output)
}

fn infer_generic_profile_from_document(
    document: &Html,
    source_media_kind: &str,
) -> GenericGalleryProfileDraft {
    let media_selector = match source_media_kind.to_ascii_lowercase().as_str() {
        "video" => "video[src]",
        "audio" => "audio[src]",
        _ => "img[src]",
    };
    let item_selector = best_media_container_selector(document, media_selector);

    GenericGalleryProfileDraft {
        base_url_template: None,
        item_selector,
        media_selector: Some(media_selector.to_string()),
        media_attribute: Some("src".to_string()),
        title_selector: Some("img".to_string()),
        title_attribute: Some("alt".to_string()),
        thumbnail_selector: (media_selector == "img[src]").then(|| "img".to_string()),
        thumbnail_attribute: (media_selector == "img[src]").then(|| "src".to_string()),
        link_selector: Some("a".to_string()),
        link_attribute: Some("href".to_string()),
        media_url_template: None,
        thumbnail_url_template: None,
        title_template: None,
        source_page_url_template: None,
    }
}

fn best_media_container_selector(document: &Html, media_selector: &str) -> Option<String> {
    let candidates = [
        ".gallery-card",
        ".gallery-item",
        ".portfolio-item",
        ".grid-item",
        ".media-item",
        ".post",
        ".entry",
        "article",
        "main article",
    ];

    candidates
        .iter()
        .filter_map(|candidate| {
            let count = count_media_in_container(document, candidate, media_selector);
            (count >= 2).then_some((count, *candidate))
        })
        .max_by_key(|(count, _)| *count)
        .map(|(_, selector)| selector.to_string())
}

fn count_media_in_container(document: &Html, item_selector: &str, media_selector: &str) -> usize {
    let Ok(item_selector) = Selector::parse(item_selector) else {
        return 0;
    };
    let Ok(media_selector) = Selector::parse(media_selector) else {
        return 0;
    };
    document
        .select(&item_selector)
        .filter(|item| item.select(&media_selector).next().is_some())
        .count()
}

fn generic_profile_json(
    base_url_template: Option<&str>,
    profile: &GenericGalleryProfileDraft,
) -> Result<String> {
    let mut object = serde_json::Map::new();
    if let Some(base_url_template) = base_url_template {
        object.insert(
            "base_url_template".to_string(),
            serde_json::Value::String(base_url_template.to_string()),
        );
    }
    insert_json_string(
        &mut object,
        "item_selector",
        profile.item_selector.as_deref(),
    );
    insert_json_string(
        &mut object,
        "media_selector",
        profile.media_selector.as_deref(),
    );
    insert_json_string(
        &mut object,
        "media_attribute",
        profile.media_attribute.as_deref(),
    );
    insert_json_string(
        &mut object,
        "title_selector",
        profile.title_selector.as_deref(),
    );
    insert_json_string(
        &mut object,
        "title_attribute",
        profile.title_attribute.as_deref(),
    );
    insert_json_string(
        &mut object,
        "thumbnail_selector",
        profile.thumbnail_selector.as_deref(),
    );
    insert_json_string(
        &mut object,
        "thumbnail_attribute",
        profile.thumbnail_attribute.as_deref(),
    );
    insert_json_string(
        &mut object,
        "link_selector",
        profile.link_selector.as_deref(),
    );
    insert_json_string(
        &mut object,
        "link_attribute",
        profile.link_attribute.as_deref(),
    );
    insert_json_string(
        &mut object,
        "media_url_template",
        profile.media_url_template.as_deref(),
    );
    insert_json_string(
        &mut object,
        "thumbnail_url_template",
        profile.thumbnail_url_template.as_deref(),
    );
    insert_json_string(
        &mut object,
        "title_template",
        profile.title_template.as_deref(),
    );
    insert_json_string(
        &mut object,
        "source_page_url_template",
        profile.source_page_url_template.as_deref(),
    );

    serde_json::to_string_pretty(&serde_json::Value::Object(object))
        .context("could not serialize generic source profile suggestion")
}

fn insert_json_string(
    object: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<&str>,
) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        object.insert(
            key.to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }
}

fn same_site_join(base: &Url, raw: &str) -> Option<Url> {
    let raw = raw.trim();
    if raw.is_empty()
        || raw.starts_with('#')
        || raw.starts_with("mailto:")
        || raw.starts_with("tel:")
        || raw.starts_with("javascript:")
    {
        return None;
    }
    let url = base.join(raw).ok()?;
    (base.host_str() == url.host_str()).then_some(url)
}

fn encode_url_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn friendly_site_probe_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("timeout") || lower.contains("timed out") {
        "the site did not answer before the timeout".to_string()
    } else if lower.contains("403") || lower.contains("forbidden") {
        "the site refused the probe, often because it blocks automated access".to_string()
    } else if lower.contains("404") || lower.contains("not found") {
        "the current source URL was not found".to_string()
    } else {
        "the sampled page could not be inspected cleanly".to_string()
    }
}

struct IssueProfile {
    key: &'static str,
    short_label: &'static str,
    description: &'static str,
    confidence: &'static str,
}

fn classify_issue(
    adapter_kind: &str,
    issue_summary: &str,
    reproduction_notes: &str,
    patch_notes: &str,
) -> IssueProfile {
    let combined =
        format!("{} {} {}", issue_summary, reproduction_notes, patch_notes).to_ascii_lowercase();

    if combined.contains("thumb")
        || combined.contains("thumbnail")
        || combined.contains("preview image")
    {
        return IssueProfile {
            key: "thumbnail_drift",
            short_label: "thumbnail drift",
            description: "thumbnail or preview URL extraction drift",
            confidence: "High confidence",
        };
    }
    if combined.contains("license")
        || combined.contains("creator")
        || combined.contains("artist")
        || combined.contains("attribution")
    {
        return IssueProfile {
            key: "metadata_drift",
            short_label: "metadata drift",
            description: "license / creator metadata extraction drift",
            confidence: "Medium confidence",
        };
    }
    if combined.contains("page")
        || combined.contains("next")
        || combined.contains("offset")
        || combined.contains("pagination")
    {
        return IssueProfile {
            key: "pagination_drift",
            short_label: "pagination drift",
            description: "page window or has-more logic drifting away from the source",
            confidence: "Medium confidence",
        };
    }
    if combined.contains("link")
        || combined.contains("url")
        || combined.contains("download")
        || combined.contains("media")
    {
        return IssueProfile {
            key: "media_url_drift",
            short_label: "media URL drift",
            description: "media URL or source page URL extraction drift",
            confidence: "High confidence",
        };
    }
    if combined.contains("empty")
        || combined.contains("nothing")
        || combined.contains("no results")
        || combined.contains("search")
        || adapter_kind == "generic_gallery_html"
    {
        return IssueProfile {
            key: "empty_results",
            short_label: "empty results",
            description: "request parameters or result parsing returning an empty preview batch",
            confidence: "Medium confidence",
        };
    }

    IssueProfile {
        key: "generic_drift",
        short_label: "adapter drift",
        description: "site-specific structure drift that needs a small adapter-only correction",
        confidence: "Low-to-medium confidence",
    }
}

fn target_symbols(adapter_kind: &str, adapter_source: &str, issue_key: &str) -> Vec<&'static str> {
    let mut targets = Vec::new();
    if adapter_source.contains("pub async fn search") || adapter_source.contains("pub fn search") {
        targets.push("search()");
    }

    match adapter_kind {
        "openverse_images" | "openverse_audio" => {
            if issue_key == "pagination_drift" {
                targets.push("payload.page_count handling");
            }
            if matches!(
                issue_key,
                "thumbnail_drift" | "media_url_drift" | "empty_results"
            ) {
                targets.push("OpenverseItem -> PreviewItem mapping");
            }
            if issue_key == "metadata_drift" {
                targets.push("license / creator fallbacks");
            }
        }
        "wikimedia_commons" => {
            if issue_key == "pagination_drift" {
                targets.push("gsroffset / has_more detection");
            }
            if matches!(
                issue_key,
                "thumbnail_drift" | "media_url_drift" | "metadata_drift"
            ) {
                targets.push("WikimediaImageInfo mapping");
                targets.push("extmetadata extraction");
            }
        }
        "generic_gallery_html" => {
            targets.push("site-specific HTML selectors");
            targets.push("preview card extraction");
        }
        _ => {}
    }

    targets
}

fn build_patch_sketch(
    adapter_kind: &str,
    adapter_rel: &str,
    request: &SourceFixProposeRequest,
    _issue_key: &str,
    touched_symbols: &[&str],
    inspected_generic_profile_json: Option<&str>,
) -> String {
    let summary = if request.issue_summary.trim().is_empty() {
        "No issue summary supplied".to_string()
    } else {
        request.issue_summary.trim().to_string()
    };
    let reproduction = if request.reproduction_notes.trim().is_empty() {
        "No reproduction notes supplied".to_string()
    } else {
        request.reproduction_notes.trim().to_string()
    };
    let touch_line = if touched_symbols.is_empty() {
        "search() and the preview item mapping".to_string()
    } else {
        touched_symbols.join(", ")
    };

    match adapter_kind {
        "openverse_images" | "openverse_audio" => format!(
            "Issue focus: {summary}\nReproduction notes: {reproduction}\n\nSuggested touch points:\n- {touch_line}\n- request parameters for q/page/page_size\n- PreviewItem fallbacks for media_url, source_page_url, and thumbnail/preview\n\nPatch sketch:\n```rust\n// {adapter_rel}\n// Keep this fix local to the Openverse adapter.\n\n// 1. Re-check the endpoint request parameters in search()\n//    so the query still matches the current Openverse API contract.\n\n// 2. Tighten PreviewItem mapping so drifted or partial items do not poison a whole page.\nlet media_url = item\n    .url\n    .clone()\n    .or_else(|| item.foreign_landing_url.clone())\n    .unwrap_or_default();\nif media_url.trim().is_empty() {{\n    return None;\n}}\n\n// 3. Expand fallback handling for preview/thumb fields if Openverse moved them.\nlet thumb_url = if image_mode {{\n    item.thumbnail.clone().or_else(|| item.url.clone())\n}} else {{\n    None\n}};\n```\n\nReview note:\n- If the problem is only on one query family, prefer adjusting field fallbacks before changing the whole request shape."
        ),
        "wikimedia_commons" => format!(
            "Issue focus: {summary}\nReproduction notes: {reproduction}\n\nSuggested touch points:\n- {touch_line}\n- Wikimedia API params in search()\n- imageinfo/extmetadata mapping for url/thumb/license/creator fields\n\nPatch sketch:\n```rust\n// {adapter_rel}\n// Keep this fix local to the Wikimedia adapter.\n\n// 1. Re-check the query params (generator/search/imageinfo) first.\n// 2. Harden imageinfo extraction so pages with partial metadata are skipped cleanly.\nlet image = page_data.imageinfo.and_then(|mut info| info.drain(..).next())?;\nlet media_url = image.url.clone().unwrap_or_default();\nif media_url.trim().is_empty() {{\n    return None;\n}}\n\n// 3. Widen metadata fallbacks if extmetadata keys have shifted.\nlet license = ext\n    .get(\"LicenseShortName\")\n    .and_then(|field| field.value.clone())\n    .or_else(|| ext.get(\"UsageTerms\").and_then(|field| field.value.clone()));\n```\n\nReview note:\n- Keep paging and polite delay intact unless the break is clearly pagination-related."
        ),
        "generic_gallery_html" => {
            let profile_json =
                inspected_generic_profile_json.unwrap_or(DEFAULT_GENERIC_PROFILE_JSON);
            format!(
                "Issue focus: {summary}\nReproduction notes: {reproduction}\n\nSuggested touch points:\n- {touch_line}\n- URL template, if the site exposes page numbers in links\n- title / thumb / media-link extraction selectors\n- source/detail page link selector if previews need attribution\n- derived templates when the page exposes separate download, poster, or detail URLs\n\nSource connection profile draft:\n```json\n{profile_json}\n```\n\nTemplate placeholders you can use:\n- {{media_url}}, {{source_page_url}}, {{title}}\n- {{media_id}}, {{basename}}, {{basename_stem}}, {{media_path}}\n\nHow to apply:\n- Copy this JSON into Patch notes.\n- If the suggested URL template looks wrong, edit only `base_url_template` or remove that line before review.\n- Use derived templates when the site has the right material but the card is still too dumb to show it cleanly.\n- Generate adapter patch review.\n- Apply after the diff shows only this source's config/sources.json entry changing.\n\nReview note:\n- Do not generalise this into crawler core. The safe apply path stores a validated selector profile and/or URL template for this one source, while {adapter_rel} stays unchanged."
            )
        }
        _ => format!(
            "Issue focus: {summary}\nReproduction notes: {reproduction}\n\nSuggested touch points:\n- {touch_line}\n\nPatch sketch:\n```rust\n// {adapter_rel}\n// Keep the fix local to this adapter file.\n// Prefer correcting field extraction and page parsing before changing shared crawl behaviour.\n```\n\nReview note:\n- Treat this as a scoped draft. Confirm the failing search again after each small adapter edit."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_template_replaces_query_page_value() {
        let url = Url::parse("https://example.com/search?q=dragon&page=2")
            .expect("test URL should parse");

        assert_eq!(
            url_template_from_pagination_url(&url).as_deref(),
            Some("https://example.com/search?q=dragon&page={page}")
        );
    }

    #[test]
    fn url_template_replaces_path_page_value() {
        let url =
            Url::parse("https://example.com/images/category/page2").expect("test URL should parse");

        assert_eq!(
            url_template_from_pagination_url(&url).as_deref(),
            Some("https://example.com/images/category/page{page}")
        );
    }

    #[test]
    fn connection_fix_accepts_url_template_without_selector_profile() {
        let fix = build_generic_gallery_connection_fix(
            "",
            "",
            r#"{"base_url_template":"https://example.com/gallery/page/{page}"}"#,
        )
        .expect("URL-only generic source fix should be valid");

        assert_eq!(
            fix.base_url_template.as_deref(),
            Some("https://example.com/gallery/page/{page}")
        );
        assert!(fix.profile.is_none());
    }

    #[test]
    fn connection_fix_accepts_selector_profile_with_url_template() {
        let fix = build_generic_gallery_connection_fix(
            "",
            "",
            r#"{
              "base_url_template": "https://example.com/gallery?page={page}",
              "item_selector": ".gallery-card",
              "media_selector": "img[src]",
              "media_attribute": "src"
            }"#,
        )
        .expect("combined URL and selector generic source fix should be valid");

        assert_eq!(
            fix.base_url_template.as_deref(),
            Some("https://example.com/gallery?page={page}")
        );
        assert_eq!(
            fix.profile
                .as_ref()
                .map(|profile| profile.media_selector.as_str()),
            Some("img[src]")
        );
    }
}
