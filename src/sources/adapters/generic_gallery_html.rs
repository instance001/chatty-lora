use std::collections::BTreeSet;

use anyhow::{Context, Result};
use reqwest::{Client, Url};
use scraper::{ElementRef, Html, Selector};

use crate::types::{
    GenericGalleryProfile, PagePreviewGroup, PreviewItem, SourceEntry, SourcePreviewBatch,
};

use super::{PREVIEW_PAGE_SIZE, USER_AGENT, polite_sleep};

const DISCOVERY_FETCH_LIMIT: usize = 4;
const DISCOVERY_LINK_LIMIT: usize = 18;

pub async fn search(
    client: &Client,
    source: &SourceEntry,
    query: &str,
    start_page: u32,
    end_page: u32,
    requested_media_kinds: &[String],
) -> Result<SourcePreviewBatch> {
    let mut pages = Vec::new();
    let mut notes = Vec::new();
    let mut has_more = false;

    for page in start_page..=end_page {
        let page_url = build_search_url(&source.base_url, query, page)?;
        if source.respect_robots_txt && !robots_allows(client, &page_url).await? {
            pages.push(PagePreviewGroup {
                page_number: page,
                items: Vec::new(),
            });
            notes.push(format!(
                "{} blocked page {} through robots.txt, so Chatty-lora skipped it.",
                source.name, page
            ));
            break;
        }

        let html = fetch_html(client, source, page_url.clone(), page).await?;
        let mut items =
            extract_preview_items(source, page, &page_url, &html, requested_media_kinds);
        let direct_item_count = items.len();
        if direct_item_count < PREVIEW_PAGE_SIZE as usize {
            let rescued = discover_linked_items(
                client,
                source,
                query,
                page,
                &page_url,
                &html,
                requested_media_kinds,
            )
            .await?;
            if !rescued.items.is_empty() {
                if direct_item_count == 0 {
                    notes.push(format!(
                        "{} found media by following same-site sitemap/feed/gallery links because the starting page had no direct training-style media for this {}.",
                        source.name,
                        if query.trim().is_empty() { "browse" } else { "search" }
                    ));
                } else {
                    notes.push(format!(
                        "{} only found {} direct media item(s), so Chatty-lora also followed same-site sitemap/feed/gallery links for this {}.",
                        source.name,
                        direct_item_count,
                        if query.trim().is_empty() { "browse" } else { "search" }
                    ));
                }
                has_more |= rescued.has_more;
                merge_preview_items(&mut items, rescued.items);
            } else if direct_item_count == 0 {
                notes.push(format!(
                    "{} starting page did not expose training-style media or obvious same-site gallery/feed links for this {}.",
                    source.name,
                    if query.trim().is_empty() { "browse" } else { "search" }
                ));
            }
        }
        has_more |= items.len() as u32 >= PREVIEW_PAGE_SIZE;
        pages.push(PagePreviewGroup {
            page_number: page,
            items,
        });
        polite_sleep(source.crawl_delay_ms, page, end_page).await;
    }

    let mut note = format!(
        "{} previewed with the generic gallery HTML adapter. {}",
        source.name,
        if source.site_profile.is_some() {
            "A source-specific selector profile is active before the cautious fallback scan."
        } else {
            "This is a cautious best-effort scan of common image/audio/video links, not a site-specific integration."
        }
    );
    if !notes.is_empty() {
        note.push(' ');
        note.push_str(&notes.join(" "));
    }

    Ok(SourcePreviewBatch {
        source_id: source.id.clone(),
        source_name: source.name.clone(),
        media_kind: source.media_kind.clone(),
        note,
        has_more,
        pages,
    })
}

struct DiscoveryItems {
    items: Vec<PreviewItem>,
    has_more: bool,
}

fn merge_preview_items(items: &mut Vec<PreviewItem>, additional_items: Vec<PreviewItem>) {
    let mut seen_media = items
        .iter()
        .map(|item| item.media_url.clone())
        .collect::<BTreeSet<_>>();
    for item in additional_items {
        if seen_media.insert(item.media_url.clone()) {
            items.push(item);
        }
        if items.len() >= PREVIEW_PAGE_SIZE as usize {
            break;
        }
    }
}

async fn discover_linked_items(
    client: &Client,
    source: &SourceEntry,
    query: &str,
    page_number: u32,
    page_url: &Url,
    html: &str,
    requested_media_kinds: &[String],
) -> Result<DiscoveryItems> {
    let candidates = discover_candidate_pages(client, page_url, html, query).await?;
    if candidates.is_empty() {
        return Ok(DiscoveryItems {
            items: Vec::new(),
            has_more: false,
        });
    }

    let page_offset = page_number.saturating_sub(1) as usize * DISCOVERY_FETCH_LIMIT;
    let has_more_candidates = candidates.len() > page_offset + DISCOVERY_FETCH_LIMIT;
    let mut seen_media = BTreeSet::new();
    let mut items = Vec::new();
    for candidate in candidates
        .into_iter()
        .skip(page_offset)
        .take(DISCOVERY_FETCH_LIMIT)
    {
        let child_html = match fetch_html(client, source, candidate.clone(), page_number).await {
            Ok(child_html) => child_html,
            Err(_) => continue,
        };
        for item in extract_preview_items(
            source,
            page_number,
            &candidate,
            &child_html,
            requested_media_kinds,
        ) {
            if seen_media.insert(item.media_url.clone()) {
                items.push(item);
            }
            if items.len() >= PREVIEW_PAGE_SIZE as usize {
                return Ok(DiscoveryItems {
                    items,
                    has_more: true,
                });
            }
        }
    }

    Ok(DiscoveryItems {
        items,
        has_more: has_more_candidates,
    })
}

async fn discover_candidate_pages(
    client: &Client,
    page_url: &Url,
    html: &str,
    query: &str,
) -> Result<Vec<Url>> {
    let mut candidates = Vec::new();
    collect_candidate_links(page_url, html, query, &mut candidates);

    if candidates.len() < DISCOVERY_LINK_LIMIT {
        let robots_candidates = discover_from_robots(client, page_url)
            .await
            .unwrap_or_default();
        push_unique_candidates(page_url, &mut candidates, robots_candidates);
    }

    if candidates.len() < DISCOVERY_LINK_LIMIT {
        let sitemap_url = same_site_join(page_url, "/sitemap.xml");
        if let Some(sitemap_url) = sitemap_url {
            let sitemap_candidates = discover_from_sitemap(client, page_url, sitemap_url)
                .await
                .unwrap_or_default();
            push_unique_candidates(page_url, &mut candidates, sitemap_candidates);
        }
    }

    candidates.truncate(DISCOVERY_LINK_LIMIT);
    Ok(candidates)
}

async fn discover_from_robots(client: &Client, page_url: &Url) -> Result<Vec<Url>> {
    let mut robots_url = page_url.clone();
    robots_url.set_path("/robots.txt");
    robots_url.set_query(None);
    robots_url.set_fragment(None);

    let response = client
        .get(robots_url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?;
    if !response.status().is_success() {
        return Ok(Vec::new());
    }

    let robots = response.text().await.unwrap_or_default();
    let mut candidates = Vec::new();
    for line in robots.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case("sitemap") {
            if let Ok(sitemap_url) = Url::parse(value.trim()) {
                let sitemap_candidates = discover_from_sitemap(client, page_url, sitemap_url)
                    .await
                    .unwrap_or_default();
                push_unique_candidates(page_url, &mut candidates, sitemap_candidates);
            }
        }
    }
    Ok(candidates)
}

async fn discover_from_sitemap(
    client: &Client,
    page_url: &Url,
    sitemap_url: Url,
) -> Result<Vec<Url>> {
    if !same_site_url(page_url, &sitemap_url) {
        return Ok(Vec::new());
    }
    let response = client
        .get(sitemap_url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?;
    if !response.status().is_success() {
        return Ok(Vec::new());
    }

    let body = response.text().await.unwrap_or_default();
    let mut preferred = Vec::new();
    let mut fallback = Vec::new();
    for raw in extract_tag_values(&body, "loc") {
        let Ok(url) = Url::parse(&raw) else {
            continue;
        };
        if !same_site_url(page_url, &url) {
            continue;
        }
        if link_looks_useful(url.as_str(), "") {
            push_unique_url(&mut preferred, url);
        } else {
            push_unique_url(&mut fallback, url);
        }
        if preferred.len() >= DISCOVERY_LINK_LIMIT {
            break;
        }
    }
    push_unique_candidates(page_url, &mut preferred, fallback);
    preferred.truncate(DISCOVERY_LINK_LIMIT);
    Ok(preferred)
}

fn collect_candidate_links(page_url: &Url, html: &str, query: &str, candidates: &mut Vec<Url>) {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href], link[href]").expect("valid discovery selector");
    for element in document.select(&selector) {
        let Some(raw_href) = element.value().attr("href") else {
            continue;
        };
        let Some(url) = same_site_join(page_url, raw_href) else {
            continue;
        };

        let rel = element.value().attr("rel").unwrap_or_default();
        let kind = element.value().attr("type").unwrap_or_default();
        let text = element.text().collect::<Vec<_>>().join(" ");
        let descriptor = format!("{rel} {kind} {text}");
        if !link_looks_useful(url.as_str(), &descriptor) {
            continue;
        }
        push_unique_url(candidates, search_candidate_url(url, query));
        if candidates.len() >= DISCOVERY_LINK_LIMIT {
            break;
        }
    }
}

fn search_candidate_url(mut url: Url, query: &str) -> Url {
    let query = query.trim();
    if query.is_empty() || url.query().is_some() {
        return url;
    }

    let path = url.path().to_ascii_lowercase();
    let looks_searchable = [
        "search",
        "gallery",
        "galleries",
        "media",
        "photo",
        "photos",
        "image",
        "images",
        "video",
        "videos",
        "audio",
        "archive",
        "portfolio",
    ]
    .iter()
    .any(|needle| path.contains(needle));

    if looks_searchable {
        url.query_pairs_mut().append_pair("q", query);
    }
    url
}

fn extract_tag_values(body: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let lower = body.to_ascii_lowercase();
    let mut values = Vec::new();
    let mut cursor = 0usize;

    while let Some(start) = lower[cursor..].find(&open) {
        let value_start = cursor + start + open.len();
        let Some(end) = lower[value_start..].find(&close) else {
            break;
        };
        let value_end = value_start + end;
        let value = body[value_start..value_end].trim();
        if !value.is_empty() {
            values.push(value.to_string());
        }
        cursor = value_end + close.len();
        if values.len() >= DISCOVERY_LINK_LIMIT * 2 {
            break;
        }
    }

    values
}

fn link_looks_useful(url: &str, descriptor: &str) -> bool {
    let combined = format!("{url} {descriptor}").to_ascii_lowercase();
    const KEYWORDS: &[&str] = &[
        "gallery",
        "galleries",
        "photo",
        "photos",
        "image",
        "images",
        "media",
        "video",
        "videos",
        "audio",
        "portfolio",
        "works",
        "albums",
        "download",
        "downloads",
        "archive",
        "feed",
        "rss",
        "atom",
        "sitemap",
    ];
    KEYWORDS.iter().any(|keyword| combined.contains(keyword))
}

fn same_site_join(page_url: &Url, raw: &str) -> Option<Url> {
    let raw = raw.trim();
    if raw.is_empty()
        || raw.starts_with('#')
        || raw.starts_with("mailto:")
        || raw.starts_with("tel:")
        || raw.starts_with("javascript:")
    {
        return None;
    }
    let url = page_url.join(raw).ok()?;
    same_site_url(page_url, &url).then_some(url)
}

fn same_site_url(base: &Url, candidate: &Url) -> bool {
    base.host_str() == candidate.host_str()
}

fn push_unique_candidates(base: &Url, candidates: &mut Vec<Url>, new_candidates: Vec<Url>) {
    for url in new_candidates {
        if same_site_url(base, &url) {
            push_unique_url(candidates, url);
        }
        if candidates.len() >= DISCOVERY_LINK_LIMIT {
            break;
        }
    }
}

fn push_unique_url(candidates: &mut Vec<Url>, url: Url) {
    if !candidates.iter().any(|candidate| candidate == &url) {
        candidates.push(url);
    }
}

fn build_search_url(base_url: &str, query: &str, page: u32) -> Result<Url> {
    let encoded_query = encode_url_component(query.trim());
    let expanded = base_url
        .replace("{query}", &encoded_query)
        .replace("{page}", &page.to_string());
    let mut url =
        Url::parse(&expanded).with_context(|| format!("invalid source URL: {base_url}"))?;

    if !base_url.contains("{query}") && !query.trim().is_empty() {
        url.query_pairs_mut().append_pair("q", query);
    }
    if !base_url.contains("{page}") {
        url.query_pairs_mut().append_pair("page", &page.to_string());
    }

    Ok(url)
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

async fn fetch_html(
    client: &Client,
    source: &SourceEntry,
    page_url: Url,
    page: u32,
) -> Result<String> {
    let response = client
        .get(page_url.clone())
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .with_context(|| format!("{} request failed on page {}", source.name, page))?
        .error_for_status()
        .with_context(|| format!("{} returned an error for {}", source.name, page_url))?;

    response
        .text()
        .await
        .with_context(|| format!("could not read {} page {} as HTML", source.name, page))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_query_template_special_characters() {
        let url = build_search_url(
            "https://example.com/search?q={query}&page={page}",
            "@journal cover #4598 & straps",
            3,
        )
        .expect("search URL should parse");

        assert_eq!(
            url.as_str(),
            "https://example.com/search?q=%40journal%20cover%20%234598%20%26%20straps&page=3"
        );
    }

    #[test]
    fn encodes_query_template_inside_path() {
        let url = build_search_url(
            "https://example.com/tags/{query}/page/{page}",
            "@leather/books",
            2,
        )
        .expect("path URL should parse");

        assert_eq!(
            url.as_str(),
            "https://example.com/tags/%40leather%2Fbooks/page/2"
        );
    }

    #[test]
    fn appends_non_template_query_with_url_encoder() {
        let url = build_search_url("https://example.com/gallery", "@journal cover #4598", 4)
            .expect("gallery URL should parse");

        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "q").unwrap().1,
            "@journal cover #4598"
        );
        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "page").unwrap().1,
            "4"
        );
    }

    #[test]
    fn treats_language_flags_as_site_chrome() {
        assert!(image_metadata_looks_like_site_chrome(
            "https://example.com/static/flags/en.png language selector",
            Some(32),
            Some(20)
        ));
    }

    #[test]
    fn keeps_regular_gallery_images() {
        assert!(!image_metadata_looks_like_site_chrome(
            "https://example.com/gallery/leather-journal-cover-4598.jpg handcrafted journal",
            Some(1200),
            Some(900)
        ));
    }
}

async fn robots_allows(client: &Client, page_url: &Url) -> Result<bool> {
    let mut robots_url = page_url.clone();
    robots_url.set_path("/robots.txt");
    robots_url.set_query(None);
    robots_url.set_fragment(None);

    let response = match client
        .get(robots_url.clone())
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => return Ok(true),
    };

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(true);
    }
    if !response.status().is_success() {
        return Ok(true);
    }

    let robots = response.text().await.unwrap_or_default();
    Ok(path_allowed_by_robots(&robots, page_url.path()))
}

fn path_allowed_by_robots(robots: &str, path: &str) -> bool {
    let mut applies = false;
    for line in robots.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        if key == "user-agent" {
            let agent = value.to_ascii_lowercase();
            applies = agent == "*" || agent.contains("chatty-lora");
            continue;
        }
        if applies && key == "disallow" && !value.is_empty() && path.starts_with(value) {
            return false;
        }
    }
    true
}

fn extract_preview_items(
    source: &SourceEntry,
    page_number: u32,
    page_url: &Url,
    html: &str,
    requested_media_kinds: &[String],
) -> Vec<PreviewItem> {
    let document = Html::parse_document(html);
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();

    if let Some(profile) = source.site_profile.as_ref() {
        let profiled_items = extract_profile_items(
            source,
            page_number,
            page_url,
            &document,
            profile,
            requested_media_kinds,
        );
        if !profiled_items.is_empty() {
            return profiled_items;
        }
    }

    collect_img_items(
        source,
        page_number,
        page_url,
        &document,
        &mut seen,
        &mut items,
        requested_media_kinds,
    );
    collect_media_tag_items(
        source,
        page_number,
        page_url,
        &document,
        &mut seen,
        &mut items,
        requested_media_kinds,
    );
    collect_anchor_items(
        source,
        page_number,
        page_url,
        &document,
        &mut seen,
        &mut items,
        requested_media_kinds,
    );

    items.truncate(PREVIEW_PAGE_SIZE as usize);
    items
}

fn extract_profile_items(
    source: &SourceEntry,
    page_number: u32,
    page_url: &Url,
    document: &Html,
    profile: &GenericGalleryProfile,
    requested_media_kinds: &[String],
) -> Vec<PreviewItem> {
    let Ok(media_selector) = Selector::parse(&profile.media_selector) else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();

    if let Some(item_selector) = profile
        .item_selector
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let Ok(item_selector) = Selector::parse(item_selector) else {
            return Vec::new();
        };
        for card in document.select(&item_selector) {
            let Some(media_element) = card.select(&media_selector).next() else {
                continue;
            };
            if let Some(item) = profile_item_from_element(
                source,
                page_number,
                page_url,
                profile,
                &media_element,
                Some(&card),
                requested_media_kinds,
                &mut seen,
            ) {
                items.push(item);
            }
            if items.len() >= PREVIEW_PAGE_SIZE as usize {
                break;
            }
        }
    } else {
        for media_element in document.select(&media_selector) {
            if let Some(item) = profile_item_from_element(
                source,
                page_number,
                page_url,
                profile,
                &media_element,
                None,
                requested_media_kinds,
                &mut seen,
            ) {
                items.push(item);
            }
            if items.len() >= PREVIEW_PAGE_SIZE as usize {
                break;
            }
        }
    }

    items
}

fn profile_item_from_element(
    source: &SourceEntry,
    page_number: u32,
    page_url: &Url,
    profile: &GenericGalleryProfile,
    media_element: &ElementRef<'_>,
    card: Option<&ElementRef<'_>>,
    requested_media_kinds: &[String],
    seen: &mut BTreeSet<String>,
) -> Option<PreviewItem> {
    let media_url = absolutize_profile_attr(page_url, media_element, &profile.media_attribute)?;
    let kind = media_kind_from_url(&media_url)
        .or_else(|| profile_kind_from_context(&source.media_kind, &profile.media_selector))?;
    if !matches_media_focus(&source.media_kind, kind)
        || !matches_requested_media(requested_media_kinds, kind)
        || !seen.insert(media_url.clone())
    {
        return None;
    }

    let title = profile_title(card, media_element, profile)
        .unwrap_or_else(|| title_from_url(&media_url, &kind.to_ascii_lowercase()));
    let thumbnail_url = profile
        .thumbnail_selector
        .as_deref()
        .and_then(|selector| {
            profile_selected_url(
                card,
                media_element,
                page_url,
                selector,
                profile.thumbnail_attribute.as_deref(),
            )
        })
        .or_else(|| (kind == "Image").then(|| media_url.clone()));
    let source_page_url = profile
        .link_selector
        .as_deref()
        .and_then(|selector| {
            profile_selected_url(
                card,
                media_element,
                page_url,
                selector,
                profile.link_attribute.as_deref(),
            )
        })
        .unwrap_or_else(|| page_url.as_str().to_string());

    Some(PreviewItem {
        key: format!("{}::{}::{}", source.id, page_number, media_url),
        title,
        thumb_url: thumbnail_url,
        preview_url: if kind == "Image" {
            None
        } else {
            Some(media_url.clone())
        },
        media_url,
        source_page_url,
        license: None,
        creator: None,
        source_label: source.name.clone(),
        page_number,
        kind: kind.to_string(),
    })
}

fn profile_title(
    card: Option<&ElementRef<'_>>,
    media_element: &ElementRef<'_>,
    profile: &GenericGalleryProfile,
) -> Option<String> {
    if let Some(selector) = profile.title_selector.as_deref() {
        if let Some(title) = profile_selected_text(
            card,
            media_element,
            selector,
            profile.title_attribute.as_deref(),
        ) {
            return Some(title);
        }
    }

    first_non_empty(&[
        media_element.value().attr("alt"),
        media_element.value().attr("title"),
        media_element.value().attr("aria-label"),
    ])
}

fn profile_selected_text(
    card: Option<&ElementRef<'_>>,
    media_element: &ElementRef<'_>,
    selector: &str,
    attribute: Option<&str>,
) -> Option<String> {
    let selector = Selector::parse(selector).ok()?;
    let found = card
        .and_then(|card| card.select(&selector).next())
        .or_else(|| media_element.select(&selector).next())?;
    if let Some(attribute) = attribute {
        return selected_attribute(&found, attribute).and_then(|value| clean_title(&value));
    }
    clean_title(&found.text().collect::<Vec<_>>().join(" "))
}

fn profile_selected_url(
    card: Option<&ElementRef<'_>>,
    media_element: &ElementRef<'_>,
    page_url: &Url,
    selector: &str,
    attribute: Option<&str>,
) -> Option<String> {
    let selector = Selector::parse(selector).ok()?;
    let found = card
        .and_then(|card| card.select(&selector).next())
        .or_else(|| media_element.select(&selector).next())?;
    absolutize_profile_attr(page_url, &found, attribute.unwrap_or("href"))
}

fn absolutize_profile_attr(
    page_url: &Url,
    element: &ElementRef<'_>,
    attribute: &str,
) -> Option<String> {
    let raw = selected_attribute(element, attribute)?;
    let urlish = if attribute.eq_ignore_ascii_case("srcset") {
        raw.split(',')
            .filter_map(|candidate| candidate.split_whitespace().next())
            .find(|candidate| !candidate.trim().is_empty())
            .unwrap_or_default()
            .to_string()
    } else {
        raw
    };
    absolutize_media_url(page_url, &urlish)
}

fn selected_attribute(element: &ElementRef<'_>, attribute: &str) -> Option<String> {
    if attribute.eq_ignore_ascii_case("text") {
        return clean_title(&element.text().collect::<Vec<_>>().join(" "));
    }
    element
        .value()
        .attr(attribute)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn profile_kind_from_context(source_media_kind: &str, selector: &str) -> Option<&'static str> {
    match source_media_kind.to_ascii_lowercase().as_str() {
        "image" => Some("Image"),
        "video" => Some("Video"),
        "audio" => Some("Audio"),
        "mixed" if selector.contains("video") => Some("Video"),
        "mixed" if selector.contains("audio") => Some("Audio"),
        "mixed" if selector.contains("img") || selector.contains("image") => Some("Image"),
        _ => None,
    }
}

fn collect_img_items(
    source: &SourceEntry,
    page_number: u32,
    page_url: &Url,
    document: &Html,
    seen: &mut BTreeSet<String>,
    items: &mut Vec<PreviewItem>,
    requested_media_kinds: &[String],
) {
    if !matches_media_focus(&source.media_kind, "Image")
        || !matches_requested_media(requested_media_kinds, "Image")
    {
        return;
    }
    let selector = Selector::parse("img[src]").expect("valid img selector");
    for element in document.select(&selector) {
        let Some(raw_src) = element.value().attr("src") else {
            continue;
        };
        let Some(media_url) = absolutize_media_url(page_url, raw_src) else {
            continue;
        };
        if image_looks_like_site_chrome(&media_url, &element) {
            continue;
        }
        if !looks_like_media_url(&media_url, "Image") || !seen.insert(media_url.clone()) {
            continue;
        }
        let title = first_non_empty(&[
            element.value().attr("alt"),
            element.value().attr("title"),
            element.value().attr("aria-label"),
        ])
        .unwrap_or_else(|| title_from_url(&media_url, "image"));

        items.push(PreviewItem {
            key: format!("{}::{}::{}", source.id, page_number, media_url),
            title,
            thumb_url: Some(media_url.clone()),
            preview_url: None,
            media_url: media_url.clone(),
            source_page_url: page_url.as_str().to_string(),
            license: None,
            creator: None,
            source_label: source.name.clone(),
            page_number,
            kind: "Image".to_string(),
        });
    }
}

fn collect_media_tag_items(
    source: &SourceEntry,
    page_number: u32,
    page_url: &Url,
    document: &Html,
    seen: &mut BTreeSet<String>,
    items: &mut Vec<PreviewItem>,
    requested_media_kinds: &[String],
) {
    let selector =
        Selector::parse("audio[src], video[src], source[src]").expect("valid media selector");
    for element in document.select(&selector) {
        let Some(raw_src) = element.value().attr("src") else {
            continue;
        };
        let Some(media_url) = absolutize_media_url(page_url, raw_src) else {
            continue;
        };
        let Some(kind) = media_kind_from_url(&media_url) else {
            continue;
        };
        if !matches_media_focus(&source.media_kind, kind)
            || !matches_requested_media(requested_media_kinds, kind)
            || !seen.insert(media_url.clone())
        {
            continue;
        }

        items.push(PreviewItem {
            key: format!("{}::{}::{}", source.id, page_number, media_url),
            title: title_from_url(&media_url, &kind.to_ascii_lowercase()),
            thumb_url: None,
            preview_url: Some(media_url.clone()),
            media_url: media_url.clone(),
            source_page_url: page_url.as_str().to_string(),
            license: None,
            creator: None,
            source_label: source.name.clone(),
            page_number,
            kind: kind.to_string(),
        });
    }
}

fn collect_anchor_items(
    source: &SourceEntry,
    page_number: u32,
    page_url: &Url,
    document: &Html,
    seen: &mut BTreeSet<String>,
    items: &mut Vec<PreviewItem>,
    requested_media_kinds: &[String],
) {
    let selector = Selector::parse("a[href]").expect("valid anchor selector");
    for element in document.select(&selector) {
        let Some(raw_href) = element.value().attr("href") else {
            continue;
        };
        let Some(media_url) = absolutize_media_url(page_url, raw_href) else {
            continue;
        };
        let Some(kind) = media_kind_from_url(&media_url) else {
            continue;
        };
        if !matches_media_focus(&source.media_kind, kind)
            || !matches_requested_media(requested_media_kinds, kind)
            || !seen.insert(media_url.clone())
        {
            continue;
        }

        let text = element.text().collect::<Vec<_>>().join(" ");
        let title = clean_title(&text)
            .unwrap_or_else(|| title_from_url(&media_url, &kind.to_ascii_lowercase()));

        items.push(PreviewItem {
            key: format!("{}::{}::{}", source.id, page_number, media_url),
            title,
            thumb_url: if kind == "Image" {
                Some(media_url.clone())
            } else {
                None
            },
            preview_url: if kind == "Image" {
                None
            } else {
                Some(media_url.clone())
            },
            media_url: media_url.clone(),
            source_page_url: page_url.as_str().to_string(),
            license: None,
            creator: None,
            source_label: source.name.clone(),
            page_number,
            kind: kind.to_string(),
        });
    }
}

fn absolutize_media_url(page_url: &Url, raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty()
        || raw.starts_with('#')
        || raw.starts_with("data:")
        || raw.starts_with("blob:")
        || raw.starts_with("javascript:")
    {
        return None;
    }
    page_url.join(raw).ok().map(|url| url.to_string())
}

fn media_kind_from_url(url: &str) -> Option<&'static str> {
    let lower = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    if lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".webp")
        || lower.ends_with(".gif")
    {
        Some("Image")
    } else if lower.ends_with(".mp3")
        || lower.ends_with(".wav")
        || lower.ends_with(".flac")
        || lower.ends_with(".ogg")
        || lower.ends_with(".m4a")
    {
        Some("Audio")
    } else if lower.ends_with(".mp4")
        || lower.ends_with(".webm")
        || lower.ends_with(".mov")
        || lower.ends_with(".mkv")
    {
        Some("Video")
    } else {
        None
    }
}

fn looks_like_media_url(url: &str, fallback_kind: &str) -> bool {
    media_kind_from_url(url).is_some() || fallback_kind == "Image"
}

fn image_looks_like_site_chrome(media_url: &str, element: &ElementRef<'_>) -> bool {
    let descriptor = [
        media_url,
        element.value().attr("alt").unwrap_or_default(),
        element.value().attr("title").unwrap_or_default(),
        element.value().attr("aria-label").unwrap_or_default(),
        element.value().attr("class").unwrap_or_default(),
        element.value().attr("id").unwrap_or_default(),
    ]
    .join(" ");
    let width = numeric_attr(element, "width");
    let height = numeric_attr(element, "height");
    image_metadata_looks_like_site_chrome(&descriptor, width, height)
}

fn image_metadata_looks_like_site_chrome(
    descriptor: &str,
    width: Option<u32>,
    height: Option<u32>,
) -> bool {
    let descriptor = descriptor.to_ascii_lowercase();
    let strong_chrome_tokens = [
        "flag",
        "flags",
        "language",
        "locale",
        "i18n",
        "translate",
        "favicon",
        "sprite",
        "spinner",
        "placeholder",
        "pixel",
        "tracking",
    ];
    if contains_any_token(&descriptor, &strong_chrome_tokens) {
        return true;
    }

    let small_chrome_tokens = ["icon", "logo", "badge"];
    contains_any_token(&descriptor, &small_chrome_tokens)
        && width
            .zip(height)
            .map(|(width, height)| width <= 160 && height <= 160)
            .unwrap_or(false)
}

fn contains_any_token(value: &str, needles: &[&str]) -> bool {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|token| needles.iter().any(|needle| token == *needle))
}

fn numeric_attr(element: &ElementRef<'_>, attribute: &str) -> Option<u32> {
    element.value().attr(attribute)?.trim().parse::<u32>().ok()
}

fn matches_media_focus(source_media_kind: &str, item_kind: &str) -> bool {
    source_media_kind.eq_ignore_ascii_case("mixed")
        || source_media_kind.eq_ignore_ascii_case(item_kind)
}

fn matches_requested_media(requested_media_kinds: &[String], item_kind: &str) -> bool {
    requested_media_kinds
        .iter()
        .any(|kind| kind.eq_ignore_ascii_case(item_kind))
}

fn first_non_empty(values: &[Option<&str>]) -> Option<String> {
    values
        .iter()
        .find_map(|value| clean_title(value.unwrap_or_default()))
}

fn clean_title(raw: &str) -> Option<String> {
    let title = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() { None } else { Some(title) }
}

fn title_from_url(url: &str, fallback: &str) -> String {
    let file_name = url
        .split('?')
        .next()
        .unwrap_or(url)
        .rsplit('/')
        .next()
        .unwrap_or(fallback)
        .trim();
    clean_title(&file_name.replace(['-', '_'], " "))
        .unwrap_or_else(|| format!("Untitled {fallback}"))
}
