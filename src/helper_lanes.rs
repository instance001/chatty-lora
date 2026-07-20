use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    helper::providers,
    state::ProjectPaths,
    types::{
        HelperLaneActivityItem, HelperLaneEntryInput, HelperLaneRegistryPayload,
        HelperLaneSummary, HelperLaneUpdateRequest,
    },
};

const HELPER_LANES_FILE: &str = "helper_lanes.json";
const LOCAL_RULE_BASED_LANE_ID: &str = "local-rule-based";
const CLEAR_API_KEY_SENTINEL: &str = "__CLEAR__";
const MAX_RECENT_ACTIVITY: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HelperLaneConfig {
    selected_lane_id: String,
    lanes: Vec<HelperLaneStoredEntry>,
    #[serde(default)]
    recent_activity: Vec<StoredHelperLaneActivity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HelperLaneStoredEntry {
    id: String,
    label: String,
    lane_mode: String,
    provider_kind: String,
    model_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    metadata: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_verified_unix_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_verification_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_verification_note: Option<String>,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredHelperLaneActivity {
    unix_seconds: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lane_id: Option<String>,
    lane_label: String,
    action: String,
    note: String,
}

#[derive(Debug, Clone)]
pub struct HelperLaneResolution {
    pub lane_id: String,
    pub lane_label: String,
    pub lane_mode: String,
    pub provider_kind: String,
    pub model_name: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub supports_remote_inference: bool,
    pub lane_status_note: String,
}

pub fn load_registry_payload(paths: &ProjectPaths) -> Result<HelperLaneRegistryPayload> {
    let config = load_config(paths)?;
    let selected = select_lane(&config);
    let supports_remote_inference = lane_supports_remote_inference(selected);

    Ok(HelperLaneRegistryPayload {
        selected_lane_id: selected.id.clone(),
        selected_lane_label: selected.label.clone(),
        selected_lane_mode: selected.lane_mode.clone(),
        supports_remote_inference,
        notes: build_registry_notes(selected),
        lanes: config.lanes.iter().map(to_summary).collect(),
        recent_activity: config
            .recent_activity
            .iter()
            .map(|item| HelperLaneActivityItem {
                unix_seconds: item.unix_seconds,
                lane_id: item.lane_id.clone(),
                lane_label: item.lane_label.clone(),
                action: item.action.clone(),
                note: item.note.clone(),
            })
            .collect(),
    })
}

pub fn save_registry(
    paths: &ProjectPaths,
    request: HelperLaneUpdateRequest,
) -> Result<HelperLaneRegistryPayload> {
    let existing = load_config(paths).unwrap_or_else(|_| default_config());
    let mut lanes = request
        .lanes
        .into_iter()
        .map(to_stored_entry)
        .collect::<Vec<_>>();
    normalize_lanes(&mut lanes);
    merge_existing_lane_state(&mut lanes, &existing.lanes);

    let selected_lane_id = if lanes.iter().any(|lane| lane.id == request.selected_lane_id) {
        request.selected_lane_id
    } else {
        LOCAL_RULE_BASED_LANE_ID.to_string()
    };

    let config = HelperLaneConfig {
        selected_lane_id,
        lanes: lanes.clone(),
        recent_activity: {
            let mut recent = existing.recent_activity.clone();
            reconcile_recent_activity(&mut recent, &existing.lanes, &lanes);
            recent
        },
    };

    fs::create_dir_all(&paths.config)
        .with_context(|| format!("could not create {}", paths.config.display()))?;
    let output =
        serde_json::to_string_pretty(&config).context("could not serialize helper lanes")?;
    fs::write(paths.config.join(HELPER_LANES_FILE), output)
        .context("could not write config/helper_lanes.json")?;

    load_registry_payload(paths)
}

pub fn resolve_selected_lane(paths: &ProjectPaths) -> Result<HelperLaneResolution> {
    let config = load_config(paths)?;
    let selected = select_lane(&config);

    Ok(HelperLaneResolution {
        lane_id: selected.id.clone(),
        lane_label: selected.label.clone(),
        lane_mode: selected.lane_mode.clone(),
        provider_kind: selected.provider_kind.clone(),
        model_name: selected.model_name.clone(),
        base_url: selected.base_url.clone(),
        api_key: selected.api_key.clone(),
        supports_remote_inference: lane_supports_remote_inference(selected),
        lane_status_note: lane_status_note(selected),
    })
}

pub fn resolve_input_lane(input: HelperLaneEntryInput) -> Result<HelperLaneResolution> {
    validate_cloud_lane_input(&input)?;
    let mut lane = to_stored_entry(input);
    let mut lanes = vec![lane.clone()];
    normalize_lanes(&mut lanes);
    lane = lanes
        .into_iter()
        .next()
        .context("could not normalize helper lane input")?;

    Ok(HelperLaneResolution {
        lane_id: lane.id.clone(),
        lane_label: lane.label.clone(),
        lane_mode: lane.lane_mode.clone(),
        provider_kind: lane.provider_kind.clone(),
        model_name: lane.model_name.clone(),
        base_url: lane.base_url.clone(),
        api_key: lane.api_key.clone(),
        supports_remote_inference: lane_supports_remote_inference(&lane),
        lane_status_note: lane_status_note(&lane),
    })
}

pub fn record_verification_result(
    paths: &ProjectPaths,
    lane_id: &str,
    verification_status: &str,
    verification_note: &str,
    last_verified_unix_seconds: Option<u64>,
) -> Result<()> {
    let mut config = load_config(paths)?;
    let Some(lane) = config.lanes.iter_mut().find(|lane| lane.id == lane_id) else {
        return Ok(());
    };

    lane.last_verification_status = Some(verification_status.trim().to_string());
    lane.last_verification_note = Some(verification_note.trim().to_string());
    if last_verified_unix_seconds.is_some() {
        lane.last_verified_unix_seconds = last_verified_unix_seconds;
    }
    push_recent_activity(
        &mut config.recent_activity,
        StoredHelperLaneActivity {
            unix_seconds: unix_now(),
            lane_id: Some(lane.id.clone()),
            lane_label: lane.label.clone(),
            action: if verification_status.trim().eq_ignore_ascii_case("ready") {
                "verified".to_string()
            } else {
                "verification_failed".to_string()
            },
            note: verification_note.trim().to_string(),
        },
    );

    persist_config(paths, &config)
}

fn load_config(paths: &ProjectPaths) -> Result<HelperLaneConfig> {
    let config_path = paths.config.join(HELPER_LANES_FILE);
    let mut config = if config_path.exists() {
        let contents = fs::read_to_string(&config_path)
            .with_context(|| format!("could not read {}", config_path.display()))?;
        serde_json::from_str::<HelperLaneConfig>(&contents)
            .context("invalid config/helper_lanes.json")?
    } else {
        default_config()
    };

    normalize_lanes(&mut config.lanes);
    if !config
        .lanes
        .iter()
        .any(|lane| lane.id == config.selected_lane_id && lane.enabled)
    {
        config.selected_lane_id = LOCAL_RULE_BASED_LANE_ID.to_string();
    }
    Ok(config)
}

fn default_config() -> HelperLaneConfig {
    HelperLaneConfig {
        selected_lane_id: LOCAL_RULE_BASED_LANE_ID.to_string(),
        lanes: vec![local_rule_based_lane()],
        recent_activity: vec![StoredHelperLaneActivity {
            unix_seconds: unix_now(),
            lane_id: Some(LOCAL_RULE_BASED_LANE_ID.to_string()),
            lane_label: "Local page-aware helper".to_string(),
            action: "seeded".to_string(),
            note: "Built-in offline helper lane is available by default.".to_string(),
        }],
    }
}

fn local_rule_based_lane() -> HelperLaneStoredEntry {
    HelperLaneStoredEntry {
        id: LOCAL_RULE_BASED_LANE_ID.to_string(),
        label: "Local page-aware helper".to_string(),
        lane_mode: "local".to_string(),
        provider_kind: "local_builtin".to_string(),
        model_name: "Rule-based guidance".to_string(),
        base_url: None,
        metadata: None,
        api_key: None,
        last_verified_unix_seconds: Some(unix_now()),
        last_verification_status: Some("ready".to_string()),
        last_verification_note: Some(
            "Always available offline. Uses the built-in page-aware helper.".to_string(),
        ),
        enabled: true,
    }
}

fn normalize_lanes(lanes: &mut Vec<HelperLaneStoredEntry>) {
    for lane in lanes.iter_mut() {
        lane.id = slugify(&lane.id);
        lane.label = lane.label.trim().to_string();
        lane.lane_mode = normalize_lane_mode(&lane.lane_mode);
        lane.provider_kind = providers::normalize_provider_kind(&lane.provider_kind);
        lane.model_name = lane.model_name.trim().to_string();
        lane.base_url = clean_optional_text(lane.base_url.take());
        lane.metadata = clean_optional_metadata(lane.metadata.take());
        lane.api_key = clean_optional_text(lane.api_key.take());
        lane.last_verification_status = clean_optional_text(lane.last_verification_status.take());
        lane.last_verification_note = clean_optional_text(lane.last_verification_note.take());
        if lane.label.is_empty() {
            lane.label = if lane.id == LOCAL_RULE_BASED_LANE_ID {
                "Local page-aware helper".to_string()
            } else {
                "Unnamed helper lane".to_string()
            };
        }
        if lane.id == LOCAL_RULE_BASED_LANE_ID {
            lane.lane_mode = "local".to_string();
            lane.provider_kind = providers::LOCAL_BUILTIN_PROVIDER.to_string();
            lane.model_name = "Rule-based guidance".to_string();
            lane.base_url = None;
            lane.metadata = None;
            lane.api_key = None;
            lane.last_verified_unix_seconds = Some(unix_now());
            lane.last_verification_status = Some("ready".to_string());
            lane.last_verification_note = Some(
                "Always available offline. Uses the built-in page-aware helper.".to_string(),
            );
            lane.enabled = true;
        }
    }

    lanes.retain(|lane| !lane.id.is_empty() && !lane.label.is_empty());
    lanes.sort_by(|left, right| {
        lane_sort_rank(&left.id)
            .cmp(&lane_sort_rank(&right.id))
            .then_with(|| left.label.to_ascii_lowercase().cmp(&right.label.to_ascii_lowercase()))
    });
    lanes.dedup_by(|left, right| left.id == right.id);

    if !lanes.iter().any(|lane| lane.id == LOCAL_RULE_BASED_LANE_ID) {
        lanes.insert(0, local_rule_based_lane());
    }
}

fn lane_sort_rank(id: &str) -> u8 {
    if id == LOCAL_RULE_BASED_LANE_ID { 0 } else { 1 }
}

fn select_lane<'a>(config: &'a HelperLaneConfig) -> &'a HelperLaneStoredEntry {
    config
        .lanes
        .iter()
        .find(|lane| lane.id == config.selected_lane_id && lane.enabled)
        .or_else(|| config.lanes.iter().find(|lane| lane.id == LOCAL_RULE_BASED_LANE_ID))
        .or_else(|| config.lanes.iter().find(|lane| lane.enabled))
        .unwrap_or(&config.lanes[0])
}

fn to_summary(lane: &HelperLaneStoredEntry) -> HelperLaneSummary {
    HelperLaneSummary {
        id: lane.id.clone(),
        label: lane.label.clone(),
        lane_mode: lane.lane_mode.clone(),
        provider_kind: lane.provider_kind.clone(),
        model_name: lane.model_name.clone(),
        base_url: lane.base_url.clone(),
        metadata: lane.metadata.clone(),
        enabled: lane.enabled,
        supports_remote_inference: lane_supports_remote_inference(lane),
        has_api_key: lane
            .api_key
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false),
        verification_status: verification_status(lane),
        verification_note: verification_note(lane),
        last_verified_unix_seconds: lane.last_verified_unix_seconds,
        source: if lane.id == LOCAL_RULE_BASED_LANE_ID {
            "built-in".to_string()
        } else {
            "host-managed".to_string()
        },
    }
}

fn lane_supports_remote_inference(lane: &HelperLaneStoredEntry) -> bool {
    providers::lane_supports_remote_inference(
        &lane.lane_mode,
        &lane.provider_kind,
        &lane.model_name,
        lane.base_url.as_deref(),
        lane.api_key.as_deref(),
    )
}

fn verification_status(lane: &HelperLaneStoredEntry) -> String {
    if let Some(status) = lane
        .last_verification_status
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return status.to_string();
    }
    if lane.id == LOCAL_RULE_BASED_LANE_ID {
        return "ready".to_string();
    }
    if lane.lane_mode == "local" {
        return "parallel-local".to_string();
    }
    if lane_supports_remote_inference(lane) {
        "configured".to_string()
    } else {
        "needs-setup".to_string()
    }
}

fn verification_note(lane: &HelperLaneStoredEntry) -> String {
    if let Some(note) = lane
        .last_verification_note
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return note.to_string();
    }
    if lane.id == LOCAL_RULE_BASED_LANE_ID {
        return "Always available offline. Uses the built-in page-aware helper.".to_string();
    }
    if lane.lane_mode == "local" {
        return "Reserved for future local helper runtimes that should sit beside cloud lanes without changing the helper UI philosophy.".to_string();
    }
    if lane_supports_remote_inference(lane) {
        if providers::provider_supports_live_helper(lane.provider_kind.as_str()) {
            "Provider, model, and API key are present. This helper lane can now attempt live remote guidance and will fall back to the local helper if the provider call fails.".to_string()
        } else {
            "Provider, model, and API key are present, but this provider adapter is not wired yet, so the lane still falls back to local guidance.".to_string()
        }
    } else {
        providers::setup_incomplete_note(&lane.provider_kind)
    }
}

fn build_registry_notes(selected: &HelperLaneStoredEntry) -> Vec<String> {
    let mut notes = vec![
        "Helper lanes are host-owned and live in config/helper_lanes.json rather than inside training plans.".to_string(),
        "Local and cloud lanes stay parallel here. Training, datasets, and saved LoRA outputs remain local-first.".to_string(),
    ];
    notes.push(format!(
        "Current helper lane: {} ({}, provider {}).",
        selected.label, selected.lane_mode, selected.provider_kind
    ));
    notes.push(lane_status_note(selected));
    notes
}

fn lane_status_note(lane: &HelperLaneStoredEntry) -> String {
    if lane.id == LOCAL_RULE_BASED_LANE_ID {
        "Using the built-in local helper lane for offline page-aware guidance.".to_string()
    } else if lane_supports_remote_inference(lane) {
        if providers::provider_supports_live_helper(lane.provider_kind.as_str()) {
            format!(
                "Cloud helper lane \"{}\" is configured and can attempt live remote helper answers through the {} adapter, with automatic fallback to local guidance if the request fails.",
                lane.label, lane.provider_kind
            )
        } else {
            format!(
                "Cloud helper lane \"{}\" is configured, but provider \"{}\" is not wired yet, so helper answers still fall back to the local guidance core.",
                lane.label, lane.provider_kind
            )
        }
    } else {
        format!(
            "Cloud helper lane \"{}\" is selected, but it is not fully configured yet, so helper answers fall back to the local guidance core.",
            lane.label
        )
    }
}

fn to_stored_entry(input: HelperLaneEntryInput) -> HelperLaneStoredEntry {
    let api_key = match input.api_key.as_deref().map(str::trim) {
        Some(CLEAR_API_KEY_SENTINEL) => None,
        Some(value) if !value.is_empty() => Some(value.to_string()),
        _ => None,
    };

    HelperLaneStoredEntry {
        id: input.id,
        label: input.label,
        lane_mode: input.lane_mode,
        provider_kind: input.provider_kind,
        model_name: input.model_name,
        base_url: input.base_url,
        metadata: input.metadata,
        api_key,
        last_verified_unix_seconds: None,
        last_verification_status: None,
        last_verification_note: None,
        enabled: input.enabled,
    }
}

fn merge_existing_lane_state(
    lanes: &mut [HelperLaneStoredEntry],
    existing_lanes: &[HelperLaneStoredEntry],
) {
    for lane in lanes.iter_mut() {
        let Some(existing) = existing_lanes.iter().find(|candidate| candidate.id == lane.id) else {
            continue;
        };

        if lane.api_key.is_none() {
            lane.api_key = existing.api_key.clone();
        }
        if lane.metadata.is_none() {
            lane.metadata = existing.metadata.clone();
        }

        if lane_signature(lane) == lane_signature(existing) && lane.api_key == existing.api_key {
            lane.last_verified_unix_seconds = existing.last_verified_unix_seconds;
            lane.last_verification_status = existing.last_verification_status.clone();
            lane.last_verification_note = existing.last_verification_note.clone();
        } else {
            lane.last_verified_unix_seconds = None;
            lane.last_verification_status = None;
            lane.last_verification_note = None;
        }
    }
}

fn reconcile_recent_activity(
    recent: &mut Vec<StoredHelperLaneActivity>,
    before: &[HelperLaneStoredEntry],
    after: &[HelperLaneStoredEntry],
) {
    let before_map = before
        .iter()
        .map(|lane| (lane.id.clone(), lane))
        .collect::<std::collections::BTreeMap<_, _>>();
    let after_map = after
        .iter()
        .map(|lane| (lane.id.clone(), lane))
        .collect::<std::collections::BTreeMap<_, _>>();

    for (id, lane) in &after_map {
        match before_map.get(id) {
            None => push_recent_activity(
                recent,
                StoredHelperLaneActivity {
                    unix_seconds: unix_now(),
                    lane_id: Some(id.clone()),
                    lane_label: lane.label.clone(),
                    action: "added".to_string(),
                    note: format!("Added {} helper lane for provider {}.", lane.lane_mode, lane.provider_kind),
                },
            ),
            Some(previous) => {
                if previous.enabled != lane.enabled {
                    push_recent_activity(
                        recent,
                        StoredHelperLaneActivity {
                            unix_seconds: unix_now(),
                            lane_id: Some(id.clone()),
                            lane_label: lane.label.clone(),
                            action: if lane.enabled { "enabled" } else { "disabled" }.to_string(),
                            note: format!(
                                "{} helper lane \"{}\".",
                                if lane.enabled { "Enabled" } else { "Disabled" },
                                lane.label
                            ),
                        },
                    );
                }
                if previous.api_key != lane.api_key {
                    let (action, note) = match (
                        previous.api_key.as_ref().map(|value| !value.is_empty()).unwrap_or(false),
                        lane.api_key.as_ref().map(|value| !value.is_empty()).unwrap_or(false),
                    ) {
                        (false, true) => ("key_saved", format!("Saved a new API key for \"{}\".", lane.label)),
                        (true, true) => ("key_replaced", format!("Replaced the stored API key for \"{}\".", lane.label)),
                        (true, false) => ("key_cleared", format!("Cleared the stored API key for \"{}\".", lane.label)),
                        _ => ("updated", format!("Updated helper lane \"{}\".", lane.label)),
                    };
                    push_recent_activity(
                        recent,
                        StoredHelperLaneActivity {
                            unix_seconds: unix_now(),
                            lane_id: Some(id.clone()),
                            lane_label: lane.label.clone(),
                            action: action.to_string(),
                            note,
                        },
                    );
                } else if lane_signature(previous) != lane_signature(lane) {
                    push_recent_activity(
                        recent,
                        StoredHelperLaneActivity {
                            unix_seconds: unix_now(),
                            lane_id: Some(id.clone()),
                            lane_label: lane.label.clone(),
                            action: "updated".to_string(),
                            note: format!("Updated helper lane settings for \"{}\".", lane.label),
                        },
                    );
                }
            }
        }
    }

    for (id, lane) in &before_map {
        if !after_map.contains_key(id) {
            push_recent_activity(
                recent,
                StoredHelperLaneActivity {
                    unix_seconds: unix_now(),
                    lane_id: Some(id.clone()),
                    lane_label: lane.label.clone(),
                    action: "deleted".to_string(),
                    note: format!("Deleted helper lane \"{}\".", lane.label),
                },
            );
        }
    }
}

fn push_recent_activity(recent: &mut Vec<StoredHelperLaneActivity>, item: StoredHelperLaneActivity) {
    recent.insert(0, item);
    if recent.len() > MAX_RECENT_ACTIVITY {
        recent.truncate(MAX_RECENT_ACTIVITY);
    }
}

fn lane_signature(lane: &HelperLaneStoredEntry) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        lane.lane_mode.trim(),
        lane.provider_kind.trim(),
        lane.model_name.trim(),
        lane.base_url.as_deref().unwrap_or("").trim(),
        lane.metadata
            .as_ref()
            .and_then(|value| serde_json::to_string(value).ok())
            .unwrap_or_default(),
        lane.enabled
    )
}

fn normalize_lane_mode(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "cloud" => "cloud".to_string(),
        _ => "local".to_string(),
    }
}

fn persist_config(paths: &ProjectPaths, config: &HelperLaneConfig) -> Result<()> {
    fs::create_dir_all(&paths.config)
        .with_context(|| format!("could not create {}", paths.config.display()))?;
    let output =
        serde_json::to_string_pretty(config).context("could not serialize helper lanes")?;
    fs::write(paths.config.join(HELPER_LANES_FILE), output)
        .context("could not write config/helper_lanes.json")?;
    Ok(())
}

fn clean_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn clean_optional_metadata(value: Option<Value>) -> Option<Value> {
    match value {
        Some(Value::Null) => None,
        Some(Value::Object(map)) if map.is_empty() => None,
        other => other,
    }
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

#[allow(dead_code)]
pub fn validate_cloud_lane_input(input: &HelperLaneEntryInput) -> Result<()> {
    if input.lane_mode.trim().eq_ignore_ascii_case("cloud") && input.label.trim().is_empty() {
        bail!("Cloud helper lanes need a display label.");
    }
    Ok(())
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
