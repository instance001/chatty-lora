use anyhow::{Context, Result, bail};

use crate::types::{HelperProviderCatalogEntry, HelperProviderCatalogPayload};

pub(crate) const LOCAL_BUILTIN_PROVIDER: &str = "local_builtin";

pub(crate) fn normalize_provider_kind(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "openai" => "openai".to_string(),
        "openai_compatible" => "openai_compatible".to_string(),
        "anthropic" => "anthropic".to_string(),
        "gemini" => "gemini".to_string(),
        "xai" => "xai".to_string(),
        "other" => "other".to_string(),
        LOCAL_BUILTIN_PROVIDER => LOCAL_BUILTIN_PROVIDER.to_string(),
        _ => "other".to_string(),
    }
}

pub(crate) fn provider_supports_live_helper(provider_kind: &str) -> bool {
    matches!(
        normalize_provider_kind(provider_kind).as_str(),
        "openai" | "openai_compatible" | "anthropic" | "gemini" | "xai"
    )
}

pub(crate) fn provider_default_base_url(provider_kind: &str) -> Option<&'static str> {
    match normalize_provider_kind(provider_kind).as_str() {
        "openai" => Some("https://api.openai.com/v1"),
        "anthropic" => Some("https://api.anthropic.com"),
        "gemini" => Some("https://generativelanguage.googleapis.com/v1beta"),
        "xai" => Some("https://api.x.ai/v1"),
        _ => None,
    }
}

pub(crate) fn provider_requires_base_url(provider_kind: &str) -> bool {
    normalize_provider_kind(provider_kind) == "openai_compatible"
}

pub(crate) fn lane_supports_remote_inference(
    lane_mode: &str,
    provider_kind: &str,
    model_name: &str,
    base_url: Option<&str>,
    api_key: Option<&str>,
) -> bool {
    lane_mode.trim().eq_ignore_ascii_case("cloud")
        && !model_name.trim().is_empty()
        && api_key.map(|value| !value.trim().is_empty()).unwrap_or(false)
        && (!provider_requires_base_url(provider_kind)
            || base_url.map(|value| !value.trim().is_empty()).unwrap_or(false))
}

pub(crate) fn setup_incomplete_note(provider_kind: &str) -> String {
    match normalize_provider_kind(provider_kind).as_str() {
        "openai" => "Add an OpenAI model name and API key to finish this cloud helper lane."
            .to_string(),
        "openai_compatible" => "Add a model name and base URL for this OpenAI-compatible lane, then save an API key if the host requires one."
            .to_string(),
        "anthropic" => {
            "Add an Anthropic model name and API key to finish this cloud helper lane."
                .to_string()
        }
        "gemini" => {
            "Add a Gemini model name and API key to finish this cloud helper lane.".to_string()
        }
        "xai" => "Add an xAI model name and API key to finish this cloud helper lane."
            .to_string(),
        _ => "Add a provider model name for this cloud helper lane. Base URL and API key may also be needed."
            .to_string(),
    }
}

pub(crate) fn normalized_base_url(provider_kind: &str, base_url: Option<&str>) -> Result<String> {
    let normalized = normalize_provider_kind(provider_kind);
    match normalized.as_str() {
        "openai" | "anthropic" | "gemini" | "xai" => Ok(base_url
            .filter(|value| !value.trim().is_empty())
            .or_else(|| provider_default_base_url(&normalized))
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .context("provider is missing its default base URL")?),
        "openai_compatible" => base_url
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .context("openai-compatible lanes need a base URL"),
        other => bail!("provider {} is not wired for live helper calls", other),
    }
}

pub(crate) fn catalog_payload() -> HelperProviderCatalogPayload {
    HelperProviderCatalogPayload {
        providers: vec![
            catalog_entry("local_builtin"),
            catalog_entry("openai"),
            catalog_entry("openai_compatible"),
            catalog_entry("anthropic"),
            catalog_entry("gemini"),
            catalog_entry("xai"),
            catalog_entry("other"),
        ],
    }
}

fn catalog_entry(provider_kind: &str) -> HelperProviderCatalogEntry {
    let kind = normalize_provider_kind(provider_kind);
    match kind.as_str() {
        "local_builtin" => HelperProviderCatalogEntry {
            kind,
            label: "Local built-in".to_string(),
            live_helper_supported: false,
            base_url_mode: "hidden".to_string(),
            api_key_mode: "hidden".to_string(),
            default_base_url: None,
            example_model: Some("Rule-based guidance".to_string()),
            model_placeholder: "Rule-based guidance".to_string(),
            base_url_placeholder: "".to_string(),
            setup_note: "This lane stays local on this machine and does not call a remote provider."
                .to_string(),
            draft_verification_note:
                "This local helper lane is ready to answer without a remote API key."
                    .to_string(),
        },
        "openai" => HelperProviderCatalogEntry {
            kind,
            label: "OpenAI".to_string(),
            live_helper_supported: true,
            base_url_mode: "optional".to_string(),
            api_key_mode: "required".to_string(),
            default_base_url: provider_default_base_url("openai").map(str::to_string),
            example_model: Some("gpt-4.1-mini".to_string()),
            model_placeholder: "e.g. gpt-4.1-mini".to_string(),
            base_url_placeholder: "https://api.openai.com/v1".to_string(),
            setup_note:
                "Use an OpenAI chat model name and an API key. The default base URL is usually correct."
                    .to_string(),
            draft_verification_note:
                "Add an OpenAI model name and API key to finish this cloud helper lane."
                    .to_string(),
        },
        "openai_compatible" => HelperProviderCatalogEntry {
            kind,
            label: "OpenAI-compatible".to_string(),
            live_helper_supported: true,
            base_url_mode: "required".to_string(),
            api_key_mode: "recommended".to_string(),
            default_base_url: None,
            example_model: Some("mistral-small".to_string()),
            model_placeholder: "e.g. mistral-small, llama-3.1-70b-instruct".to_string(),
            base_url_placeholder: "https://api.example.com/v1".to_string(),
            setup_note:
                "Point this lane at any OpenAI-compatible endpoint. Base URL is required here because hosts vary."
                    .to_string(),
            draft_verification_note:
                "Add a model name and base URL for this OpenAI-compatible lane, then save an API key if the host requires one."
                    .to_string(),
        },
        "anthropic" => HelperProviderCatalogEntry {
            kind,
            label: "Anthropic".to_string(),
            live_helper_supported: true,
            base_url_mode: "optional".to_string(),
            api_key_mode: "required".to_string(),
            default_base_url: provider_default_base_url("anthropic").map(str::to_string),
            example_model: Some("claude-sonnet-4-20250514".to_string()),
            model_placeholder: "e.g. claude-sonnet-4-20250514".to_string(),
            base_url_placeholder: "https://api.anthropic.com".to_string(),
            setup_note:
                "Use an Anthropic model name and API key. The default Anthropic API host is usually correct."
                    .to_string(),
            draft_verification_note:
                "Add an Anthropic model name and API key to finish this cloud helper lane."
                    .to_string(),
        },
        "gemini" => HelperProviderCatalogEntry {
            kind,
            label: "Gemini".to_string(),
            live_helper_supported: true,
            base_url_mode: "optional".to_string(),
            api_key_mode: "required".to_string(),
            default_base_url: provider_default_base_url("gemini").map(str::to_string),
            example_model: Some("gemini-2.5-flash".to_string()),
            model_placeholder: "e.g. gemini-2.5-flash".to_string(),
            base_url_placeholder: "https://generativelanguage.googleapis.com".to_string(),
            setup_note:
                "Use a Gemini model name and API key. The default Google Generative Language host is usually correct."
                    .to_string(),
            draft_verification_note:
                "Add a Gemini model name and API key to finish this cloud helper lane."
                    .to_string(),
        },
        "xai" => HelperProviderCatalogEntry {
            kind,
            label: "xAI".to_string(),
            live_helper_supported: true,
            base_url_mode: "optional".to_string(),
            api_key_mode: "required".to_string(),
            default_base_url: provider_default_base_url("xai").map(str::to_string),
            example_model: Some("grok-4-0709".to_string()),
            model_placeholder: "e.g. grok-4-0709".to_string(),
            base_url_placeholder: "https://api.x.ai/v1".to_string(),
            setup_note:
                "Use an xAI model name and API key. The default xAI API host is usually correct."
                    .to_string(),
            draft_verification_note:
                "Add an xAI model name and API key to finish this cloud helper lane."
                    .to_string(),
        },
        _ => HelperProviderCatalogEntry {
            kind,
            label: "Other".to_string(),
            live_helper_supported: false,
            base_url_mode: "optional".to_string(),
            api_key_mode: "recommended".to_string(),
            default_base_url: None,
            example_model: None,
            model_placeholder: "Enter the provider model name".to_string(),
            base_url_placeholder: "https://api.example.com/v1".to_string(),
            setup_note:
                "Use this when you are staging a lane for a future provider. Verification will stay limited until backend support exists."
                    .to_string(),
            draft_verification_note:
                "Add a provider model name for this cloud helper lane. Base URL and API key may also be needed."
                    .to_string(),
        },
    }
}
