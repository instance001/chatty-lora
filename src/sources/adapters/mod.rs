mod generic_gallery_html;
mod openverse;
mod wikimedia_commons;

use std::time::Duration;

use anyhow::{Result, anyhow};
use reqwest::Client;
use tokio::time::sleep;

use crate::types::{
    PagePreviewGroup, SearchPreviewRequest, SearchPreviewResponse, SourceEntry, SourcePreviewBatch,
};

pub const USER_AGENT: &str = "Chatty-lora/0.1 (+https://github.com/)";
pub const PREVIEW_PAGE_SIZE: u32 = 12;
const DEFAULT_PAGES_PER_BATCH: u32 = 3;

pub fn adapter_ready(adapter_kind: &str) -> bool {
    matches!(
        adapter_kind,
        "openverse_images" | "openverse_audio" | "wikimedia_commons" | "generic_gallery_html"
    )
}

pub fn adapter_file_path(adapter_kind: &str) -> &'static str {
    match adapter_kind {
        "openverse_images" | "openverse_audio" => "src/sources/adapters/openverse.rs",
        "wikimedia_commons" => "src/sources/adapters/wikimedia_commons.rs",
        "generic_gallery_html" => "src/sources/adapters/generic_gallery_html.rs",
        _ => "src/sources/adapters/mod.rs",
    }
}

pub async fn search_sources(
    client: &Client,
    all_sources: Vec<SourceEntry>,
    request: SearchPreviewRequest,
) -> Result<SearchPreviewResponse> {
    let query = request.query.trim().to_string();
    let requested_media_kinds = normalize_media_kinds(&request.media_kinds);

    let selected_ids = if request.selected_source_ids.is_empty() {
        all_sources
            .iter()
            .filter(|source| source.enabled)
            .map(|source| source.id.clone())
            .collect::<Vec<_>>()
    } else {
        request.selected_source_ids.clone()
    };

    let mut notes = Vec::new();
    let mut source_batches = Vec::new();
    let (page_window_start, page_window_end) =
        page_window(request.batch_index, DEFAULT_PAGES_PER_BATCH);
    notes.push(format!(
        "Media filter: {}.",
        describe_media_kinds(&requested_media_kinds)
    ));
    if query.is_empty() {
        notes.push(
            "Browse mode: no search term was entered, so Chatty-lora asks selected sources for their first available media pages."
                .to_string(),
        );
        notes.push(
            "A homepage or plain URL only exposes media linked from the fetched page window. Use {page}, a real listing URL, or a source profile for better site coverage."
                .to_string(),
        );
    }

    for source in all_sources
        .into_iter()
        .filter(|source| selected_ids.iter().any(|id| id == &source.id))
    {
        if !source.enabled {
            continue;
        }
        if !source_supports_any_media_kind(&source, &requested_media_kinds) {
            notes.push(format!(
                "{} skipped because it supports {}, while the current media filter is {}.",
                source.name,
                describe_media_kinds(&source_supported_media_kinds(&source)),
                describe_media_kinds(&requested_media_kinds)
            ));
            continue;
        }

        let batch = match search_source(
            client,
            &source,
            &query,
            request.batch_index,
            &requested_media_kinds,
        )
        .await
        {
            Ok(batch) => batch,
            Err(error) => {
                notes.push(format!(
                    "{} had a temporary preview error: {}",
                    source.name,
                    friendly_source_error(&error.to_string())
                ));
                empty_error_batch(&source, page_window_start, page_window_end)
            }
        };
        if batch.pages.iter().all(|page| page.items.is_empty())
            && !batch.note.contains("temporarily unavailable")
        {
            notes.push(format!(
                "{} returned no preview items for pages {}-{}.",
                source.name, page_window_start, page_window_end
            ));
        }
        source_batches.push(batch);
    }

    if source_batches.is_empty() {
        notes.push(
            "No enabled, searchable sources were selected. Enable a source or add a supported one."
                .to_string(),
        );
    }

    Ok(SearchPreviewResponse {
        query,
        batch_index: request.batch_index,
        page_window_start,
        page_window_end,
        source_batches,
        notes,
    })
}

async fn search_source(
    client: &Client,
    source: &SourceEntry,
    query: &str,
    batch_index: u32,
    requested_media_kinds: &[String],
) -> Result<SourcePreviewBatch> {
    let (start_page, end_page) = page_window(batch_index, source.pages_per_batch);

    if !adapter_ready(&source.adapter_kind) {
        return Ok(SourcePreviewBatch {
            source_id: source.id.clone(),
            source_name: source.name.clone(),
            media_kind: source.media_kind.clone(),
            note: format!(
                "{} is registered, but its adapter is not implemented yet. This is where site-specific bugfix files will live later.",
                source.name
            ),
            has_more: false,
            pages: (start_page..=end_page)
                .map(|page_number| PagePreviewGroup {
                    page_number,
                    items: Vec::new(),
                })
                .collect(),
        });
    }

    match source.adapter_kind.as_str() {
        "openverse_images" => {
            openverse::search(client, source, query, start_page, end_page, true).await
        }
        "openverse_audio" => {
            openverse::search(client, source, query, start_page, end_page, false).await
        }
        "wikimedia_commons" => {
            wikimedia_commons::search(client, source, query, start_page, end_page).await
        }
        "generic_gallery_html" => {
            generic_gallery_html::search(
                client,
                source,
                query,
                start_page,
                end_page,
                requested_media_kinds,
            )
            .await
        }
        other => Err(anyhow!("unsupported adapter kind: {}", other)),
    }
}

fn normalize_media_kinds(media_kinds: &[String]) -> Vec<String> {
    let mut normalized = media_kinds
        .iter()
        .filter_map(|kind| match kind.trim().to_ascii_lowercase().as_str() {
            "image" | "images" => Some("image".to_string()),
            "video" | "videos" => Some("video".to_string()),
            "audio" => Some("audio".to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();

    if normalized.is_empty() {
        vec!["image".to_string(), "video".to_string()]
    } else {
        normalized
    }
}

fn source_supported_media_kinds(source: &SourceEntry) -> Vec<String> {
    let mut kinds = match source.adapter_kind.as_str() {
        "openverse_images" | "wikimedia_commons" => vec!["image".to_string()],
        "openverse_audio" => vec!["audio".to_string()],
        "generic_gallery_html" => match source.media_kind.to_ascii_lowercase().as_str() {
            "mixed" => vec![
                "audio".to_string(),
                "image".to_string(),
                "video".to_string(),
            ],
            "audio" => vec!["audio".to_string()],
            "video" => vec!["video".to_string()],
            _ => vec!["image".to_string()],
        },
        _ => match source.media_kind.to_ascii_lowercase().as_str() {
            "audio" => vec!["audio".to_string()],
            "video" => vec!["video".to_string()],
            "mixed" => vec![
                "audio".to_string(),
                "image".to_string(),
                "video".to_string(),
            ],
            _ => vec!["image".to_string()],
        },
    };
    kinds.sort();
    kinds.dedup();
    kinds
}

fn source_supports_any_media_kind(source: &SourceEntry, requested_media_kinds: &[String]) -> bool {
    let supported = source_supported_media_kinds(source);
    supported.iter().any(|supported_kind| {
        requested_media_kinds
            .iter()
            .any(|kind| kind == supported_kind)
    })
}

fn describe_media_kinds(media_kinds: &[String]) -> String {
    let labels = media_kinds
        .iter()
        .map(|kind| match kind.as_str() {
            "image" => "images",
            "video" => "video",
            "audio" => "audio",
            other => other,
        })
        .collect::<Vec<_>>();
    labels.join(", ")
}

pub async fn polite_sleep(delay_ms: u64, page: u32, end_page: u32) {
    if page < end_page {
        sleep(Duration::from_millis(delay_ms)).await;
    }
}

fn page_window(batch_index: u32, pages_per_batch: u32) -> (u32, u32) {
    let pages_per_batch = pages_per_batch.max(1);
    let start_page = batch_index * pages_per_batch + 1;
    let end_page = start_page + pages_per_batch - 1;
    (start_page, end_page)
}

fn empty_error_batch(source: &SourceEntry, start_page: u32, end_page: u32) -> SourcePreviewBatch {
    SourcePreviewBatch {
        source_id: source.id.clone(),
        source_name: source.name.clone(),
        media_kind: source.media_kind.clone(),
        note: format!(
            "{} is temporarily unavailable right now. Chatty-lora kept the rest of the search alive so you can still use other sources.",
            source.name
        ),
        has_more: false,
        pages: (start_page..=end_page)
            .map(|page_number| PagePreviewGroup {
                page_number,
                items: Vec::new(),
            })
            .collect(),
    }
}

fn friendly_source_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("502")
        || lower.contains("503")
        || lower.contains("504")
        || lower.contains("bad gateway")
        || lower.contains("gateway timeout")
    {
        "the source site had a temporary upstream outage.".to_string()
    } else if lower.contains("429") {
        "the source site asked us to slow down for a moment.".to_string()
    } else if lower.contains("timed out") {
        "the request timed out before the source answered.".to_string()
    } else {
        "the source did not return previews cleanly this time.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_window_advances_in_three_page_batches_by_default() {
        assert_eq!(page_window(0, DEFAULT_PAGES_PER_BATCH), (1, 3));
        assert_eq!(page_window(1, DEFAULT_PAGES_PER_BATCH), (4, 6));
        assert_eq!(page_window(2, DEFAULT_PAGES_PER_BATCH), (7, 9));
    }

    #[test]
    fn page_window_respects_source_batch_size_and_clamps_zero() {
        assert_eq!(page_window(1, 1), (2, 2));
        assert_eq!(page_window(1, 5), (6, 10));
        assert_eq!(page_window(3, 0), (4, 4));
    }
}
