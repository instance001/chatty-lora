use anyhow::{Context, Result, anyhow};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use tokio::time::{Duration, sleep};

use crate::types::{PagePreviewGroup, PreviewItem, SourceEntry, SourcePreviewBatch};

use super::{PREVIEW_PAGE_SIZE, USER_AGENT, polite_sleep};

pub async fn search(
    client: &Client,
    source: &SourceEntry,
    query: &str,
    start_page: u32,
    end_page: u32,
    image_mode: bool,
) -> Result<SourcePreviewBatch> {
    let mut pages = Vec::new();
    let mut has_more = false;
    let endpoint = if image_mode {
        "https://api.openverse.org/v1/images/"
    } else {
        "https://api.openverse.org/v1/audio/"
    };

    for page in start_page..=end_page {
        let response = send_openverse_page(client, endpoint, source, query, page).await?;

        let payload: OpenverseResponse = response
            .json()
            .await
            .with_context(|| format!("could not parse {} page {}", source.name, page))?;

        has_more |= payload
            .page_count
            .map(|count| page < count)
            .unwrap_or(false);
        let items = payload
            .results
            .into_iter()
            .map(|item| {
                let title = item
                    .title
                    .filter(|title| !title.trim().is_empty())
                    .unwrap_or_else(|| {
                        format!("Untitled {}", if image_mode { "image" } else { "audio" })
                    });
                let media_url = item
                    .url
                    .clone()
                    .unwrap_or_else(|| item.foreign_landing_url.clone().unwrap_or_default());
                let preview_url = if image_mode { None } else { item.url.clone() };
                let thumb_url = if image_mode {
                    item.thumbnail.clone()
                } else {
                    None
                };

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

        pages.push(PagePreviewGroup {
            page_number: page,
            items,
        });

        polite_sleep(source.crawl_delay_ms, page, end_page).await;
    }

    Ok(SourcePreviewBatch {
        source_id: source.id.clone(),
        source_name: source.name.clone(),
        media_kind: source.media_kind.clone(),
        note: format!(
            "{} previewed via public API in a polite {}-page batch{}.",
            source.name,
            source.pages_per_batch.max(1),
            if query.trim().is_empty() {
                " using browse mode"
            } else {
                ""
            }
        ),
        has_more,
        pages,
    })
}

async fn send_openverse_page(
    client: &Client,
    endpoint: &str,
    source: &SourceEntry,
    query: &str,
    page: u32,
) -> Result<reqwest::Response> {
    const MAX_ATTEMPTS: usize = 3;

    for attempt in 0..MAX_ATTEMPTS {
        let mut request = client
            .get(endpoint)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .query(&[
                ("page", page.to_string()),
                ("page_size", PREVIEW_PAGE_SIZE.to_string()),
            ]);
        if !query.trim().is_empty() {
            request = request.query(&[("q", query.to_string())]);
        }
        let send_result = request.send().await;

        match send_result {
            Ok(response) if response.status().is_success() => return Ok(response),
            Ok(response)
                if should_retry_status(response.status()) && attempt + 1 < MAX_ATTEMPTS =>
            {
                sleep(Duration::from_millis(900 * (attempt as u64 + 1))).await;
                continue;
            }
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow!(
                    "{} returned an error on page {}: {}{}",
                    source.name,
                    page,
                    status,
                    if body.trim().is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", truncate_body(&body))
                    }
                ));
            }
            Err(error)
                if (error.is_timeout() || error.is_connect()) && attempt + 1 < MAX_ATTEMPTS =>
            {
                sleep(Duration::from_millis(900 * (attempt as u64 + 1))).await;
                continue;
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("{} request failed on page {}", source.name, page));
            }
        }
    }

    Err(anyhow!(
        "{} request failed on page {} after retries",
        source.name,
        page
    ))
}

fn should_retry_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
            | StatusCode::INTERNAL_SERVER_ERROR
    )
}

fn truncate_body(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.len() > 120 {
        format!("{}...", &trimmed[..120])
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Deserialize)]
struct OpenverseResponse {
    #[serde(default)]
    page_count: Option<u32>,
    #[serde(default)]
    results: Vec<OpenverseItem>,
}

#[derive(Debug, Deserialize)]
struct OpenverseItem {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    creator: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    license_version: Option<String>,
    #[serde(default)]
    thumbnail: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    foreign_landing_url: Option<String>,
}
