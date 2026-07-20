"use strict";

(function attachHelperLaneProviders() {
  const FALLBACK_PROVIDERS = {
    local_builtin: {
      label: "Local built-in",
      exampleModel: "Rule-based guidance",
      modelPlaceholder: "Rule-based guidance",
      baseUrlPlaceholder: "",
      baseUrlMode: "hidden",
      apiKeyMode: "hidden",
      setupNote: "This lane stays local on this machine and does not call a remote provider.",
      draftVerificationNote: "This local helper lane is ready to answer without a remote API key.",
    },
    openai: {
      label: "OpenAI",
      exampleModel: "gpt-4.1-mini",
      modelPlaceholder: "e.g. gpt-4.1-mini",
      baseUrlPlaceholder: "https://api.openai.com/v1",
      suggestedBaseUrl: "https://api.openai.com/v1",
      baseUrlMode: "optional",
      apiKeyMode: "required",
      setupNote: "Use an OpenAI chat model name and an API key. The default base URL is usually correct.",
      draftVerificationNote: "Add an OpenAI model name and API key to finish this cloud helper lane.",
    },
    openai_compatible: {
      label: "OpenAI-compatible",
      exampleModel: "mistral-small",
      modelPlaceholder: "e.g. mistral-small, llama-3.1-70b-instruct",
      baseUrlPlaceholder: "https://api.example.com/v1",
      baseUrlMode: "required",
      apiKeyMode: "recommended",
      setupNote: "Point this lane at any OpenAI-compatible endpoint. Base URL is required here because hosts vary.",
      draftVerificationNote: "Add a model name and base URL for this OpenAI-compatible lane, then save an API key if the host requires one.",
    },
    anthropic: {
      label: "Anthropic",
      exampleModel: "claude-sonnet-4-20250514",
      modelPlaceholder: "e.g. claude-sonnet-4-20250514",
      baseUrlPlaceholder: "https://api.anthropic.com",
      suggestedBaseUrl: "https://api.anthropic.com",
      baseUrlMode: "optional",
      apiKeyMode: "required",
      setupNote: "Use an Anthropic model name and API key. The default Anthropic API host is usually correct.",
      draftVerificationNote: "Add an Anthropic model name and API key to finish this cloud helper lane.",
    },
    gemini: {
      label: "Gemini",
      exampleModel: "gemini-2.5-flash",
      modelPlaceholder: "e.g. gemini-2.5-flash",
      baseUrlPlaceholder: "https://generativelanguage.googleapis.com",
      suggestedBaseUrl: "https://generativelanguage.googleapis.com",
      baseUrlMode: "optional",
      apiKeyMode: "required",
      setupNote: "Use a Gemini model name and API key. The default Google Generative Language host is usually correct.",
      draftVerificationNote: "Add a Gemini model name and API key to finish this cloud helper lane.",
    },
    xai: {
      label: "xAI",
      exampleModel: "grok-4-0709",
      modelPlaceholder: "e.g. grok-4-0709",
      baseUrlPlaceholder: "https://api.x.ai/v1",
      suggestedBaseUrl: "https://api.x.ai/v1",
      baseUrlMode: "optional",
      apiKeyMode: "required",
      setupNote: "Use an xAI model name and API key. The default xAI API host is usually correct.",
      draftVerificationNote: "Add an xAI model name and API key to finish this cloud helper lane.",
    },
    other: {
      label: "Other",
      exampleModel: undefined,
      modelPlaceholder: "Enter the provider model name",
      baseUrlPlaceholder: "https://api.example.com/v1",
      baseUrlMode: "optional",
      apiKeyMode: "recommended",
      setupNote: "Use this when you are staging a lane for a future provider. Verification will stay limited until backend support exists.",
      draftVerificationNote: "Add a provider model name for this cloud helper lane. Base URL and API key may also be needed.",
    },
  };

  function normalizeProviderKind(value) {
    const key = String(value || "").trim().toLowerCase();
    return key || "other";
  }

  function catalogMap(catalog) {
    const entries = Array.isArray(catalog?.providers) ? catalog.providers : [];
    if (!entries.length) {
      return FALLBACK_PROVIDERS;
    }

    const mapped = {};
    for (const entry of entries) {
      const kind = normalizeProviderKind(entry?.kind);
      if (!kind) {
        continue;
      }
      mapped[kind] = {
        label: String(entry?.label || kind),
        exampleModel: entry?.example_model ? String(entry.example_model) : undefined,
        modelPlaceholder: String(entry?.model_placeholder || "Enter the provider model name"),
        baseUrlPlaceholder: String(entry?.base_url_placeholder || ""),
        suggestedBaseUrl: entry?.default_base_url ? String(entry.default_base_url) : undefined,
        baseUrlMode: String(entry?.base_url_mode || "optional"),
        apiKeyMode: String(entry?.api_key_mode || "recommended"),
        setupNote: String(entry?.setup_note || ""),
        draftVerificationNote: String(entry?.draft_verification_note || ""),
        liveHelperSupported: Boolean(entry?.live_helper_supported),
      };
    }
    if (!mapped.other) {
      mapped.other = FALLBACK_PROVIDERS.other;
    }
    if (!mapped.local_builtin) {
      mapped.local_builtin = FALLBACK_PROVIDERS.local_builtin;
    }
    return mapped;
  }

  function listProviders(catalog) {
    return Object.entries(catalogMap(catalog)).map(([kind, meta]) => ({ kind, ...meta }));
  }

  function getProviderMeta(providerKind, catalog) {
    const providers = catalogMap(catalog);
    const kind = normalizeProviderKind(providerKind);
    return providers[kind] || providers.other;
  }

  function cloudLaneDraftVerificationNote(providerKind, catalog) {
    return getProviderMeta(providerKind, catalog).draftVerificationNote;
  }

  function providerFieldState(providerKind, laneMode, catalog) {
    const meta = getProviderMeta(providerKind, catalog);
    const isCloud = String(laneMode || "local").toLowerCase() === "cloud";

    if (!isCloud) {
      return {
        modelPlaceholder: FALLBACK_PROVIDERS.local_builtin.modelPlaceholder,
        baseUrlPlaceholder: "",
        baseUrlVisible: false,
        baseUrlRequired: false,
        apiKeyVisible: false,
        apiKeyRequired: false,
        setupNote: FALLBACK_PROVIDERS.local_builtin.setupNote,
      };
    }

    return {
      modelPlaceholder: meta.modelPlaceholder,
      baseUrlPlaceholder: meta.baseUrlPlaceholder,
      baseUrlVisible: meta.baseUrlMode !== "hidden",
      baseUrlRequired: meta.baseUrlMode === "required",
      apiKeyVisible: meta.apiKeyMode !== "hidden",
      apiKeyRequired: meta.apiKeyMode === "required",
      setupNote: meta.setupNote,
    };
  }

  function validateCloudLaneDraft(lane, catalog) {
    const providerKind = normalizeProviderKind(lane?.provider_kind);
    const meta = getProviderMeta(providerKind, catalog);
    const laneMode = String(lane?.lane_mode || "local").toLowerCase();

    if (laneMode !== "cloud") {
      return { ok: true, message: "" };
    }

    if (!String(lane?.model_name || "").trim()) {
      return { ok: false, message: `${meta.label} lanes need a model name before they can be verified.` };
    }

    if (meta.baseUrlMode === "required" && !String(lane?.base_url || "").trim()) {
      return { ok: false, message: `${meta.label} lanes need a base URL before they can be verified.` };
    }

    const hasApiKey = Boolean(String(lane?.api_key || "").trim()) || lane?.has_api_key;
    if (meta.apiKeyMode === "required" && !hasApiKey) {
      return { ok: false, message: `${meta.label} lanes need an API key before they can be verified.` };
    }

    return { ok: true, message: "" };
  }

  window.ChattyLoraHelperLaneProviders = {
    listProviders,
    normalizeProviderKind,
    getProviderMeta,
    cloudLaneDraftVerificationNote,
    providerFieldState,
    validateCloudLaneDraft,
  };
}());
