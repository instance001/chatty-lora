use std::collections::BTreeMap;

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
) -> Result<SourcePreviewBatch> {
    let mut pages = Vec::new();
    let mut has_more = false;

    for page in start_page..=end_page {
        let offset = (page - 1) * PREVIEW_PAGE_SIZE;
        let browse_mode = query.trim().is_empty();
        let response = send_wikimedia_page(client, source, query, page, offset).await?;

        let payload: WikimediaResponse = response
            .json()
            .await
            .with_context(|| format!("could not parse {} page {}", source.name, page))?;

        let mut raw_pages = payload.query.map(|query| query.pages).unwrap_or_default();
        raw_pages.sort_by(|left, right| {
            left.title
                .to_ascii_lowercase()
                .cmp(&right.title.to_ascii_lowercase())
        });
        has_more |= if browse_mode {
            raw_pages.len() as u32 >= offset + PREVIEW_PAGE_SIZE
        } else {
            raw_pages.len() as u32 == PREVIEW_PAGE_SIZE
        };

        let page_items = if browse_mode {
            raw_pages
                .into_iter()
                .skip(offset as usize)
                .take(PREVIEW_PAGE_SIZE as usize)
                .collect::<Vec<_>>()
        } else {
            raw_pages
        };

        let items = page_items
            .into_iter()
            .filter_map(|page_data| {
                let image = page_data
                    .imageinfo
                    .and_then(|mut info| info.drain(..).next())?;
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
                    source_page_url: image
                        .descriptionurl
                        .unwrap_or_else(|| image.url.unwrap_or_default()),
                    license,
                    creator,
                    source_label: source.name.clone(),
                    page_number: page,
                    kind: "Image".to_string(),
                })
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
            "{} previewed through the Wikimedia API in a polite {}-page batch{}.",
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

async fn send_wikimedia_page(
    client: &Client,
    source: &SourceEntry,
    query: &str,
    page: u32,
    offset: u32,
) -> Result<reqwest::Response> {
    const MAX_ATTEMPTS: usize = 3;

    for attempt in 0..MAX_ATTEMPTS {
        let mut params = vec![
            ("action", "query".to_string()),
            ("format", "json".to_string()),
            ("formatversion", "2".to_string()),
            ("prop", "imageinfo".to_string()),
            ("iiprop", "url|extmetadata".to_string()),
            ("iiurlwidth", "480".to_string()),
            ("origin", "*".to_string()),
        ];
        if query.trim().is_empty() {
            let fetch_limit = (offset + PREVIEW_PAGE_SIZE).min(50);
            params.extend([
                ("generator", "allimages".to_string()),
                ("gailimit", fetch_limit.to_string()),
            ]);
        } else {
            params.extend([
                ("generator", "search".to_string()),
                ("gsrsearch", query.to_string()),
                ("gsrnamespace", "6".to_string()),
                ("gsrlimit", PREVIEW_PAGE_SIZE.to_string()),
                ("gsroffset", offset.to_string()),
            ]);
        }

        let send_result = client
            .get("https://commons.wikimedia.org/w/api.php")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .query(&params)
            .send()
            .await;

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
struct WikimediaResponse {
    #[serde(default)]
    query: Option<WikimediaQuery>,
}

#[derive(Debug, Deserialize)]
struct WikimediaQuery {
    #[serde(default)]
    pages: Vec<WikimediaPage>,
}

#[derive(Debug, Deserialize)]
struct WikimediaPage {
    title: String,
    #[serde(default)]
    imageinfo: Option<Vec<WikimediaImageInfo>>,
}

#[derive(Debug, Deserialize)]
struct WikimediaImageInfo {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    thumburl: Option<String>,
    #[serde(default)]
    descriptionurl: Option<String>,
    #[serde(default)]
    extmetadata: Option<BTreeMap<String, WikimediaValue>>,
}

#[derive(Debug, Deserialize)]
struct WikimediaValue {
    #[serde(default)]
    value: Option<String>,
}
