pub(crate) mod providers;

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::Deserialize;

use crate::{
    helper::providers::normalized_base_url,
    helper_lanes::HelperLaneResolution,
    types::{
        BuilderHelperContext, HelperLaneVerifyResponse, HelperQueryRequest, HelperQueryResponse,
        MaterialsHelperContext,
    },
};

pub async fn answer(client: &Client, request: HelperQueryRequest, lane: &HelperLaneResolution) -> HelperQueryResponse {
    let local = local_answer(
        request,
        &lane.lane_id,
        &lane.lane_label,
        &lane.lane_mode,
        &lane.lane_status_note,
    );

    if !lane.supports_remote_inference || lane.lane_mode != "cloud" {
        return local;
    }

    match lane.provider_kind.as_str() {
        "openai" | "openai_compatible" | "xai" => match remote_chat_completions_answer(client, lane, &local).await {
            Ok(answer) => HelperQueryResponse {
                answer,
                lane_status_note: format!(
                    "{} Remote provider response succeeded for model {}.",
                    lane.lane_status_note, lane.model_name
                ),
                ..local
            },
            Err(error) => HelperQueryResponse {
                lane_status_note: format!(
                    "{} Remote helper call failed and fell back to local guidance: {}",
                    lane.lane_status_note, error
                ),
                ..local
            },
        },
        "anthropic" => match remote_anthropic_messages_answer(client, lane, &local).await {
            Ok(answer) => HelperQueryResponse {
                answer,
                lane_status_note: format!(
                    "{} Remote provider response succeeded for model {}.",
                    lane.lane_status_note, lane.model_name
                ),
                ..local
            },
            Err(error) => HelperQueryResponse {
                lane_status_note: format!(
                    "{} Remote helper call failed and fell back to local guidance: {}",
                    lane.lane_status_note, error
                ),
                ..local
            },
        },
        "gemini" => match remote_gemini_generate_content_answer(client, lane, &local).await {
            Ok(answer) => HelperQueryResponse {
                answer,
                lane_status_note: format!(
                    "{} Remote provider response succeeded for model {}.",
                    lane.lane_status_note, lane.model_name
                ),
                ..local
            },
            Err(error) => HelperQueryResponse {
                lane_status_note: format!(
                    "{} Remote helper call failed and fell back to local guidance: {}",
                    lane.lane_status_note, error
                ),
                ..local
            },
        },
        _ => HelperQueryResponse {
            lane_status_note: format!(
                "{} Provider {} is not wired yet, so this answer came from the local helper core.",
                lane.lane_status_note, lane.provider_kind
            ),
            ..local
        },
    }
}

pub async fn verify_lane(client: &Client, lane: &HelperLaneResolution) -> HelperLaneVerifyResponse {
    match verify_lane_inner(client, lane).await {
        Ok((note, last_verified_unix_seconds)) => HelperLaneVerifyResponse {
            lane_id: lane.lane_id.clone(),
            lane_label: lane.lane_label.clone(),
            verification_status: "ready".to_string(),
            verification_note: note,
            last_verified_unix_seconds,
        },
        Err(error) => HelperLaneVerifyResponse {
            lane_id: lane.lane_id.clone(),
            lane_label: lane.lane_label.clone(),
            verification_status: "failed".to_string(),
            verification_note: format!("Lane verification failed: {}", error),
            last_verified_unix_seconds: None,
        },
    }
}

pub fn local_answer(
    request: HelperQueryRequest,
    lane_id: &str,
    lane_label: &str,
    lane_mode: &str,
    lane_status_note: &str,
) -> HelperQueryResponse {
    let page = normalized_page(&request.page);
    let question = request.question.trim().to_string();

    match page {
        "builder" => answer_builder(
            question,
            request.builder,
            lane_id,
            lane_label,
            lane_mode,
            lane_status_note,
        ),
        _ => answer_materials(
            question,
            request.materials,
            lane_id,
            lane_label,
            lane_mode,
            lane_status_note,
        ),
    }
}

async fn remote_chat_completions_answer(
    client: &Client,
    lane: &HelperLaneResolution,
    local: &HelperQueryResponse,
) -> Result<String> {
    remote_chat_completions_text(client, lane, &build_remote_prompt(local)).await
}

async fn remote_chat_completions_text(
    client: &Client,
    lane: &HelperLaneResolution,
    prompt: &str,
) -> Result<String> {
    let api_key = lane
        .api_key
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .context("missing API key")?;
    let model_name = lane.model_name.trim();
    if model_name.is_empty() {
        bail!("missing remote helper model name");
    }

    let base_url = normalized_base_url(&lane.provider_kind, lane.base_url.as_deref())?;
    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let payload = serde_json::json!({
        "model": model_name,
        "messages": [
            {
                "role": "system",
                "content": "You are Chatty-lora's helper assistant. Keep answers practical, concise, and grounded in the supplied local app context. Do not imply cloud is required for LoRA training. If the local context suggests uncertainty, say so plainly."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.4
    });

    let response = client
        .post(endpoint)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .context("could not send remote helper request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "remote provider did not return a readable error body".to_string());
        bail!("provider returned {}: {}", status, truncate_for_note(&body, 220));
    }

    let payload = response
        .json::<OpenAiChatCompletionsResponse>()
        .await
        .context("could not parse remote helper response")?;
    let content = payload
        .choices
        .into_iter()
        .find_map(|choice| choice.message.content)
        .map(|content| content.trim().to_string())
        .filter(|content| !content.is_empty())
        .context("remote helper response was empty")?;

    Ok(content)
}

async fn remote_anthropic_messages_answer(
    client: &Client,
    lane: &HelperLaneResolution,
    local: &HelperQueryResponse,
) -> Result<String> {
    remote_anthropic_messages_text(client, lane, &build_remote_prompt(local)).await
}

async fn remote_anthropic_messages_text(
    client: &Client,
    lane: &HelperLaneResolution,
    prompt: &str,
) -> Result<String> {
    let api_key = lane
        .api_key
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .context("missing API key")?;
    let model_name = lane.model_name.trim();
    if model_name.is_empty() {
        bail!("missing remote helper model name");
    }

    let base_url = lane
        .base_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("https://api.anthropic.com");
    let endpoint = format!("{}/v1/messages", base_url.trim().trim_end_matches('/'));
    let payload = serde_json::json!({
        "model": model_name,
        "max_tokens": 700,
        "system": "You are Chatty-lora's helper assistant. Keep answers practical, concise, and grounded in the supplied local app context. Do not imply cloud is required for LoRA training. If the local context suggests uncertainty, say so plainly.",
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let response = client
        .post(endpoint)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await
        .context("could not send anthropic helper request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "remote provider did not return a readable error body".to_string());
        bail!("provider returned {}: {}", status, truncate_for_note(&body, 220));
    }

    let payload = response
        .json::<AnthropicMessagesResponse>()
        .await
        .context("could not parse anthropic helper response")?;

    let content = payload
        .content
        .into_iter()
        .filter(|block| block.kind == "text")
        .filter_map(|block| block.text)
        .map(|text| text.trim().to_string())
        .find(|text| !text.is_empty())
        .context("anthropic helper response was empty")?;

    Ok(content)
}

async fn remote_gemini_generate_content_answer(
    client: &Client,
    lane: &HelperLaneResolution,
    local: &HelperQueryResponse,
) -> Result<String> {
    remote_gemini_generate_content_text(client, lane, &build_remote_prompt(local)).await
}

async fn remote_gemini_generate_content_text(
    client: &Client,
    lane: &HelperLaneResolution,
    prompt: &str,
) -> Result<String> {
    let api_key = lane
        .api_key
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .context("missing API key")?;
    let model_name = lane.model_name.trim();
    if model_name.is_empty() {
        bail!("missing remote helper model name");
    }

    let base_url = lane
        .base_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("https://generativelanguage.googleapis.com/v1beta");
    let endpoint = format!(
        "{}/models/{}:generateContent",
        base_url.trim().trim_end_matches('/'),
        model_name
    );
    let payload = serde_json::json!({
        "contents": [
            {
                "role": "user",
                "parts": [
                    {
                        "text": format!(
                            "You are Chatty-lora's helper assistant. Keep answers practical, concise, and grounded in the supplied local app context. Do not imply cloud is required for LoRA training. If the local context suggests uncertainty, say so plainly.\n\n{}",
                            prompt
                        )
                    }
                ]
            }
        ]
    });

    let response = client
        .post(endpoint)
        .header("x-goog-api-key", api_key)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await
        .context("could not send gemini helper request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "remote provider did not return a readable error body".to_string());
        bail!("provider returned {}: {}", status, truncate_for_note(&body, 220));
    }

    let payload = response
        .json::<GeminiGenerateContentResponse>()
        .await
        .context("could not parse gemini helper response")?;

    let content = payload
        .candidates
        .into_iter()
        .find_map(|candidate| candidate.content)
        .into_iter()
        .flat_map(|content| content.parts.into_iter())
        .filter_map(|part| part.text)
        .map(|text| text.trim().to_string())
        .find(|text| !text.is_empty())
        .context("gemini helper response was empty")?;

    Ok(content)
}

fn build_remote_prompt(local: &HelperQueryResponse) -> String {
    format!(
        "Current helper lane: {} ({})\nLane note: {}\n\nPage: {}\nContext title: {}\n\nLocal guidance core:\n{}\n\nSuggestions:\n- {}\n\nRewrite that into one practical answer for the user. Keep it grounded in the same constraints, preserve the local-first training boundary, and avoid generic filler.",
        local.lane_label,
        local.lane_mode,
        local.lane_status_note,
        local.page,
        local.context_title,
        local.answer,
        local.suggestions.join("\n- ")
    )
}

async fn verify_lane_inner(client: &Client, lane: &HelperLaneResolution) -> Result<(String, Option<u64>)> {
    if lane.lane_id == "local-rule-based" || lane.lane_mode == "local" {
        return Ok((
            "Local helper lane is ready offline. No remote provider check was needed."
                .to_string(),
            Some(unix_now()),
        ));
    }

    if !lane.supports_remote_inference {
        bail!("this cloud lane is not fully configured yet");
    }

    let prompt = "Reply with one short sentence confirming that the Chatty-lora helper lane is reachable.";
    let reply = match lane.provider_kind.as_str() {
        "openai" | "openai_compatible" | "xai" => remote_chat_completions_text(client, lane, prompt).await?,
        "anthropic" => remote_anthropic_messages_text(client, lane, prompt).await?,
        "gemini" => remote_gemini_generate_content_text(client, lane, prompt).await?,
        other => bail!("provider {} is not wired for verification", other),
    };

    Ok((
        format!(
            "Remote helper lane responded successfully through provider {} using model {}. Sample reply: {}",
            lane.provider_kind,
            lane.model_name,
            truncate_for_note(&reply, 120)
        ),
        Some(unix_now()),
    ))
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn truncate_for_note(value: &str, limit: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= limit {
        return trimmed.to_string();
    }
    let shortened = trimmed.chars().take(limit).collect::<String>();
    format!("{}...", shortened)
}

#[derive(Debug, Deserialize)]
struct OpenAiChatCompletionsResponse {
    #[serde(default)]
    choices: Vec<OpenAiChatChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatChoice {
    message: OpenAiChatMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessagesResponse {
    #[serde(default)]
    content: Vec<AnthropicContentBlock>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiGenerateContentResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    #[serde(default)]
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

fn answer_materials(
    question: String,
    context: Option<MaterialsHelperContext>,
    lane_id: &str,
    lane_label: &str,
    lane_mode: &str,
    lane_status_note: &str,
) -> HelperQueryResponse {
    let context = context.unwrap_or(MaterialsHelperContext {
        search_query: String::new(),
        media_kinds: vec!["image".to_string(), "video".to_string()],
        enabled_source_names: Vec::new(),
        selected_preview_count: 0,
        preview_batch_loaded: false,
        input_file_count: 0,
        output_file_count: 0,
    });

    let lowered = question.to_ascii_lowercase();
    let context_title = if context.search_query.is_empty() {
        "Materials helper is looking at your source list and local library.".to_string()
    } else {
        format!(
            "Materials helper is looking at search term \"{}\" for {} across {} enabled source{}.",
            context.search_query,
            join_or_none(&media_kind_labels(&context.media_kinds)),
            context.enabled_source_names.len(),
            if context.enabled_source_names.len() == 1 {
                ""
            } else {
                "s"
            }
        )
    };

    let answer = if context.enabled_source_names.is_empty() {
        "No sources are enabled yet, so the next useful move is to tick one or two polite starter sources before searching. Keeping the source count small is the easiest way to stay respectful and keep preview batches fast."
            .to_string()
    } else if lowered.contains("nothing") || lowered.contains("empty") || lowered.contains("why") {
        if context.search_query.is_empty() {
            "The search box is empty, so Search runs in browse mode. That asks enabled sources for their first available media pages instead of a specific query. If browse mode returns nothing, the source probably needs a real listing URL, a {page} placeholder, or a source-specific selector profile."
                .to_string()
        } else if !context.preview_batch_loaded {
            "The preview batch has not landed yet, so the next checkpoint is whether the current site adapters can return a first three-page window for this exact query. If the query is good and the source is enabled, an empty result usually means the adapter needs refinement or the source simply has nothing relevant."
                .to_string()
        } else {
            format!(
                "The current batch loaded, so the crawler path is alive. If the results still feel wrong for \"{}\", I would first tighten the search phrase, then try a different enabled source before we even think about site-specific bugfix work.",
                context.search_query
            )
        }
    } else if lowered.contains("enough") || lowered.contains("dataset") {
        if context.selected_preview_count == 0 {
            "You do not have any preview items selected yet, so we are still in the discovery phase rather than the curation phase. A healthy next step is to select a small, coherent set first, then let the helper judge whether the resulting dataset looks broad enough."
                .to_string()
        } else {
            format!(
                "You currently have {} preview item{} selected. That is enough for a quick experimental dataset shell, but for a LoRA worth keeping we usually want a coherent set with enough variation in pose, framing, or context to teach the concept instead of memorising one scene.",
                context.selected_preview_count,
                if context.selected_preview_count == 1 {
                    ""
                } else {
                    "s"
                }
            )
        }
    } else if lowered.contains("source") || lowered.contains("site") {
        format!(
            "Right now you have {} enabled source{}: {}. My recommendation is to keep the active list small while you search, because that keeps previews punchy and makes it easier to tell whether a bad result is a query problem or one specific source adapter drifting.",
            context.enabled_source_names.len(),
            if context.enabled_source_names.len() == 1 {
                ""
            } else {
                "s"
            },
            join_or_none(&context.enabled_source_names)
        )
    } else {
        format!(
            "At this stage the Materials page is best used as a polite funnel: search a few enabled sources, preview only the first three pages, select the strongest items, then let Chatty-lora do the download and naming grunt work into a clean dataset folder. You currently have {} input file{} and {} output file{} already on disk, so the tray is ready to support that flow.",
            context.input_file_count,
            if context.input_file_count == 1 {
                ""
            } else {
                "s"
            },
            context.output_file_count,
            if context.output_file_count == 1 {
                ""
            } else {
                "s"
            }
        )
    };

    let suggestions = vec![
        "Keep only one or two sources enabled while testing a new search term.".to_string(),
        "Use direct noun phrases first, then get more specific only if the preview is too broad."
            .to_string(),
        "Curate a small coherent set before we worry about perfect scale.".to_string(),
    ];

    HelperQueryResponse {
        page: "materials".to_string(),
        answer,
        context_title,
        suggestions,
        lane_id: lane_id.to_string(),
        lane_label: lane_label.to_string(),
        lane_mode: lane_mode.to_string(),
        lane_status_note: lane_status_note.to_string(),
    }
}

fn media_kind_labels(media_kinds: &[String]) -> Vec<String> {
    let mut labels = media_kinds
        .iter()
        .map(|kind| match kind.as_str() {
            "image" => "images".to_string(),
            "video" => "video".to_string(),
            "audio" => "audio".to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>();
    labels.sort();
    labels.dedup();
    labels
}

fn answer_builder(
    question: String,
    context: Option<BuilderHelperContext>,
    lane_id: &str,
    lane_label: &str,
    lane_mode: &str,
    lane_status_note: &str,
) -> HelperQueryResponse {
    let context = context.unwrap_or(BuilderHelperContext {
        selected_dataset_slug: None,
        selected_dataset_file_count: None,
        selected_dataset_image_count: None,
        selected_dataset_audio_count: None,
        selected_dataset_video_count: None,
        prepared_project_count: 0,
        project_name: String::new(),
        base_model: String::new(),
        base_model_family_id: None,
        base_model_family_label: None,
        training_backend_id: "kohya_ss".to_string(),
        concept_type: "style".to_string(),
        training_preset: "balanced".to_string(),
        caption_strategy: "source-title".to_string(),
        rank: None,
        repeats: None,
        epochs: None,
        resolution: None,
        batch_size: None,
        learning_rate: None,
        validation_split_percent: None,
    });

    let lowered = question.to_ascii_lowercase();
    let dataset_label = context
        .selected_dataset_slug
        .clone()
        .unwrap_or_else(|| "no dataset selected yet".to_string());
    let base_model_family_id = context
        .base_model_family_id
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let base_model_family = context
        .base_model_family_label
        .clone()
        .unwrap_or_else(|| "unknown family".to_string());
    let context_title = format!(
        "Builder helper is looking at dataset \"{}\", backend \"{}\", base-model family \"{}\" ({}), concept type \"{}\", and preset \"{}\".",
        dataset_label,
        context.training_backend_id,
        base_model_family,
        base_model_family_id,
        context.concept_type,
        context.training_preset
    );

    let answer = if context.selected_dataset_slug.is_none() {
        "No curated dataset is selected yet, so the Builder is still waiting for training material. The cleanest next move is to curate one dataset from Materials, then come back here and let the training plan inherit that choice."
            .to_string()
    } else if lowered.contains("rank") {
        format!(
            "Rank controls how much detail the LoRA is allowed to hold onto. Higher rank can preserve more nuance, but it also makes the LoRA heavier and easier to overfit. With your current preset I would treat rank {} as a balanced starting point rather than something to push upward immediately.",
            context.rank.unwrap_or(32)
        )
    } else if lowered.contains("repeat") {
        format!(
            "Repeats control how many times each training example is revisited in one training cycle. With repeats at {}, you are telling the trainer to lean harder on this dataset. That can help small coherent sets, but it can get brittle fast if the data is narrow or messy.",
            context.repeats.unwrap_or(10)
        )
    } else if lowered.contains("epoch") {
        format!(
            "Epochs are full passes over the repeated dataset. At {} epoch{}, you are still in a sensible exploratory range. More is not automatically better; it often just means more chance to memorise quirks instead of learning the concept cleanly.",
            context.epochs.unwrap_or(8),
            if context.epochs.unwrap_or(8) == 1 {
                ""
            } else {
                "s"
            }
        )
    } else if lowered.contains("trigger") {
        if context.project_name.is_empty() {
            "A trigger phrase is the short anchor token you will later use to call the LoRA on purpose. Keep it distinctive, short, and unlikely to collide with normal language so you can summon the LoRA cleanly."
                .to_string()
        } else {
            format!(
                "Your trigger phrase wants to be the clean recall handle for project \"{}\". The best trigger phrases are distinctive and boring: short tokens that do not already have heavy meaning in the base model, especially once you settle on a {} family base.",
                context.project_name, base_model_family
            )
        }
    } else if lowered.contains("caption") {
        format!(
            "Caption strategy decides how much text guidance the trainer gets. \"{}\" is a practical starter because it keeps the pipeline moving, but if the source titles are noisy you may eventually want cleaner manual captions.",
            context.caption_strategy
        )
    } else if lowered.contains("backend")
        || lowered.contains("trainer")
        || lowered.contains("kohya")
        || lowered.contains("onetrainer")
        || lowered.contains("toolkit")
    {
        format!(
            "The current training plan is targeting backend \"{}\". Treat that as the trainer family this project is being prepared for, not a promise that it is already installed and ready to run. Right now the useful question is whether this backend matches your base model and concept type cleanly before we worry about the eventual execution layer.",
            context.training_backend_id
        )
    } else if lowered.contains("learning rate")
        || lowered.contains("batch")
        || lowered.contains("validation")
    {
        format!(
            "Right now the Builder is set to batch size {}, learning rate {}, and validation split {}%. These are setup controls rather than magic numbers: batch size mostly affects speed and memory, learning rate affects how aggressively the LoRA learns, and validation split gives you a cleaner check on whether the trainer is memorising instead of generalising.",
            context.batch_size.unwrap_or(1),
            context.learning_rate.unwrap_or(0.0001),
            context.validation_split_percent.unwrap_or(10)
        )
    } else if lowered.contains("enough") || lowered.contains("dataset") {
        let total = context.selected_dataset_file_count.unwrap_or(0);
        let images = context.selected_dataset_image_count.unwrap_or(0);
        let audio = context.selected_dataset_audio_count.unwrap_or(0);
        let video = context.selected_dataset_video_count.unwrap_or(0);
        format!(
            "The selected dataset currently exposes {} file{}: {} image{}, {} audio file{}, and {} video file{}. That is enough to scaffold a project and learn from the shape of the data, but the real question is coherence: do these files all teach the same concept in complementary ways rather than pulling the LoRA in different directions?",
            total,
            if total == 1 { "" } else { "s" },
            images,
            if images == 1 { "" } else { "s" },
            audio,
            if audio == 1 { "" } else { "s" },
            video,
            if video == 1 { "" } else { "s" }
        )
    } else {
        format!(
            "The Builder page is now at the training-handoff stage: choose the curated dataset, pick a backend target, name the concept clearly, choose the base model, then lock in a sensible starting setup. For the Wan/Musubi lane, saved plans generate manual WSL scripts rather than launching training from the browser. You are currently pointing at base model \"{}\" from the {} family through backend \"{}\" with {}px resolution, batch size {}, learning rate {}, and validation split {}%. You already have {} saved training plan{} here.",
            if context.base_model.is_empty() {
                "none selected yet"
            } else {
                &context.base_model
            },
            base_model_family,
            context.training_backend_id,
            context.resolution.unwrap_or(768),
            context.batch_size.unwrap_or(1),
            context.learning_rate.unwrap_or(0.0001),
            context.validation_split_percent.unwrap_or(10),
            context.prepared_project_count,
            if context.prepared_project_count == 1 {
                ""
            } else {
                "s"
            }
        )
    };

    let suggestions = vec![
        "Start with a balanced preset unless you already know this is a style-only or identity-heavy concept.".to_string(),
        "Treat rank, repeats, and epochs as a trio: pushing all three up together is usually where overfitting starts.".to_string(),
        "Get the dataset coherent first; hyperparameter tweaking cannot rescue a confused dataset.".to_string(),
    ];

    HelperQueryResponse {
        page: "builder".to_string(),
        answer,
        context_title,
        suggestions,
        lane_id: lane_id.to_string(),
        lane_label: lane_label.to_string(),
        lane_mode: lane_mode.to_string(),
        lane_status_note: lane_status_note.to_string(),
    }
}

fn normalized_page(page: &str) -> &str {
    match page.trim().to_ascii_lowercase().as_str() {
        "builder" => "builder",
        _ => "materials",
    }
}

fn join_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}
