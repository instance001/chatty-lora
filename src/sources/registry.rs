use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::{
    state::ProjectPaths,
    types::{GenericGalleryProfile, SourceEntry, SourceRegistryUpdateRequest},
};

const DEFAULT_SOURCES_FILE: &str = "sources.json";

pub fn load_sources(paths: &ProjectPaths) -> Result<Vec<SourceEntry>> {
    let config_path = paths.config.join(DEFAULT_SOURCES_FILE);
    if config_path.exists() {
        let contents = fs::read_to_string(&config_path)
            .with_context(|| format!("could not read {}", config_path.display()))?;
        let mut sources: Vec<SourceEntry> =
            serde_json::from_str(&contents).context("invalid config/sources.json")?;
        normalize_sources(&mut sources);
        return Ok(sources);
    }

    let default_path = paths.defaults.join(DEFAULT_SOURCES_FILE);
    if default_path.exists() {
        let contents = fs::read_to_string(&default_path)
            .with_context(|| format!("could not read {}", default_path.display()))?;
        let mut sources: Vec<SourceEntry> =
            serde_json::from_str(&contents).context("invalid defaults/sources.json")?;
        normalize_sources(&mut sources);
        return Ok(sources);
    }

    let mut sources = builtin_defaults();
    normalize_sources(&mut sources);
    Ok(sources)
}

pub fn save_sources(paths: &ProjectPaths, request: SourceRegistryUpdateRequest) -> Result<()> {
    fs::create_dir_all(&paths.config)
        .with_context(|| format!("could not create {}", paths.config.display()))?;
    let mut sources = request.sources;
    normalize_sources(&mut sources);
    let output = serde_json::to_string_pretty(&sources).context("could not serialize sources")?;
    fs::write(paths.config.join(DEFAULT_SOURCES_FILE), output)
        .context("could not write config/sources.json")?;
    Ok(())
}

fn normalize_sources(sources: &mut Vec<SourceEntry>) {
    sources.retain(|source| !source.id.trim().is_empty() && !source.name.trim().is_empty());
    for source in sources.iter_mut() {
        source.id = slugify(&source.id);
        if source.pages_per_batch == 0 {
            source.pages_per_batch = 3;
        }
        if source.crawl_delay_ms < 500 {
            source.crawl_delay_ms = 500;
        }
        if source.notes.trim().is_empty() {
            source.notes = "No notes yet.".to_string();
        }
    }

    sources.sort_by(|left, right| {
        right.enabled.cmp(&left.enabled).then_with(|| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        })
    });
    sources.dedup_by(|left, right| left.id == right.id);
}

fn slugify(input: &str) -> String {
    input
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

fn builtin_defaults() -> Vec<SourceEntry> {
    vec![
        SourceEntry {
            id: "openverse-images".to_string(),
            name: "Openverse Images".to_string(),
            base_url: "https://api.openverse.org/v1/images/".to_string(),
            adapter_kind: "openverse_images".to_string(),
            media_kind: "image".to_string(),
            enabled: true,
            user_added: false,
            crawl_delay_ms: 1200,
            pages_per_batch: 3,
            respect_robots_txt: true,
            notes: "Free-use image search via the public Openverse API.".to_string(),
            site_profile: None,
        },
        SourceEntry {
            id: "wikimedia-commons".to_string(),
            name: "Wikimedia Commons".to_string(),
            base_url: "https://commons.wikimedia.org/w/api.php".to_string(),
            adapter_kind: "wikimedia_commons".to_string(),
            media_kind: "image".to_string(),
            enabled: true,
            user_added: false,
            crawl_delay_ms: 1500,
            pages_per_batch: 3,
            respect_robots_txt: true,
            notes: "Public media repository with image search routed through the Wikimedia API."
                .to_string(),
            site_profile: None,
        },
        SourceEntry {
            id: "pexels-images".to_string(),
            name: "Pexels Images".to_string(),
            base_url: "https://www.pexels.com/search/{query}/?page={page}".to_string(),
            adapter_kind: "generic_gallery_html".to_string(),
            media_kind: "image".to_string(),
            enabled: true,
            user_added: false,
            crawl_delay_ms: 1500,
            pages_per_batch: 3,
            respect_robots_txt: true,
            notes: "Public Pexels photo search using the generic gallery adapter. Best-effort HTML extraction.".to_string(),
            site_profile: Some(GenericGalleryProfile {
                item_selector: None,
                media_selector: "img[src*='images.pexels.com/photos/']".to_string(),
                media_attribute: "src".to_string(),
                title_selector: None,
                title_attribute: None,
                thumbnail_selector: None,
                thumbnail_attribute: None,
                link_selector: None,
                link_attribute: None,
                media_url_template: None,
                thumbnail_url_template: None,
                title_template: None,
                source_page_url_template: None,
            }),
        },
        SourceEntry {
            id: "pexels-videos".to_string(),
            name: "Pexels Videos".to_string(),
            base_url: "https://www.pexels.com/search/videos/{query}/?page={page}".to_string(),
            adapter_kind: "generic_gallery_html".to_string(),
            media_kind: "video".to_string(),
            enabled: true,
            user_added: false,
            crawl_delay_ms: 1500,
            pages_per_batch: 3,
            respect_robots_txt: true,
            notes: "Public Pexels video search using the generic gallery adapter. Best-effort HTML extraction through visible download links.".to_string(),
            site_profile: Some(GenericGalleryProfile {
                item_selector: None,
                media_selector: "a[href*='/download/video/']".to_string(),
                media_attribute: "href".to_string(),
                title_selector: None,
                title_attribute: None,
                thumbnail_selector: None,
                thumbnail_attribute: None,
                link_selector: None,
                link_attribute: None,
                media_url_template: None,
                thumbnail_url_template: Some("https://images.pexels.com/videos/{media_id}/pexels-photo-{media_id}.jpeg?auto=compress&cs=tinysrgb&dpr=1&w=500".to_string()),
                title_template: Some("Pexels video {media_id}".to_string()),
                source_page_url_template: None,
            }),
        },
        SourceEntry {
            id: "pixabay-images".to_string(),
            name: "Pixabay Images".to_string(),
            base_url: "https://pixabay.com/images/search/{query}/?pagi={page}".to_string(),
            adapter_kind: "generic_gallery_html".to_string(),
            media_kind: "image".to_string(),
            enabled: true,
            user_added: false,
            crawl_delay_ms: 1500,
            pages_per_batch: 3,
            respect_robots_txt: true,
            notes: "Public Pixabay image search using the generic gallery adapter. Best-effort HTML extraction.".to_string(),
            site_profile: Some(GenericGalleryProfile {
                item_selector: None,
                media_selector: "img[src*='cdn.pixabay.com/photo/']".to_string(),
                media_attribute: "src".to_string(),
                title_selector: None,
                title_attribute: None,
                thumbnail_selector: None,
                thumbnail_attribute: None,
                link_selector: None,
                link_attribute: None,
                media_url_template: None,
                thumbnail_url_template: None,
                title_template: None,
                source_page_url_template: None,
            }),
        },
        SourceEntry {
            id: "openverse-audio".to_string(),
            name: "Openverse Audio".to_string(),
            base_url: "https://api.openverse.org/v1/audio/".to_string(),
            adapter_kind: "openverse_audio".to_string(),
            media_kind: "audio".to_string(),
            enabled: false,
            user_added: false,
            crawl_delay_ms: 1200,
            pages_per_batch: 3,
            respect_robots_txt: true,
            notes: "Free-use audio search via the public Openverse API.".to_string(),
            site_profile: None,
        },
    ]
}

#[allow(dead_code)]
fn _path_exists(path: &Path) -> bool {
    path.exists()
}
