pub mod adapters;
pub mod registry;
pub mod site_fix;

use anyhow::Result;
use reqwest::Client;

use crate::{
    state::ProjectPaths,
    types::{SearchPreviewRequest, SearchPreviewResponse, SourceFixSummary, SourceRegistryPayload},
};

pub fn load_registry_payload(paths: &ProjectPaths) -> Result<SourceRegistryPayload> {
    let sources = registry::load_sources(paths)?;
    let total = sources.len();
    let enabled = sources.iter().filter(|source| source.enabled).count();
    let custom = sources.iter().filter(|source| source.user_added).count();
    let search_ready = sources
        .iter()
        .filter(|source| adapters::adapter_ready(&source.adapter_kind))
        .count();

    Ok(SourceRegistryPayload {
        total,
        enabled,
        custom,
        search_ready,
        sources,
    })
}

pub async fn search_preview(
    client: &Client,
    paths: &ProjectPaths,
    request: SearchPreviewRequest,
) -> Result<SearchPreviewResponse> {
    let all_sources = registry::load_sources(paths)?;
    adapters::search_sources(client, all_sources, request).await
}

pub fn site_fix_summaries(paths: &ProjectPaths) -> Result<Vec<SourceFixSummary>> {
    site_fix::summaries(paths)
}
