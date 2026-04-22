const MAX_EXPLORATORY_BATCH_INDEX = 9;

const state = {
  page: "materials",
  dashboard: null,
  dashboardLoading: false,
  builder: {
    selectedDatasetSlug: "",
    selectedTrainingBackendId: "",
    lastSeededProjectName: "",
    loadedProjectSlug: "",
    loadedProjectName: "",
    loadedSnapshot: null,
    draftMode: "",
    deletingProjectSlug: "",
  },
  training: {
    status: null,
    pollingId: null,
    busy: false,
  },
  systemTelemetry: null,
  systemTelemetryPollingId: null,
  siteFix: {
    selectedSourceId: "",
    currentShell: null,
    loading: false,
    proposing: false,
    saving: false,
    previewingApply: false,
    applying: false,
    proposal: null,
    applyPreview: null,
  },
  helper: {
    loading: false,
    messages: [],
  },
  sources: [],
  localImport: {
    importing: false,
  },
  search: {
    query: "",
    batchIndex: 0,
    preview: null,
    selectedKeys: new Set(),
    selectedItems: new Map(),
    mediaKinds: new Set(["image", "video"]),
    loading: false,
    curating: false,
    abortController: null,
  },
};

const SYSTEM_TELEMETRY_WIDTH = 220;
const SYSTEM_TELEMETRY_HEIGHT = 54;

const elements = {
  pageButtons: Array.from(document.querySelectorAll("[data-page]")),
  materialsPage: document.getElementById("materialsPage"),
  builderPage: document.getElementById("builderPage"),
  refreshButton: document.getElementById("refreshButton"),
  webSearchInput: document.getElementById("webSearchInput"),
  searchMediaButtons: Array.from(document.querySelectorAll("[data-search-media]")),
  runSearchButton: document.getElementById("runSearchButton"),
  cancelSearchButton: document.getElementById("cancelSearchButton"),
  clearSearchButton: document.getElementById("clearSearchButton"),
  searchWindowNote: document.getElementById("searchWindowNote"),
  sourceRegistryList: document.getElementById("sourceRegistryList"),
  sourceRegistrySummary: document.getElementById("sourceRegistrySummary"),
  customSourceNameInput: document.getElementById("customSourceNameInput"),
  customSourceUrlInput: document.getElementById("customSourceUrlInput"),
  customSourceAdapterSelect: document.getElementById("customSourceAdapterSelect"),
  customSourceMediaSelect: document.getElementById("customSourceMediaSelect"),
  addCustomSourceButton: document.getElementById("addCustomSourceButton"),
  clearCustomSourceButton: document.getElementById("clearCustomSourceButton"),
  siteFixSourceSelect: document.getElementById("siteFixSourceSelect"),
  siteFixInspectButton: document.getElementById("siteFixInspectButton"),
  siteFixCloseButton: document.getElementById("siteFixCloseButton"),
  siteFixScopePanel: document.getElementById("siteFixScopePanel"),
  siteFixIssueSummary: document.getElementById("siteFixIssueSummary"),
  siteFixReproductionNotes: document.getElementById("siteFixReproductionNotes"),
  siteFixPatchNotes: document.getElementById("siteFixPatchNotes"),
  siteFixProposeButton: document.getElementById("siteFixProposeButton"),
  siteFixReviewApplyButton: document.getElementById("siteFixReviewApplyButton"),
  siteFixApplyMainButton: document.getElementById("siteFixApplyMainButton"),
  siteFixSaveButton: document.getElementById("siteFixSaveButton"),
  siteFixClearButton: document.getElementById("siteFixClearButton"),
  siteFixProposalPanel: document.getElementById("siteFixProposalPanel"),
  siteFixStatusNote: document.getElementById("siteFixStatusNote"),
  previousBatchButton: document.getElementById("previousBatchButton"),
  nextBatchButton: document.getElementById("nextBatchButton"),
  datasetNameInput: document.getElementById("datasetNameInput"),
  curateSelectionButton: document.getElementById("curateSelectionButton"),
  curationStatusNote: document.getElementById("curationStatusNote"),
  selectionTraySummary: document.getElementById("selectionTraySummary"),
  selectionTrayList: document.getElementById("selectionTrayList"),
  clearSelectionButton: document.getElementById("clearSelectionButton"),
  searchMetaPanel: document.getElementById("searchMetaPanel"),
  searchPreviewResults: document.getElementById("searchPreviewResults"),
  materialsSearchInput: document.getElementById("materialsSearchInput"),
  localImportSourceSelect: document.getElementById("localImportSourceSelect"),
  localImportDatasetNameInput: document.getElementById("localImportDatasetNameInput"),
  localImportDatasetButton: document.getElementById("localImportDatasetButton"),
  localImportStatusNote: document.getElementById("localImportStatusNote"),
  materialSummaryGrid: document.getElementById("materialSummaryGrid"),
  inputsList: document.getElementById("inputsList"),
  outputsList: document.getElementById("outputsList"),
  modelsList: document.getElementById("modelsList"),
  runtimeGrid: document.getElementById("runtimeGrid"),
  runtimePill: document.getElementById("runtimePill"),
  projectNameInput: document.getElementById("projectNameInput"),
  datasetSelect: document.getElementById("datasetSelect"),
  baseModelSelect: document.getElementById("baseModelSelect"),
  trainingBackendSelect: document.getElementById("trainingBackendSelect"),
  conceptTypeSelect: document.getElementById("conceptTypeSelect"),
  trainingPresetSelect: document.getElementById("trainingPresetSelect"),
  triggerPhraseInput: document.getElementById("triggerPhraseInput"),
  conceptSummaryInput: document.getElementById("conceptSummaryInput"),
  captionStrategySelect: document.getElementById("captionStrategySelect"),
  conceptTypeHelp: document.getElementById("conceptTypeHelp"),
  trainingPresetHelp: document.getElementById("trainingPresetHelp"),
  captionStrategyHelp: document.getElementById("captionStrategyHelp"),
  builderPlanExplanationPanel: document.getElementById("builderPlanExplanationPanel"),
  builderSettingsExplanationPanel: document.getElementById("builderSettingsExplanationPanel"),
  builderPlanStatePanel: document.getElementById("builderPlanStatePanel"),
  prepareProjectButton: document.getElementById("prepareProjectButton"),
  prepareProjectNote: document.getElementById("prepareProjectNote"),
  rankInput: document.getElementById("rankInput"),
  repeatsInput: document.getElementById("repeatsInput"),
  epochsInput: document.getElementById("epochsInput"),
  resolutionSelect: document.getElementById("resolutionSelect"),
  batchSizeInput: document.getElementById("batchSizeInput"),
  learningRateInput: document.getElementById("learningRateInput"),
  validationSplitInput: document.getElementById("validationSplitInput"),
  rankHelp: document.getElementById("rankHelp"),
  repeatsHelp: document.getElementById("repeatsHelp"),
  epochsHelp: document.getElementById("epochsHelp"),
  resolutionHelp: document.getElementById("resolutionHelp"),
  batchSizeHelp: document.getElementById("batchSizeHelp"),
  learningRateHelp: document.getElementById("learningRateHelp"),
  validationSplitHelp: document.getElementById("validationSplitHelp"),
  builderStatusList: document.getElementById("builderStatusList"),
  builderNotesList: document.getElementById("builderNotesList"),
  builderDatasetSummary: document.getElementById("builderDatasetSummary"),
  builderDatasetPreflight: document.getElementById("builderDatasetPreflight"),
  curatedDatasetsList: document.getElementById("curatedDatasetsList"),
  preparedProjectsList: document.getElementById("preparedProjectsList"),
  trainingBackendList: document.getElementById("trainingBackendList"),
  wanTrainingStatus: document.getElementById("wanTrainingStatus"),
  helperContextTitle: document.getElementById("helperContextTitle"),
  helperMessages: document.getElementById("helperMessages"),
  helperQuickActions: document.getElementById("helperQuickActions"),
  helperInput: document.getElementById("helperInput"),
  helperSendButton: document.getElementById("helperSendButton"),
};

for (const button of elements.pageButtons) {
  button.addEventListener("click", () => {
    state.page = button.dataset.page;
    renderPage();
    renderHelper();
  });
}

elements.refreshButton.addEventListener("click", () => {
  void loadDashboard();
});

elements.materialsSearchInput.addEventListener("input", () => {
  renderMaterialLists();
});

elements.localImportSourceSelect.addEventListener("change", () => {
  seedLocalImportDatasetName({ force: true });
  renderLocalImportControls();
});

elements.localImportDatasetNameInput.addEventListener("input", () => {
  renderLocalImportControls();
});

elements.localImportDatasetButton.addEventListener("click", () => {
  void importLocalDataset();
});

elements.runSearchButton.addEventListener("click", () => {
  state.search.batchIndex = 0;
  void runPreviewSearch();
});

elements.cancelSearchButton.addEventListener("click", () => {
  if (state.search.abortController) {
    state.search.abortController.abort();
  }
});

elements.clearSearchButton.addEventListener("click", () => {
  clearSearchPreview();
});

elements.webSearchInput.addEventListener("keydown", (event) => {
  if (event.key === "Enter") {
    event.preventDefault();
    state.search.batchIndex = 0;
    void runPreviewSearch();
  }
});

elements.webSearchInput.addEventListener("input", updateSearchControls);

for (const button of elements.searchMediaButtons) {
  button.addEventListener("click", () => {
    const mediaKind = button.dataset.searchMedia;
    if (!mediaKind) {
      return;
    }
    if (state.search.mediaKinds.has(mediaKind)) {
      if (state.search.mediaKinds.size === 1) {
        elements.searchWindowNote.textContent = "Keep at least one media type selected.";
        return;
      }
      state.search.mediaKinds.delete(mediaKind);
    } else {
      state.search.mediaKinds.add(mediaKind);
    }
    state.search.batchIndex = 0;
    state.search.preview = null;
    state.search.selectedKeys.clear();
    state.search.selectedItems.clear();
    renderSearchMediaButtons();
    renderSearchMeta();
    renderPreviewResults();
    renderSelectionTray();
    elements.searchWindowNote.textContent = `Media filter set to ${formatMediaKindList(selectedSearchMediaKinds())}.`;
  });
}

elements.previousBatchButton.addEventListener("click", () => {
  if (state.search.batchIndex === 0 || state.search.loading) {
    return;
  }
  state.search.batchIndex -= 1;
  void runPreviewSearch();
});

elements.nextBatchButton.addEventListener("click", () => {
  if (state.search.loading || !canProbeNextSearchBatch(state.search.preview)) {
    return;
  }
  state.search.batchIndex += 1;
  void runPreviewSearch();
});

elements.addCustomSourceButton.addEventListener("click", () => {
  void addCustomSource();
});

elements.clearCustomSourceButton.addEventListener("click", () => {
  clearCustomSourceForm();
});

elements.siteFixSourceSelect.addEventListener("change", () => {
  state.siteFix.selectedSourceId = elements.siteFixSourceSelect.value;
  state.siteFix.currentShell = null;
  state.siteFix.proposal = null;
  state.siteFix.applyPreview = null;
  elements.siteFixIssueSummary.value = "";
  elements.siteFixReproductionNotes.value = "";
  elements.siteFixPatchNotes.value = "";
  elements.siteFixStatusNote.textContent = "Select a source shell to inspect its adapter scope and local fix brief.";
  renderSiteFixShell();
});

for (const input of [
  elements.siteFixIssueSummary,
  elements.siteFixReproductionNotes,
  elements.siteFixPatchNotes,
]) {
  input.addEventListener("input", () => {
    if (state.siteFix.proposal || state.siteFix.applyPreview) {
      state.siteFix.proposal = null;
      state.siteFix.applyPreview = null;
      elements.siteFixStatusNote.textContent =
        "The scoped proposal review was cleared because the brief changed. Draft a fresh one when you are ready.";
    }
    renderSiteFixShell();
  });
}

elements.siteFixInspectButton.addEventListener("click", () => {
  void openSiteFixShell();
});

elements.siteFixCloseButton.addEventListener("click", () => {
  closeSiteFixShell();
});

elements.siteFixProposeButton.addEventListener("click", () => {
  void proposeSiteFixShell();
});

elements.siteFixReviewApplyButton.addEventListener("click", () => {
  void previewSiteFixApply();
});

elements.siteFixApplyMainButton.addEventListener("click", () => {
  void applySiteFix();
});

elements.siteFixSaveButton.addEventListener("click", () => {
  void saveSiteFixShell();
});

elements.siteFixClearButton.addEventListener("click", () => {
  clearSiteFixBrief();
});

elements.curateSelectionButton.addEventListener("click", () => {
  void createDatasetFromSelection();
});

elements.clearSelectionButton.addEventListener("click", () => {
  state.search.selectedKeys.clear();
  state.search.selectedItems.clear();
  renderSearchMeta();
  renderPreviewResults();
  renderSelectionTray();
});

elements.datasetSelect.addEventListener("change", () => {
  state.builder.selectedDatasetSlug = elements.datasetSelect.value;
  syncProjectNameSuggestion();
  renderBuilder();
});

elements.trainingBackendSelect.addEventListener("change", () => {
  state.builder.selectedTrainingBackendId = elements.trainingBackendSelect.value;
  renderBuilder();
});

for (const input of [
  elements.projectNameInput,
  elements.baseModelSelect,
  elements.conceptTypeSelect,
  elements.trainingPresetSelect,
  elements.triggerPhraseInput,
  elements.conceptSummaryInput,
  elements.captionStrategySelect,
  elements.rankInput,
  elements.repeatsInput,
  elements.epochsInput,
  elements.resolutionSelect,
  elements.batchSizeInput,
  elements.learningRateInput,
  elements.validationSplitInput,
]) {
  input.addEventListener("input", renderBuilderGuidance);
  input.addEventListener("change", renderBuilderGuidance);
}

elements.prepareProjectButton.addEventListener("click", () => {
  void prepareBuilderProject();
});

elements.helperSendButton.addEventListener("click", () => {
  void askHelper();
});

elements.helperInput.addEventListener("keydown", (event) => {
  if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
    event.preventDefault();
    void askHelper();
  }
});

void loadDashboard();
void loadTrainingStatus();
startSystemTelemetryPolling();

async function loadDashboard() {
  setDashboardLoading(true);

  try {
    const response = await fetch("/api/dashboard");
    if (!response.ok) {
      throw new Error(`Dashboard request failed with ${response.status}`);
    }

    state.dashboard = await response.json();
    state.sources = state.dashboard.materials.source_registry.sources.map((source) => ({ ...source }));
    seedBuilderForm();
    renderAll();
  } catch (error) {
    console.error(error);
    const message = `Could not load dashboard data yet. ${String(error.message || error)}`;
    elements.materialSummaryGrid.innerHTML = `<div class="empty-state">${escapeHtml(message)}</div>`;
    elements.searchPreviewResults.innerHTML = `<div class="empty-state">${escapeHtml(message)}</div>`;
  } finally {
    setDashboardLoading(false);
  }
}

function startSystemTelemetryPolling() {
  if (state.systemTelemetryPollingId) {
    return;
  }

  void loadSystemTelemetry();
  state.systemTelemetryPollingId = window.setInterval(() => {
    void loadSystemTelemetry();
  }, 2200);
}

async function loadSystemTelemetry() {
  try {
    const response = await fetch("/api/telemetry/system");
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || `Telemetry request failed with ${response.status}`);
    }
    state.systemTelemetry = payload;
  } catch (error) {
    console.error(error);
    state.systemTelemetry = {
      supported: false,
      label: "ECG Window",
      note: "ECG Window sampling is temporarily unavailable.",
      cpu_label: "CPU",
      gpu_label: "GPU",
      current_cpu_percent: 0,
      current_gpu_percent: 0,
      cpu_history: [],
      gpu_history: [],
    };
  }
  renderSystemTelemetry();
}

async function loadTrainingStatus({ render = true } = {}) {
  try {
    const response = await fetch("/api/training/status");
    if (!response.ok) {
      throw new Error(`Training status request failed with ${response.status}`);
    }
    state.training.status = await response.json();
    ensureTrainingPolling();
    if (render && state.dashboard) {
      renderBuilder();
    }
  } catch (error) {
    console.error(error);
  }
}

function ensureTrainingPolling() {
  const running = isTrainingStatusActive(state.training.status);
  if (running && !state.training.pollingId) {
    state.training.pollingId = window.setInterval(() => {
      void loadTrainingStatus();
    }, 1600);
  } else if (!running && state.training.pollingId) {
    window.clearInterval(state.training.pollingId);
    state.training.pollingId = null;
  }
}

function isTrainingStatusActive(status) {
  return status && (status.state === "running" || status.state === "stopping");
}

function renderAll() {
  renderPage();
  renderMaterialSummary();
  renderSourceRegistry();
  renderSiteFixShell();
  renderMaterialLists();
  renderLocalImportControls();
  renderSearchMediaButtons();
  renderSearchMeta();
  renderSelectionTray();
  renderPreviewResults();
  renderModelList();
  renderRuntime();
  renderBuilder();
  renderHelper();
}

function renderPage() {
  for (const button of elements.pageButtons) {
    button.classList.toggle("active", button.dataset.page === state.page);
  }
  elements.materialsPage.classList.toggle("active", state.page === "materials");
  elements.builderPage.classList.toggle("active", state.page === "builder");
}

function renderHelper() {
  const contextTitle = helperContextTitle();
  elements.helperContextTitle.textContent = contextTitle;
  renderHelperMessages();
  renderHelperQuickActions();
  elements.helperSendButton.disabled = state.helper.loading;
  elements.helperSendButton.textContent = state.helper.loading ? "Thinking..." : "Ask helper";
}

function renderMaterialSummary() {
  if (!state.dashboard) {
    return;
  }

  const { input_summary: inputs, output_summary: outputs, model_summary: models, source_registry: sources } =
    state.dashboard.materials;

  elements.materialSummaryGrid.innerHTML = [
    summaryCard("Sources", `${sources.enabled}/${sources.total}`, `${sources.search_ready} ready adapters, ${sources.custom} custom entries`),
    summaryCard("Inputs", `${inputs.total} files`, `${inputs.images} images, ${inputs.audio} audio, ${inputs.video} video`),
    summaryCard("Outputs", `${outputs.total} files`, `${outputs.images} images, ${outputs.audio} audio, ${outputs.video} video`),
    summaryCard("Models", `${models.total} files`, `${models.gguf} GGUF, ${models.safetensors} safetensors, ${models.checkpoints} checkpoints`),
  ].join("");
}

function renderSourceRegistry() {
  const enabled = state.sources.filter((source) => source.enabled).length;
  elements.sourceRegistrySummary.textContent = `${enabled} of ${state.sources.length} source${state.sources.length === 1 ? "" : "s"} enabled for search.`;

  if (!state.sources.length) {
    elements.sourceRegistryList.innerHTML = `<div class="empty-state">No sources configured yet. Add a custom source to start building your own registry.</div>`;
    return;
  }

  elements.sourceRegistryList.innerHTML = state.sources
    .map((source) => {
      const ready = adapterReady(source.adapter_kind);
      return `
        <article class="source-card ${source.enabled ? "active" : ""}">
          <label class="source-toggle">
            <input
              type="checkbox"
              data-source-toggle="${escapeAttribute(source.id)}"
              ${source.enabled ? "checked" : ""}
            />
            <div>
              <h4>${escapeHtml(source.name)}</h4>
              <p>${escapeHtml(source.base_url)}</p>
            </div>
          </label>
          <div class="source-meta">
            <span class="list-badge">${escapeHtml(source.media_kind)}</span>
            <span class="list-badge ${ready ? "ok-badge" : "muted-badge"}">${ready ? "ready" : "adapter scaffold"}</span>
            ${source.user_added ? `<span class="list-badge warm-badge">custom</span>` : ""}
          </div>
          <p class="source-note">${escapeHtml(source.notes)}</p>
          <div class="inline-actions compact source-card-actions">
            <a class="secondary-button source-link-button" href="${escapeAttribute(source.base_url)}" target="_blank" rel="noreferrer">Open source</a>
            <button class="secondary-button source-fix-open-button" type="button" data-open-site-fix="${escapeAttribute(source.id)}">Open site fix shell</button>
            <button class="secondary-button source-remove-button" type="button" data-remove-source="${escapeAttribute(source.id)}">Remove from list</button>
          </div>
        </article>
      `;
    })
    .join("");

  for (const input of elements.sourceRegistryList.querySelectorAll("[data-source-toggle]")) {
    input.addEventListener("change", (event) => {
      const id = event.currentTarget.dataset.sourceToggle;
      const source = state.sources.find((entry) => entry.id === id);
      if (!source) {
        return;
      }
      source.enabled = event.currentTarget.checked;
      renderSourceRegistry();
      renderSiteFixShell();
      void persistSources();
    });
  }

  for (const button of elements.sourceRegistryList.querySelectorAll("[data-open-site-fix]")) {
    button.addEventListener("click", () => {
      const sourceId = button.dataset.openSiteFix;
      if (!sourceId) {
        return;
      }
      state.siteFix.selectedSourceId = sourceId;
      elements.siteFixSourceSelect.value = sourceId;
      void openSiteFixShell();
    });
  }

  for (const button of elements.sourceRegistryList.querySelectorAll("[data-remove-source]")) {
    button.addEventListener("click", () => {
      const sourceId = button.dataset.removeSource;
      const source = state.sources.find((entry) => entry.id === sourceId);
      if (!source) {
        return;
      }
      const sourceKind = source.user_added ? "custom source" : "bundled source";
      const confirmed = window.confirm(
        `Remove ${sourceKind} "${source.name}" from your local source list? This only changes your local config/sources.json.`,
      );
      if (!confirmed) {
        return;
      }
      state.sources = state.sources.filter((entry) => entry.id !== sourceId);
      if (state.siteFix.selectedSourceId === sourceId) {
        closeSiteFixShell();
        state.siteFix.selectedSourceId = "";
      }
      elements.searchWindowNote.textContent = `Removed "${source.name}" from the local source list.`;
      renderSourceRegistry();
      renderSiteFixShell();
      void persistSources();
    });
  }
}

function siteFixHasPatchDraft() {
  return Boolean(String(elements.siteFixPatchNotes.value || "").trim());
}

function renderSiteFixShell() {
  const items = state.dashboard?.materials?.site_fix_summaries || [];
  const currentId = state.siteFix.selectedSourceId || items[0]?.source_id || "";
  state.siteFix.selectedSourceId = currentId;

  elements.siteFixSourceSelect.innerHTML = items.length
    ? items
        .map(
          (item) => `<option value="${escapeAttribute(item.source_id)}">${escapeHtml(item.source_name)} (${escapeHtml(item.adapter_kind)})</option>`,
        )
        .join("")
    : `<option value="">No source entries yet</option>`;
  elements.siteFixSourceSelect.disabled = items.length === 0;
  if (currentId) {
    elements.siteFixSourceSelect.value = currentId;
  }

  elements.siteFixInspectButton.disabled = items.length === 0 || state.siteFix.loading;
  elements.siteFixInspectButton.textContent = state.siteFix.loading ? "Opening..." : "Open source shell";
  elements.siteFixCloseButton.disabled = !state.siteFix.currentShell || state.siteFix.loading;
  elements.siteFixProposeButton.disabled =
    items.length === 0 || state.siteFix.proposing || !state.siteFix.currentShell;
  elements.siteFixProposeButton.textContent = state.siteFix.proposing
    ? "Drafting..."
    : "Draft scoped fix proposal";
  const hasPatchDraft = siteFixHasPatchDraft();
  elements.siteFixReviewApplyButton.disabled =
    items.length === 0 ||
    !state.siteFix.currentShell ||
    state.siteFix.previewingApply ||
    state.siteFix.applying ||
    (!state.siteFix.proposal && !hasPatchDraft);
  elements.siteFixReviewApplyButton.textContent = state.siteFix.previewingApply
    ? "Building review..."
    : "Review proposed fix";
  elements.siteFixApplyMainButton.disabled =
    items.length === 0 ||
    !state.siteFix.currentShell ||
    !state.siteFix.applyPreview ||
    state.siteFix.previewingApply ||
    state.siteFix.applying;
  elements.siteFixApplyMainButton.textContent = state.siteFix.applying
    ? "Applying..."
    : "Apply proposed fix";
  elements.siteFixSaveButton.disabled = items.length === 0 || state.siteFix.saving || !state.siteFix.currentShell;
  elements.siteFixSaveButton.textContent = state.siteFix.saving ? "Saving..." : "Save site-fix brief";
  elements.siteFixClearButton.disabled =
    !state.siteFix.currentShell ||
    state.siteFix.loading ||
    state.siteFix.proposing ||
    state.siteFix.saving ||
    state.siteFix.previewingApply ||
    state.siteFix.applying;

  if (!state.siteFix.currentShell) {
    elements.siteFixScopePanel.innerHTML = `<p>Select a source to inspect its adapter scope and local fix brief.</p>`;
    elements.siteFixProposalPanel.innerHTML = `<p>Draft a proposal to review a source-specific adapter fix before anything is edited.</p>`;
    return;
  }

  const shell = state.siteFix.currentShell;
  elements.siteFixScopePanel.innerHTML = `
    <h3>${escapeHtml(shell.source_name)}</h3>
    <p><strong>Adapter file:</strong> ${escapeHtml(shell.adapter_file_path)}</p>
    <p><strong>Local note path:</strong> ${escapeHtml(shell.note_relative_path)}</p>
    <div class="inline-actions compact">
      <button class="secondary-button" type="button" id="siteFixCopyAdapterPathButton">Copy adapter path</button>
      <button class="secondary-button" type="button" id="siteFixCopyNotePathButton">Copy note path</button>
    </div>
    <p><strong>Readiness:</strong> ${shell.adapter_ready ? "Implemented adapter present" : "Adapter scaffold / not searchable yet"}</p>
    <p>${escapeHtml(shell.scope_note)}</p>
    <ul class="bullet-list">
      ${shell.starter_steps.map((step) => `<li>${escapeHtml(step)}</li>`).join("")}
    </ul>
  `;

  const copyAdapterPathButton = document.getElementById("siteFixCopyAdapterPathButton");
  if (copyAdapterPathButton) {
    copyAdapterPathButton.addEventListener("click", () => {
      void copyTextToClipboard(shell.adapter_file_path, "Adapter path copied for this source shell.");
    });
  }

  const copyNotePathButton = document.getElementById("siteFixCopyNotePathButton");
  if (copyNotePathButton) {
    copyNotePathButton.addEventListener("click", () => {
      void copyTextToClipboard(shell.note_relative_path, "Local note path copied for this source shell.");
    });
  }

  if (!state.siteFix.proposal) {
    elements.siteFixProposalPanel.innerHTML = `
      <h3>Review-first scoped proposal</h3>
      <p>Use this to draft a local adapter-only fix outline for <strong>${escapeHtml(shell.adapter_file_path)}</strong>.</p>
      <p class="muted-copy">The proposal stays scoped to the selected source adapter and does not edit crawler core. A proposal is only a plan; nothing is changed until you generate the apply review and press the final apply button.</p>
      <p class="muted-copy">Main-row buttons above will light up as each step becomes available: draft or paste a fix, review it, then apply it.</p>
      ${renderApplyHistory(shell.apply_history)}
      ${renderProposalHistory(shell.proposal_history)}
    `;
    return;
  }

  const proposal = state.siteFix.proposal;
  elements.siteFixProposalPanel.innerHTML = `
    <div class="section-heading-row">
      <div>
        <h3>${escapeHtml(proposal.proposal_title)}</h3>
        <p><strong>Adapter file:</strong> ${escapeHtml(proposal.adapter_file_path)}</p>
      </div>
      <span class="list-badge ${proposal.confidence_label.toLowerCase().includes("high") ? "ok-badge" : "warm-badge"}">${escapeHtml(proposal.confidence_label)}</span>
    </div>
    <div class="site-fix-proposal-copy">
      <h4>Analysis</h4>
      <ul class="bullet-list">
        ${proposal.analysis_points.map((point) => `<li>${escapeHtml(point)}</li>`).join("")}
      </ul>
      <h4>Scoped patch draft</h4>
      <pre class="proposal-code"><code>${escapeHtml(proposal.proposed_patch)}</code></pre>
      <div class="inline-actions compact">
        <button class="secondary-button" type="button" id="siteFixUseProposalButton">Use proposal as patch notes draft</button>
        <button class="secondary-button" type="button" id="siteFixSaveProposalButton">Save proposal snapshot</button>
        <button class="primary-button" type="button" id="siteFixPreviewApplyButton">${state.siteFix.previewingApply ? "Building review..." : "Review proposed fix before applying"}</button>
      </div>
      <p class="muted-copy">Not applied yet. The review step shows exactly what Chatty-lora plans to change, then reveals the final apply button.</p>
      <h4>Review checklist</h4>
      <ul class="bullet-list">
        ${proposal.review_checklist.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}
      </ul>
      ${renderApplyPreview(state.siteFix.applyPreview)}
      ${renderApplyHistory(shell.apply_history)}
      ${renderProposalHistory(shell.proposal_history)}
    </div>
  `;

  const useProposalButton = document.getElementById("siteFixUseProposalButton");
  if (useProposalButton) {
    useProposalButton.addEventListener("click", () => {
      elements.siteFixPatchNotes.value = proposal.proposed_patch;
      elements.siteFixStatusNote.textContent =
        "Proposal copied into Patch notes. Review and trim it before saving the local source brief.";
    });
  }

  const saveProposalButton = document.getElementById("siteFixSaveProposalButton");
  if (saveProposalButton) {
    saveProposalButton.addEventListener("click", () => {
      void saveSiteFixProposalSnapshot();
    });
  }

  const previewApplyButton = document.getElementById("siteFixPreviewApplyButton");
  if (previewApplyButton) {
    previewApplyButton.disabled = state.siteFix.previewingApply || state.siteFix.applying;
    previewApplyButton.addEventListener("click", () => {
      void previewSiteFixApply();
    });
  }

  const applyAdapterButton = document.getElementById("siteFixApplyButton");
  if (applyAdapterButton) {
    applyAdapterButton.disabled = state.siteFix.applying;
    applyAdapterButton.addEventListener("click", () => {
      void applySiteFix();
    });
  }

  for (const button of elements.siteFixProposalPanel.querySelectorAll("[data-copy-proposal-path]")) {
    button.addEventListener("click", () => {
      const relativePath = button.dataset.copyProposalPath;
      if (!relativePath) {
        return;
      }
      void copyTextToClipboard(relativePath, "Saved proposal path copied.");
    });
  }

  for (const button of elements.siteFixProposalPanel.querySelectorAll("[data-copy-apply-path]")) {
    button.addEventListener("click", () => {
      const relativePath = button.dataset.copyApplyPath;
      if (!relativePath) {
        return;
      }
      void copyTextToClipboard(relativePath, "Saved apply-record path copied.");
    });
  }
}

function renderMaterialLists() {
  if (!state.dashboard) {
    return;
  }

  const query = String(elements.materialsSearchInput.value || "").trim().toLowerCase();
  const filter = (item) =>
    !query
    || item.name.toLowerCase().includes(query)
    || item.relative_path.toLowerCase().includes(query)
    || item.kind.toLowerCase().includes(query);

  renderLibraryList(elements.inputsList, state.dashboard.materials.input_files.filter(filter), "No input files yet. Drop training material into inputs/.");
  renderLibraryList(elements.outputsList, state.dashboard.materials.output_files.filter(filter), "No output files yet. Later exports and curated datasets can appear here.");
}

function renderLocalImportControls() {
  if (!state.dashboard) {
    return;
  }

  const folders = state.dashboard.builder.curated_datasets || [];
  const currentValue = elements.localImportSourceSelect.value;
  const selected =
    folders.find((folder) => folder.slug === currentValue) ||
    folders.find((folder) => folder.total_files > 0) ||
    folders[0] ||
    null;

  elements.localImportSourceSelect.innerHTML = folders.length
    ? folders
        .map((folder) => {
          const detail = `${folder.total_files} file${folder.total_files === 1 ? "" : "s"} | ${folder.images} img | ${folder.video} vid | ${folder.audio} aud`;
          return `<option value="${escapeAttribute(folder.slug)}">${escapeHtml(folder.display_name)} (${escapeHtml(detail)})</option>`;
        })
        .join("")
    : `<option value="">No input folders found yet</option>`;
  elements.localImportSourceSelect.disabled = folders.length === 0 || state.localImport.importing;
  if (selected) {
    elements.localImportSourceSelect.value = selected.slug;
    seedLocalImportDatasetName();
  }

  elements.localImportDatasetButton.disabled =
    state.localImport.importing || !selected || !elements.localImportDatasetNameInput.value.trim();
  elements.localImportDatasetButton.textContent = state.localImport.importing
    ? "Cleaning..."
    : "Clean folder into dataset";

  if (!folders.length) {
    elements.localImportStatusNote.textContent =
      "Drop a folder into inputs/, refresh scan, then clean it here.";
  } else if (!elements.localImportDatasetNameInput.value.trim()) {
    elements.localImportStatusNote.textContent =
      "Pick a source folder and name the cleaned dataset before running cleanup.";
  }
}

function seedLocalImportDatasetName({ force = false } = {}) {
  if (!state.dashboard) {
    return;
  }
  if (!force && elements.localImportDatasetNameInput.value.trim()) {
    return;
  }

  const selectedSlug = elements.localImportSourceSelect.value;
  const selected = state.dashboard.builder.curated_datasets.find((folder) => folder.slug === selectedSlug);
  if (!selected) {
    return;
  }
  elements.localImportDatasetNameInput.value = `${slugify(selected.display_name || selected.slug)}-clean`;
}

function renderLibraryList(target, items, emptyMessage) {
  if (!items.length) {
    target.innerHTML = `<div class="empty-state">${escapeHtml(emptyMessage)}</div>`;
    return;
  }

  target.innerHTML = items
    .map(
      (item) => `
        <article class="list-item">
          <div>
            <h4>${escapeHtml(item.name)}</h4>
            <p>${escapeHtml(item.relative_path)}</p>
          </div>
          <span class="list-badge">${escapeHtml(item.kind)}</span>
        </article>
      `,
    )
    .join("");
}

function renderSearchMeta() {
  const preview = state.search.preview;
  if (!preview) {
    elements.searchMetaPanel.innerHTML = `
      <p>Run a search, or leave the term blank to browse pages 1 to 3 from the selected sources.</p>
      <p><strong>Media filter:</strong> ${escapeHtml(formatMediaKindList(selectedSearchMediaKinds()))}</p>
      <p class="muted-copy">Browse mode only sees media exposed by the fetched source pages. For better site coverage, use a listing URL with {page}, a real search URL with {query}, or a source profile.</p>
    `;
    elements.previousBatchButton.disabled = true;
    elements.nextBatchButton.disabled = true;
    elements.curateSelectionButton.disabled = state.search.curating || state.search.selectedItems.size === 0;
    updateSearchControls();
    return;
  }

  const selectedCount = state.search.selectedKeys.size;
  const nextHint = nextSearchBatchHint(preview);
  elements.searchMetaPanel.innerHTML = `
    <p><strong>Mode:</strong> ${preview.query.trim() ? `Search for "${escapeHtml(preview.query)}"` : "Browse selected source pages"}</p>
    <p><strong>Media filter:</strong> ${escapeHtml(formatMediaKindList(selectedSearchMediaKinds()))}</p>
    <p><strong>Loaded pages:</strong> ${preview.page_window_start} to ${preview.page_window_end}</p>
    <p><strong>Selected preview items:</strong> ${selectedCount}</p>
    ${nextHint ? `<p class="muted-copy">${escapeHtml(nextHint)}</p>` : ""}
    ${preview.notes.map((note) => `<p class="muted-copy">${escapeHtml(note)}</p>`).join("")}
  `;

  elements.previousBatchButton.disabled = state.search.loading || state.search.batchIndex === 0;
  elements.nextBatchButton.disabled = state.search.loading || !canProbeNextSearchBatch(preview);
  elements.curateSelectionButton.disabled = state.search.loading || state.search.curating || selectedCount === 0;
  updateSearchControls();
}

function canProbeNextSearchBatch(preview) {
  if (!preview) {
    return false;
  }
  if (preview.source_batches.some((batch) => batch.has_more)) {
    return true;
  }
  if (state.search.batchIndex >= MAX_EXPLORATORY_BATCH_INDEX) {
    return false;
  }
  return preview.source_batches.some((batch) => {
    const source = state.sources.find((candidate) => candidate.id === batch.source_id);
    return source?.adapter_kind === "generic_gallery_html";
  });
}

function nextSearchBatchHint(preview) {
  if (!preview || preview.source_batches.some((batch) => batch.has_more)) {
    return "";
  }
  if (state.search.batchIndex >= MAX_EXPLORATORY_BATCH_INDEX) {
    return "Exploratory paging stopped at the safety limit. Add a better listing URL, {page} template, or source profile to keep crawling respectfully.";
  }
  const canProbeGenericSource = preview.source_batches.some((batch) => {
    const source = state.sources.find((candidate) => candidate.id === batch.source_id);
    return source?.adapter_kind === "generic_gallery_html";
  });
  if (!canProbeGenericSource) {
    return "";
  }
  return "Generic site probe: this source did not advertise another page, but Next stays available so you can manually try the next page window.";
}

function updateSearchControls() {
  const isSearching = state.search.loading;
  const hasSearchState =
    Boolean(elements.webSearchInput.value.trim()) ||
    Boolean(state.search.preview) ||
    state.search.batchIndex !== 0 ||
    state.search.selectedItems.size > 0;

  elements.runSearchButton.disabled = state.dashboardLoading || isSearching;
  elements.runSearchButton.textContent = isSearching
    ? "Working..."
    : elements.webSearchInput.value.trim()
      ? "Search selected sources"
      : "Browse selected sources";
  elements.cancelSearchButton.disabled = !isSearching;
  elements.clearSearchButton.disabled = isSearching || !hasSearchState;
}

function renderSearchMediaButtons() {
  for (const button of elements.searchMediaButtons) {
    const mediaKind = button.dataset.searchMedia;
    const active = mediaKind && state.search.mediaKinds.has(mediaKind);
    button.classList.toggle("active", Boolean(active));
    button.setAttribute("aria-pressed", active ? "true" : "false");
  }
}

function selectedSearchMediaKinds() {
  return ["image", "video", "audio"].filter((kind) => state.search.mediaKinds.has(kind));
}

function formatMediaKindList(mediaKinds) {
  const labels = {
    image: "images",
    video: "video",
    audio: "audio",
  };
  return mediaKinds.map((kind) => labels[kind] || kind).join(", ");
}

function clearSearchPreview() {
  if (state.search.loading && state.search.abortController) {
    state.search.abortController.abort();
  }

  elements.webSearchInput.value = "";
  state.search.query = "";
  state.search.batchIndex = 0;
  state.search.preview = null;
  state.search.selectedKeys.clear();
  state.search.selectedItems.clear();
  elements.searchWindowNote.textContent = "Search cleared. Enter a term, or leave it blank to browse selected sources.";
  elements.curationStatusNote.textContent =
    "Select preview items, then let Chatty-lora do the download and naming grunt work.";
  renderSearchMeta();
  renderPreviewResults();
  renderSelectionTray();
}

function clearCustomSourceForm() {
  elements.customSourceNameInput.value = "";
  elements.customSourceUrlInput.value = "";
  elements.customSourceAdapterSelect.value = "generic_gallery_html";
  elements.customSourceMediaSelect.value = "image";
  elements.searchWindowNote.textContent = "Custom source form cleared.";
}

function closeSiteFixShell() {
  state.siteFix.currentShell = null;
  state.siteFix.proposal = null;
  state.siteFix.applyPreview = null;
  elements.siteFixIssueSummary.value = "";
  elements.siteFixReproductionNotes.value = "";
  elements.siteFixPatchNotes.value = "";
  elements.siteFixStatusNote.textContent = "Site-fix shell closed. Open a source shell when you need it.";
  renderSiteFixShell();
}

function clearSiteFixBrief() {
  elements.siteFixIssueSummary.value = "";
  elements.siteFixReproductionNotes.value = "";
  elements.siteFixPatchNotes.value = "";
  state.siteFix.proposal = null;
  state.siteFix.applyPreview = null;
  elements.siteFixStatusNote.textContent = "Site-fix brief cleared for the currently selected source.";
  renderSiteFixShell();
}

function renderSelectionTray() {
  const items = Array.from(state.search.selectedItems.values()).sort((left, right) =>
    left.title.toLowerCase().localeCompare(right.title.toLowerCase()),
  );

  const imageCount = items.filter((item) => item.kind === "Image").length;
  const audioCount = items.filter((item) => item.kind === "Audio").length;
  const videoCount = items.filter((item) => item.kind === "Video").length;

  elements.selectionTraySummary.textContent = items.length
    ? `${items.length} selected | ${imageCount} image${imageCount === 1 ? "" : "s"} | ${audioCount} audio | ${videoCount} video`
    : "Selected preview items will gather here before curation.";

  elements.clearSelectionButton.disabled = items.length === 0;

  if (!items.length) {
    elements.selectionTrayList.innerHTML = `
      <div class="empty-state">
        Tick preview items and they will appear here as a lightweight review step before download.
      </div>
    `;
    return;
  }

  elements.selectionTrayList.innerHTML = items
    .map(
      (item) => `
        <article class="selection-item">
          <div class="selection-item-copy">
            <div class="dataset-card-title-row">
              <h4>${escapeHtml(item.title)}</h4>
              <span class="list-badge">${escapeHtml(item.kind)}</span>
            </div>
            <p>${escapeHtml(item.source_label)} | page ${item.page_number}${item.creator ? ` | ${escapeHtml(item.creator)}` : ""}</p>
            <div class="preview-links">
              <a href="${escapeAttribute(item.source_page_url)}" target="_blank" rel="noreferrer">Source page</a>
              <a href="${escapeAttribute(item.media_url)}" target="_blank" rel="noreferrer">Open media</a>
            </div>
          </div>
          <button class="secondary-button selection-remove" data-remove-selection="${escapeAttribute(item.key)}">Remove</button>
        </article>
      `,
    )
    .join("");

  for (const button of elements.selectionTrayList.querySelectorAll("[data-remove-selection]")) {
    button.addEventListener("click", () => {
      const key = button.dataset.removeSelection;
      if (!key) {
        return;
      }
      state.search.selectedKeys.delete(key);
      state.search.selectedItems.delete(key);
      renderSearchMeta();
      renderPreviewResults();
      renderSelectionTray();
    });
  }
}

function renderPreviewResults() {
  const preview = state.search.preview;
  if (!preview) {
    elements.searchPreviewResults.innerHTML = `
      <div class="empty-state">
        Search previews will land here in grouped 3-page batches so the app stays punchy and kind to source sites.
      </div>
    `;
    return;
  }

  if (!preview.source_batches.length) {
    elements.searchPreviewResults.innerHTML = `
      <div class="empty-state">No sources were ready for this search. Enable a supported source and try again.</div>
    `;
    return;
  }

  elements.searchPreviewResults.innerHTML = preview.source_batches
    .map((batch) => `
      <section class="preview-source-block">
        <div class="section-heading-row">
          <div>
            <h3>${escapeHtml(batch.source_name)}</h3>
            <p class="muted-copy">${escapeHtml(batch.note)}</p>
          </div>
          <span class="list-badge">${escapeHtml(batch.media_kind)}</span>
        </div>
        ${batch.pages.map((page) => `
          <div class="page-group">
            <div class="page-group-heading">
              <h4>Page ${page.page_number}</h4>
              <span class="section-note">${page.items.length} preview item${page.items.length === 1 ? "" : "s"}</span>
            </div>
            <div class="preview-grid">
              ${page.items.length
                ? page.items.map((item) => previewCard(item)).join("")
                : `<div class="empty-state compact">No preview items on this page.</div>`}
            </div>
          </div>
        `).join("")}
      </section>
    `)
    .join("");

  for (const checkbox of elements.searchPreviewResults.querySelectorAll("[data-preview-key]")) {
    checkbox.addEventListener("change", (event) => {
      const key = event.currentTarget.dataset.previewKey;
      if (!key) {
        return;
      }
      const item = lookupPreviewItem(key);
      if (event.currentTarget.checked) {
        state.search.selectedKeys.add(key);
        if (item) {
          state.search.selectedItems.set(key, item);
        }
      } else {
        state.search.selectedKeys.delete(key);
        state.search.selectedItems.delete(key);
      }
      renderSearchMeta();
      renderSelectionTray();
    });
  }
}

function previewCard(item) {
  const checked = state.search.selectedKeys.has(item.key) ? "checked" : "";
  return `
    <article class="preview-card">
      <label class="preview-select">
        <input type="checkbox" data-preview-key="${escapeAttribute(item.key)}" ${checked} />
        <span>Use later</span>
      </label>
      <div class="preview-thumb">
        ${renderPreviewMedia(item)}
      </div>
      <div class="preview-copy">
        <h4>${escapeHtml(item.title)}</h4>
        <p>${escapeHtml(item.creator || "Unknown creator")} ${item.license ? `| ${escapeHtml(item.license)}` : ""}</p>
        <div class="preview-links">
          <a href="${escapeAttribute(item.source_page_url)}" target="_blank" rel="noreferrer">Source page</a>
          <a href="${escapeAttribute(item.media_url)}" target="_blank" rel="noreferrer">Open media</a>
        </div>
      </div>
    </article>
  `;
}

function renderPreviewMedia(item) {
  if (item.kind === "Image" && item.thumb_url) {
    return `<img src="${escapeAttribute(item.thumb_url)}" alt="${escapeAttribute(item.title)}" loading="lazy" />`;
  }
  if (item.kind === "Audio" && item.preview_url) {
    return `<audio controls preload="none" src="${escapeAttribute(item.preview_url)}"></audio>`;
  }
  return `<div class="preview-fallback">${escapeHtml(item.kind)}</div>`;
}

function renderModelList() {
  if (!state.dashboard) {
    return;
  }

  const items = state.dashboard.materials.model_summary.items;
  if (!items.length) {
    elements.modelsList.innerHTML = `<div class="empty-state">No local model files detected yet. Drop candidate base models or helper weights into models/.</div>`;
    return;
  }

  elements.modelsList.innerHTML = items
    .map(
      (item) => `
        <article class="list-item">
          <div>
            <h4>${escapeHtml(item.name)}</h4>
            <p>${escapeHtml(item.relative_path)}</p>
          </div>
          <span class="list-badge">${escapeHtml(item.kind)}</span>
        </article>
      `,
    )
    .join("");
}

function renderRuntime() {
  if (!state.dashboard) {
    return;
  }

  const runtime = state.dashboard.materials.runtime_summary;
  const cards = [
    runtimeCard("Llama CLI", runtime.llama_cli_ready),
    runtimeCard("Vulkan DLL", runtime.vulkan_runtime_ready),
    runtimeCard("Diffusion Runtime Folder", runtime.diffusion_runtime_present),
  ];
  elements.runtimeGrid.innerHTML = cards.join("");
  elements.runtimeGrid.insertAdjacentHTML(
    "beforeend",
    `<div class="runtime-notes">${runtime.notes.map((note) => `<p>${escapeHtml(note)}</p>`).join("")}</div>`,
  );

  elements.runtimePill.textContent = runtime.vulkan_runtime_ready
    ? "Vulkan runtime detected"
    : "Runtime still incomplete";
}

function renderBuilder() {
  if (!state.dashboard) {
    return;
  }

  const builder = state.dashboard.builder;
  const selectedDataset = getSelectedDataset();
  const selectedBackend = getSelectedTrainingBackend();
  elements.builderStatusList.innerHTML = builder.status_lines
    .map((line) => `<li>${escapeHtml(line)}</li>`)
    .join("");
  elements.builderNotesList.innerHTML = builder.starter_notes
    .map((line) => `<li>${escapeHtml(line)}</li>`)
    .join("");

  const outputs = state.dashboard.materials.output_summary;
  const models = state.dashboard.materials.model_summary;
  const sources = state.dashboard.materials.source_registry;
  elements.builderDatasetSummary.innerHTML = selectedDataset
    ? [
        summaryCard("Selected dataset", selectedDataset.display_name, selectedDataset.relative_path),
        summaryCard("Files", String(selectedDataset.total_files), `${selectedDataset.images} images, ${selectedDataset.audio} audio, ${selectedDataset.video} video`),
        summaryCard("Sources", String(selectedDataset.source_count), selectedDataset.manifest_present ? "Metadata manifest detected" : "No metadata manifest yet"),
        summaryCard("Other files", String(selectedDataset.other), "Captions, notes, or uncategorized files"),
        summaryCard("Saved outputs", String(outputs.total), "Useful later for synthetic training passes"),
        summaryCard("Base models", String(models.total), "Local candidates visible to the builder"),
        summaryCard("Backend target", selectedBackend ? selectedBackend.name : "Not chosen yet", selectedBackend ? (selectedBackend.ready ? "Looks locally ready" : "Plan target only for now") : "Choose a trainer family before you save"),
      ].join("")
    : [
        summaryCard("Enabled sources", String(sources.enabled), "Current sources available for respectful search"),
        summaryCard("Curated datasets", String(builder.curated_datasets.length), "Build one on Materials, then use it here"),
        summaryCard("Saved outputs", String(outputs.total), "Useful later for synthetic training passes"),
        summaryCard("Base models", String(models.total), "Local candidates visible to the builder"),
        summaryCard("Backend target", selectedBackend ? selectedBackend.name : "Not chosen yet", selectedBackend ? (selectedBackend.ready ? "Looks locally ready" : "Plan target only for now") : "Choose a trainer family before you save"),
      ].join("");

  renderDatasetPreflight(selectedDataset, selectedBackend);
  renderCuratedDatasetCards(builder.curated_datasets, selectedDataset);
  renderPreparedProjects(builder.prepared_projects);
  renderTrainingBackends(builder.training_backends, selectedBackend);
  renderWanTrainingStatus(builder.wan_training, selectedDataset, selectedBackend);
  renderBuilderGuidance();
}

function renderBuilderGuidance() {
  if (!state.dashboard) {
    return;
  }

  renderBuilderPlanState();

  const dataset = getSelectedDataset();
  const backend = getSelectedTrainingBackend();
  const concept = conceptGuidance(elements.conceptTypeSelect.value);
  const preset = presetGuidance(elements.trainingPresetSelect.value);
  const caption = captionGuidance(elements.captionStrategySelect.value);
  const settings = currentTrainingSettings();
  const mediaKind = trainingBackendMediaKind(backend);
  const pressure = settingsPressure(settings, dataset, mediaKind);
  const triggerPhrase = elements.triggerPhraseInput.value.trim();
  const conceptSummary = elements.conceptSummaryInput.value.trim();

  setHelpText(elements.conceptTypeHelp, concept.help);
  setHelpText(elements.trainingPresetHelp, preset.help);
  setHelpText(elements.captionStrategyHelp, caption.help);
  setHelpText(elements.rankHelp, rankHelp(settings.rank));
  setHelpText(elements.repeatsHelp, `Shows each training item ${settings.repeats} time${settings.repeats === 1 ? "" : "s"} per epoch.`);
  setHelpText(elements.epochsHelp, `Runs ${settings.epochs} full pass${settings.epochs === 1 ? "" : "es"} over the repeated dataset.`);
  setHelpText(elements.resolutionHelp, resolutionHelp(settings.resolution));
  setHelpText(elements.batchSizeHelp, batchSizeHelp(settings.batchSize));
  setHelpText(elements.learningRateHelp, learningRateHelp(settings.learningRate));
  setHelpText(elements.validationSplitHelp, validationSplitHelp(settings.validationSplit));

  const datasetNote = dataset
    ? mediaKind === "image"
      ? dataset.images > 0
        ? `${dataset.images} image file${dataset.images === 1 ? "" : "s"} detected. That matches the Wan image visual lane.`
        : "This dataset has no image files, so the selected Wan image lane will not have useful training rows yet."
      : dataset.video > 0
        ? `${dataset.video} video file${dataset.video === 1 ? "" : "s"} detected. That matches the Wan video lane.`
        : "This dataset has no video files, so the selected Wan video lane will not have useful training rows yet."
    : "Pick a curated dataset before saving a plan.";
  const triggerNote = triggerPhrase
    ? `Trigger phrase "${triggerPhrase}" will be inserted into generated captions.`
    : "Add a short rare trigger phrase so the trained concept has a clean handle later.";
  const summaryNote = conceptSummary
    ? "Concept summary is filled in and will be folded into generated captions."
    : "Add a short concept summary describing what should stay consistent.";

  elements.builderPlanExplanationPanel.innerHTML = `
    <div class="builder-guidance-heading">
      <div>
        <p class="summary-title">Plan readout</p>
        <h3>${escapeHtml(concept.title)}</h3>
      </div>
      <span class="list-badge ${backend?.ready ? "ok-badge" : "warm-badge"}">${escapeHtml(backend?.ready ? "backend ready" : "backend needs checks")}</span>
    </div>
    <p>${escapeHtml(concept.body)}</p>
    <div class="source-meta">
      <span class="list-badge">${escapeHtml(dataset?.display_name || "no dataset selected")}</span>
      <span class="list-badge">${escapeHtml(backend?.name || "no backend selected")}</span>
      <span class="list-badge">${escapeHtml(preset.label)}</span>
      <span class="list-badge">${escapeHtml(caption.label)}</span>
    </div>
    <ul class="bullet-list compact-bullets">
      <li>${escapeHtml(datasetNote)}</li>
      <li>${escapeHtml(triggerNote)}</li>
      <li>${escapeHtml(summaryNote)}</li>
      <li>${escapeHtml(caption.body)}</li>
    </ul>
  `;

  elements.builderSettingsExplanationPanel.innerHTML = `
    <div class="builder-guidance-heading">
      <div>
        <p class="summary-title">Settings readout</p>
        <h3>${escapeHtml(pressure.label)}</h3>
      </div>
      <span class="list-badge ${pressure.badgeClass}">${escapeHtml(pressure.badge)}</span>
    </div>
    <p>${escapeHtml(pressure.body)}</p>
    <div class="source-meta">
      <span class="list-badge">${settings.resolution}px</span>
      <span class="list-badge">rank ${settings.rank}</span>
      <span class="list-badge">batch ${settings.batchSize}</span>
      <span class="list-badge">${settings.repeats} repeat${settings.repeats === 1 ? "" : "s"} x ${settings.epochs} epoch${settings.epochs === 1 ? "" : "s"}</span>
      <span class="list-badge">${escapeHtml(formatLearningRate(settings.learningRate))}</span>
    </div>
    <ul class="bullet-list compact-bullets">
      ${pressure.notes.map((note) => `<li>${escapeHtml(note)}</li>`).join("")}
    </ul>
  `;
}

function renderBuilderPlanState() {
  const loadedName = state.builder.loadedProjectName;
  const draftMode = state.builder.draftMode;
  const dirty = isBuilderFormDirty();
  updatePrepareProjectButtonLabel();

  if (draftMode === "copy") {
    elements.builderPlanStatePanel.innerHTML = `
      <div class="builder-guidance-heading">
        <div>
          <p class="summary-title">Draft copy</p>
          <h3>Loaded as a new branch</h3>
        </div>
        <span class="list-badge warm-badge">not saved yet</span>
      </div>
      <p>This form was copied from a saved plan, but the runner cannot launch it until you click <strong>Save training plan</strong>.</p>
    `;
    return;
  }

  if (state.builder.loadedProjectSlug) {
    elements.builderPlanStatePanel.innerHTML = `
      <div class="builder-guidance-heading">
        <div>
          <p class="summary-title">Loaded saved plan</p>
          <h3>${escapeHtml(loadedName || state.builder.loadedProjectSlug)}</h3>
        </div>
        <span class="list-badge ${dirty ? "warm-badge" : "ok-badge"}">${dirty ? "unsaved edits" : "matches saved card"}</span>
      </div>
      <p>${dirty
        ? "The form has changed since this plan was loaded. Click Save training plan to create a safe edited copy, or run the saved card below if you want the original."
        : "The form currently matches the loaded saved card. Running the card below will use these saved values."}</p>
    `;
    return;
  }

  elements.builderPlanStatePanel.innerHTML = `
    <p>Editing a fresh training plan. Saved plan cards below are the things the runner can actually launch.</p>
  `;
}

function updatePrepareProjectButtonLabel() {
  if (elements.prepareProjectButton.disabled) {
    return;
  }

  if (state.builder.draftMode === "copy") {
    elements.prepareProjectButton.textContent = "Save copied plan";
  } else if (state.builder.loadedProjectSlug && isBuilderFormDirty()) {
    elements.prepareProjectButton.textContent = "Save edited copy";
  } else {
    elements.prepareProjectButton.textContent = "Save training plan";
  }
}

function currentTrainingSettings() {
  return {
    rank: clampNumber(toMaybeNumber(elements.rankInput.value) || 8, 1, 999),
    repeats: clampNumber(toMaybeNumber(elements.repeatsInput.value) || 1, 1, 999),
    epochs: clampNumber(toMaybeNumber(elements.epochsInput.value) || 1, 1, 999),
    resolution: clampNumber(toMaybeNumber(elements.resolutionSelect.value) || 512, 1, 4096),
    batchSize: clampNumber(toMaybeNumber(elements.batchSizeInput.value) || 1, 1, 999),
    learningRate: toMaybeFloat(elements.learningRateInput.value) || 0.0001,
    validationSplit: clampNumber(toMaybeNumber(elements.validationSplitInput.value) || 0, 0, 100),
  };
}

function getBuilderFormSnapshot() {
  return {
    project_name: elements.projectNameInput.value.trim(),
    dataset_slug: elements.datasetSelect.value,
    base_model: elements.baseModelSelect.value,
    training_backend_id: elements.trainingBackendSelect.value,
    trigger_phrase: elements.triggerPhraseInput.value.trim(),
    concept_summary: elements.conceptSummaryInput.value.trim(),
    concept_type: elements.conceptTypeSelect.value,
    training_preset: elements.trainingPresetSelect.value,
    caption_strategy: elements.captionStrategySelect.value,
    rank: toMaybeNumber(elements.rankInput.value) || 0,
    repeats: toMaybeNumber(elements.repeatsInput.value) || 0,
    epochs: toMaybeNumber(elements.epochsInput.value) || 0,
    resolution: toMaybeNumber(elements.resolutionSelect.value) || 0,
    batch_size: toMaybeNumber(elements.batchSizeInput.value) || 0,
    learning_rate: toMaybeFloat(elements.learningRateInput.value) || 0,
    validation_split_percent: toMaybeNumber(elements.validationSplitInput.value) || 0,
  };
}

function projectToBuilderSnapshot(project) {
  return {
    project_name: project.project_name || "",
    dataset_slug: project.dataset_slug || "",
    base_model: project.base_model || "",
    training_backend_id: project.training_backend_id || "",
    trigger_phrase: project.trigger_phrase || "",
    concept_summary: project.concept_summary || "",
    concept_type: project.concept_type || "style",
    training_preset: project.training_preset || "balanced",
    caption_strategy: project.caption_strategy || "source-title",
    rank: Number(project.rank || 0),
    repeats: Number(project.repeats || 0),
    epochs: Number(project.epochs || 0),
    resolution: Number(project.resolution || 0),
    batch_size: Number(project.batch_size || 0),
    learning_rate: Number(project.learning_rate || 0),
    validation_split_percent: Number(project.validation_split_percent || 0),
  };
}

function isBuilderFormDirty() {
  if (!state.builder.loadedSnapshot) {
    return false;
  }
  return JSON.stringify(getBuilderFormSnapshot()) !== JSON.stringify(state.builder.loadedSnapshot);
}

function conceptGuidance(value) {
  const guidance = {
    style: {
      title: "Style / aesthetic LoRA",
      body: "Best when the dataset shares a look: color language, texture, composition, rendering style, or camera mood.",
      help: "Use this when the look matters more than one exact subject.",
    },
    character: {
      title: "Character / person LoRA",
      body: "Best when identity consistency matters: face, body shape, outfit language, markings, or a recognizable subject.",
      help: "Use this when the same subject needs to stay recognizable.",
    },
    motion: {
      title: "Motion / action LoRA",
      body: "Best for Wan video concepts where movement pattern matters: how a subject turns, flies, walks, gestures, or animates.",
      help: "Use this for video-first concepts where the movement is the lesson.",
    },
    object: {
      title: "Object / product LoRA",
      body: "Best when the model needs to learn a specific item, prop, vehicle, product, or physical form.",
      help: "Use this when shape and object details matter most.",
    },
    location: {
      title: "Location / environment LoRA",
      body: "Best when the place itself is the concept: architecture, room layout, landscape, weather, or atmosphere.",
      help: "Use this when the setting is the thing being taught.",
    },
    assistant: {
      title: "Assistant / persona LoRA",
      body: "Useful later for text/persona lanes. For the current Wan lane, prefer style, character, object, location, or motion.",
      help: "Future-facing for non-video personality or assistant training lanes.",
    },
  };
  return guidance[value] || guidance.style;
}

function presetGuidance(value) {
  const guidance = {
    balanced: {
      label: "Balanced starter",
      body: "A cautious default for learning without immediately overcooking the dataset.",
      help: "General-purpose starting point.",
    },
    "identity-strong": {
      label: "Identity strong",
      body: "Use when you care more about a subject staying recognizable than broad flexibility.",
      help: "Leans toward stronger subject lock-in.",
    },
    "style-lite": {
      label: "Style light touch",
      body: "Use when you want a flavor or aesthetic without forcing every output to look identical.",
      help: "Leans gentler so the base model still breathes.",
    },
    "fast-test": {
      label: "Fast test pass",
      body: "A smoke test preset: prove the pipeline works before spending hours on a serious run.",
      help: "Best for confirming the machine and dataset behave.",
    },
  };
  return guidance[value] || guidance.balanced;
}

function captionGuidance(value) {
  const guidance = {
    "source-title": {
      label: "Source titles",
      body: "Source titles are convenient when crawled material has useful names, but noisy titles can teach noisy associations.",
      help: "Good when source names are descriptive and not junk.",
    },
    "filename-only": {
      label: "Filenames",
      body: "Filename captions are predictable for a small hand-curated set, especially when you renamed files intentionally.",
      help: "Good for small clean local tests.",
    },
    "manual-later": {
      label: "Manual captions later",
      body: "Manual captions are the most work, but they give the cleanest control once the pipeline is proven.",
      help: "Best quality path once you know what you are training.",
    },
  };
  return guidance[value] || guidance["source-title"];
}

function settingsPressure(settings, dataset, mediaKind = "video") {
  const itemCount = mediaKind === "image"
    ? dataset?.images || dataset?.total_files || 0
    : dataset?.video || dataset?.total_files || 0;
  const repeatedSteps = Math.max(1, itemCount) * settings.repeats * settings.epochs;
  let score = 0;
  const notes = [];

  if (settings.resolution <= 512) {
    notes.push(`512px is the proven low-VRAM starting point for the Wan ${mediaKind} lane.`);
  } else if (settings.resolution === 768) {
    score += 3;
    notes.push("768px is a meaningful jump in memory pressure. Try it after 512px is repeatable.");
  } else {
    score += 6;
    notes.push("1024px is ambitious for the current 8GB AMD route and may run out of VRAM.");
  }

  if (settings.batchSize === 1) {
    notes.push("Batch size 1 is the safest choice for Radeon/ROCm low-VRAM training.");
  } else {
    score += 4;
    notes.push("Batch size above 1 multiplies memory pressure quickly. Expect trouble on 8GB cards.");
  }

  if (settings.rank <= 8) {
    notes.push("Rank 8 is small, fast, and good for smoke tests.");
  } else if (settings.rank <= 16) {
    score += 1;
    notes.push("Rank 16 gives the LoRA more room while staying fairly modest.");
  } else if (settings.rank <= 32) {
    score += 2;
    notes.push("Rank 32 can learn more nuance, but it is no longer a tiny test.");
  } else {
    score += 4;
    notes.push("High ranks add capacity and file size, but can overfit or increase training cost.");
  }

  if (settings.repeats * settings.epochs <= 1) {
    notes.push("One repeat and one epoch is a true pipeline check, not a polished final training run.");
  } else if (settings.repeats * settings.epochs <= 4) {
    score += 1;
    notes.push("A few passes can strengthen learning without going straight to overfit country.");
  } else {
    score += 3;
    notes.push("Many repeats/epochs can burn in the dataset too hard, especially with only a few clips.");
  }

  if (settings.learningRate <= 0.00005) {
    notes.push("The learning rate is gentle. Slower, but less likely to mangle the concept.");
  } else if (settings.learningRate <= 0.0001) {
    notes.push("0.0001 is the current starter learning rate for the first Wan smoke tests.");
  } else if (settings.learningRate <= 0.0003) {
    score += 1;
    notes.push("This learning rate is more assertive. Watch for overcooked results.");
  } else {
    score += 3;
    notes.push("This learning rate is hot for a beginner run and may damage quality quickly.");
  }

  if (settings.validationSplit === 0) {
    notes.push("Validation is off. That is fine for tiny smoke tests; add it later for serious comparisons.");
  } else {
    notes.push(`${settings.validationSplit}% of the dataset will be held back from training for validation-style checking.`);
  }

  if (itemCount > 0) {
    notes.push(`Rough exposure count: ${repeatedSteps} ${mediaKind}-pass${repeatedSteps === 1 ? "" : "es"} before optimizer details.`);
  }

  if (score <= 1) {
    return {
      label: "Smoke-test friendly",
      badge: "low pressure",
      badgeClass: "ok-badge",
      body: "These settings are conservative and match the spirit of the Wan smoke-test route: prove the pipeline, then scale up.",
      notes,
    };
  }
  if (score <= 4) {
    return {
      label: "Cautious but warmer",
      badge: "moderate",
      badgeClass: "",
      body: "This should still be reasonable if the 512px starter run is already repeatable on your machine.",
      notes,
    };
  }
  if (score <= 7) {
    return {
      label: "Spicy for low VRAM",
      badge: "watch closely",
      badgeClass: "warm-badge",
      body: "These settings are pushing past the first safe lane. Try one change at a time so failures are easy to understand.",
      notes,
    };
  }
  return {
    label: "Here be dragons",
    badge: "high risk",
    badgeClass: "warm-badge",
    body: "This combination is likely to hit memory pressure or produce noisy learning on the current consumer AMD setup.",
    notes,
  };
}

function rankHelp(value) {
  if (value <= 8) {
    return "Small adapter. Safest first check.";
  }
  if (value <= 16) {
    return "More room to learn, still modest.";
  }
  if (value <= 32) {
    return "Stronger capacity, more chance to overfit tiny datasets.";
  }
  return "Advanced territory. Bigger is not automatically better.";
}

function resolutionHelp(value) {
  if (value <= 512) {
    return "Safest tested size for the current Wan/AMD route.";
  }
  if (value === 768) {
    return "Sharper, but notably heavier.";
  }
  return "Very heavy for current low-VRAM testing.";
}

function batchSizeHelp(value) {
  return value <= 1
    ? "One item at a time. Slow but survivable."
    : "Faster if it fits, but memory pressure jumps hard.";
}

function learningRateHelp(value) {
  if (value <= 0.00005) {
    return "Gentle learning. Safer but slower.";
  }
  if (value <= 0.0001) {
    return "Current starter value.";
  }
  if (value <= 0.0003) {
    return "Assertive. Watch output quality.";
  }
  return "Hot. Easy to overcook a small dataset.";
}

function validationSplitHelp(value) {
  return value <= 0
    ? "Off for tiny smoke tests."
    : "Holds back data for checking instead of training.";
}

function setHelpText(element, value) {
  if (element) {
    element.textContent = value;
  }
}

function clampNumber(value, min, max) {
  return Math.min(Math.max(value, min), max);
}

function formatLearningRate(value) {
  return Number(value || 0).toLocaleString(undefined, {
    maximumFractionDigits: 8,
  });
}

function renderHelperMessages() {
  const messages = state.helper.messages.length
    ? state.helper.messages
    : [
        {
          role: "assistant",
          content:
            state.page === "materials"
              ? "I can help you pick respectful sources, troubleshoot empty searches, and judge whether a preview selection looks strong enough to curate."
              : "I can help you judge dataset quality, pick a trigger phrase, and explain what the current training settings are likely to do.",
          suggestions: defaultHelperSuggestions(),
        },
      ];

  elements.helperMessages.innerHTML = messages
    .map((message) => `
      <article class="helper-message ${escapeAttribute(message.role)}">
        <div class="helper-message-meta">${message.role === "user" ? "You" : "Helper"}</div>
        <p>${escapeHtml(message.content)}</p>
        ${Array.isArray(message.suggestions) && message.suggestions.length
          ? `<ul class="helper-suggestions">${message.suggestions.map((suggestion) => `<li>${escapeHtml(suggestion)}</li>`).join("")}</ul>`
          : ""}
      </article>
    `)
    .join("");

  elements.helperMessages.scrollTop = elements.helperMessages.scrollHeight;
}

function renderHelperQuickActions() {
  const actions = state.page === "materials"
    ? [
        "Why did this search return nothing?",
        "Is this enough material to curate?",
        "How many sources should I enable at once?",
      ]
    : [
        "Is this enough data for a LoRA?",
        "What does rank actually do?",
        "How should I think about repeats and epochs?",
        "Which training backend should I aim for?",
      ];

  elements.helperQuickActions.innerHTML = actions
    .map(
      (label) => `
        <button class="helper-quick-button" type="button" data-helper-quick="${escapeAttribute(label)}">
          ${escapeHtml(label)}
        </button>
      `,
    )
    .join("");

  for (const button of elements.helperQuickActions.querySelectorAll("[data-helper-quick]")) {
    button.addEventListener("click", () => {
      elements.helperInput.value = button.dataset.helperQuick || "";
      elements.helperInput.focus();
    });
  }
}

function seedBuilderForm() {
  if (!state.dashboard) {
    return;
  }

  const currentBaseModel = elements.baseModelSelect.value;
  populateDatasetSelect();
  populateTrainingBackendSelect();
  syncProjectNameSuggestion();

  const options = state.dashboard.builder.base_model_options;
  const markup = options.length
    ? options
        .map((option) => `<option value="${escapeAttribute(option)}">${escapeHtml(option)}</option>`)
        .join("")
    : `<option value="">No base models detected yet</option>`;
  elements.baseModelSelect.innerHTML = markup;
  if (currentBaseModel && options.includes(currentBaseModel)) {
    elements.baseModelSelect.value = currentBaseModel;
  }

  if (!elements.triggerPhraseInput.value) {
    elements.triggerPhraseInput.value = "chatty_lora_subject";
  }
}

function populateTrainingBackendSelect() {
  const backends = state.dashboard.builder.training_backends;
  const requestedId = state.builder.selectedTrainingBackendId || state.dashboard.builder.recommended_training_backend_id || "";
  const selectedBackend = backends.find((backend) => backend.id === requestedId) || backends[0] || null;

  state.builder.selectedTrainingBackendId = selectedBackend ? selectedBackend.id : "";

  const markup = backends.length
    ? backends
        .map((backend) => {
          const detail = backend.ready ? "ready" : "not ready yet";
          return `<option value="${escapeAttribute(backend.id)}">${escapeHtml(backend.name)} (${escapeHtml(detail)})</option>`;
        })
        .join("")
    : `<option value="">No training backends detected yet</option>`;

  elements.trainingBackendSelect.innerHTML = markup;
  elements.trainingBackendSelect.disabled = backends.length === 0;
  elements.trainingBackendSelect.value = state.builder.selectedTrainingBackendId;
}

function populateDatasetSelect() {
  const datasets = state.dashboard.builder.curated_datasets;
  const requestedSlug = state.builder.selectedDatasetSlug || state.dashboard.builder.recommended_dataset_slug || "";
  const selectedDataset = datasets.find((dataset) => dataset.slug === requestedSlug) || datasets[0] || null;

  state.builder.selectedDatasetSlug = selectedDataset ? selectedDataset.slug : "";

  const markup = datasets.length
    ? datasets
        .map((dataset) => {
          const detail = `${dataset.total_files} file${dataset.total_files === 1 ? "" : "s"} | ${dataset.images} img | ${dataset.audio} aud | ${dataset.video} vid`;
          return `<option value="${escapeAttribute(dataset.slug)}">${escapeHtml(dataset.display_name)} (${escapeHtml(detail)})</option>`;
        })
        .join("")
    : `<option value="">No curated datasets yet</option>`;

  elements.datasetSelect.innerHTML = markup;
  elements.datasetSelect.disabled = datasets.length === 0;
  elements.datasetSelect.value = state.builder.selectedDatasetSlug;
}

function trainingBackendMediaKind(backendOrId) {
  const id = typeof backendOrId === "string" ? backendOrId : backendOrId?.id || "";
  return id.includes("image") ? "image" : "video";
}

function datasetLaneReadiness(dataset, backend) {
  const mediaKind = trainingBackendMediaKind(backend);
  if (!dataset) {
    return {
      mediaKind,
      status: "waiting",
      label: "Dataset preflight",
      badge: "waiting",
      badgeClass: "muted-badge",
      count: 0,
      notes: ["Choose or curate a dataset and Chatty-lora will sanity-check it for this Wan lane."],
    };
  }

  if (mediaKind === "image") {
    const notes = [];
    let status = "ok";
    let label = "Image visual set";
    let badge = "good starter";
    const count = Number(dataset.images || 0);

    if (count === 0) {
      status = "blocked";
      label = "No image rows";
      badge = "blocked";
      notes.push("The Wan image visual lane needs still images. Video files are useful for the video lane, but this handoff will ignore them.");
    } else if (count < 8) {
      status = "caution";
      label = "Very thin image set";
      badge = "thin";
      notes.push("One to seven images can prove the wiring, but the resulting LoRA will probably overfit or learn weakly.");
    } else if (count < 20) {
      label = "Image smoke-test ready";
      badge = "smoke test";
      notes.push("This is enough for an image-lane smoke test: useful for proving image metadata, caching, and training handoff.");
    } else {
      label = "Small focused image set";
      notes.push("This is a useful starting size for a focused image concept, assuming the images are coherent.");
    }

    if (!dataset.manifest_present) {
      notes.push("No curation manifest was found. Hand-added images are fine, but source labels and original URLs will be limited.");
    }
    if ((dataset.video || 0) > 0 || (dataset.audio || 0) > 0) {
      notes.push(`This folder also contains ${dataset.video || 0} video file(s) and ${dataset.audio || 0} audio file(s). The image lane will leave those alone.`);
    }
    if ((dataset.preflight?.caption_files || 0) === 0) {
      notes.push("No sidecar caption files were found. Generated captions will lean on trigger phrase, filename/title, and concept summary.");
    }

    return {
      mediaKind,
      status,
      label,
      badge,
      badgeClass: datasetPreflightClass(status),
      count,
      notes,
    };
  }

  const preflight = dataset.preflight || {};
  return {
    mediaKind,
    status: preflight.status || "ok",
    label: preflight.label || "Video dataset preflight",
    badge: preflight.badge || "checked",
    badgeClass: datasetPreflightClass(preflight.status),
    count: Number(dataset.video || 0),
    notes: Array.isArray(preflight.notes) ? preflight.notes : [],
  };
}

function renderDatasetPreflight(dataset, backend) {
  if (!elements.builderDatasetPreflight) {
    return;
  }

  if (!dataset) {
    elements.builderDatasetPreflight.innerHTML = `
      <div class="dataset-preflight-panel muted">
        <div class="builder-guidance-heading">
          <div>
            <h3>Dataset preflight</h3>
            <p>Choose or curate a dataset and Chatty-lora will sanity-check it for the first Wan training lane.</p>
          </div>
          <span class="list-badge muted-badge">waiting</span>
        </div>
      </div>
    `;
    return;
  }

  const preflight = dataset.preflight || {};
  const readiness = datasetLaneReadiness(dataset, backend);
  const timingParts = [];
  if (readiness.mediaKind === "video" && Number.isFinite(Number(preflight.total_duration_seconds))) {
    timingParts.push(`total ${formatDuration(preflight.total_duration_seconds)}`);
  }
  if (readiness.mediaKind === "video" && Number.isFinite(Number(preflight.min_duration_seconds)) && Number.isFinite(Number(preflight.max_duration_seconds))) {
    timingParts.push(`clip range ${formatDuration(preflight.min_duration_seconds)} to ${formatDuration(preflight.max_duration_seconds)}`);
  }
  if (readiness.mediaKind === "video" && Number.isFinite(Number(preflight.probed_video_count)) && preflight.probed_video_count > 0) {
    timingParts.push(`${preflight.probed_video_count} probed`);
  }

  const resolutions = readiness.mediaKind === "video" && Array.isArray(preflight.resolution_summary) ? preflight.resolution_summary : [];
  const videoDetails = readiness.mediaKind === "video" && Array.isArray(preflight.video_details) ? preflight.video_details : [];
  const notes = readiness.notes;
  const badgeClass = readiness.badgeClass;
  const mediaLabel = readiness.mediaKind === "image" ? "image" : "video";

  elements.builderDatasetPreflight.innerHTML = `
    <div class="dataset-preflight-panel ${escapeAttribute(readiness.status || "ok")}">
      <div class="builder-guidance-heading">
        <div>
          <h3>${escapeHtml(readiness.label || "Dataset preflight")}</h3>
          <p>${escapeHtml(dataset.display_name)} is being checked against the selected Wan 2.1 T2V ${escapeHtml(mediaLabel)} lane.</p>
        </div>
        <span class="list-badge ${badgeClass}">${escapeHtml(readiness.badge || "checked")}</span>
      </div>
      <div class="source-meta dataset-preflight-badges">
        <span class="list-badge">${readiness.count} ${escapeHtml(mediaLabel)}</span>
        <span class="list-badge">${dataset.images} images total</span>
        <span class="list-badge">${dataset.video} videos total</span>
        <span class="list-badge">${preflight.caption_files || 0} captions</span>
        <span class="list-badge">${dataset.manifest_present ? "manifest" : "no manifest"}</span>
        ${timingParts.map((part) => `<span class="list-badge">${escapeHtml(part)}</span>`).join("")}
        ${resolutions.map((resolution) => `<span class="list-badge">${escapeHtml(resolution)}</span>`).join("")}
        ${readiness.mediaKind === "video" && !preflight.video_probe_available ? '<span class="list-badge muted-badge">ffprobe not found</span>' : ""}
      </div>
      <ul class="bullet-list dataset-preflight-notes">
        ${notes.map((note) => `<li>${escapeHtml(note)}</li>`).join("")}
      </ul>
      ${renderDatasetVideoDetails(videoDetails)}
    </div>
  `;
}

function renderDatasetVideoDetails(videoDetails) {
  if (!videoDetails.length) {
    return "";
  }

  return `
    <div class="dataset-video-list">
      ${videoDetails
        .map((detail) => {
          const resolution = detail.width && detail.height ? `${detail.width}x${detail.height}` : "unknown size";
          const fps = Number.isFinite(Number(detail.fps)) ? `${Number(detail.fps).toFixed(1)} fps` : "unknown fps";
          const duration = Number.isFinite(Number(detail.duration_seconds)) ? formatDuration(detail.duration_seconds) : "unknown duration";
          return `
            <div class="dataset-video-row">
              <strong>${escapeHtml(detail.name || detail.relative_path || "video clip")}</strong>
              <span>${escapeHtml(duration)} | ${escapeHtml(resolution)} | ${escapeHtml(fps)}</span>
            </div>
          `;
        })
        .join("")}
    </div>
  `;
}

function renderCuratedDatasetCards(datasets, selectedDataset) {
  if (!datasets.length) {
    elements.curatedDatasetsList.innerHTML = `
      <div class="empty-state">
        Curated datasets will appear here after you pick search results on the Materials page and let Chatty-lora download and organize them.
      </div>
    `;
    return;
  }

  elements.curatedDatasetsList.innerHTML = `
    <div class="section-heading-row curated-dataset-heading">
      <h3>Curated datasets</h3>
      <span class="section-note">Anything you curate on Materials is ready to use here.</span>
    </div>
    <div class="dataset-card-grid">
      ${datasets
        .map((dataset) => {
          const selected = selectedDataset && selectedDataset.slug === dataset.slug;
          return `
            <article class="dataset-card ${selected ? "selected" : ""}">
              <div class="dataset-card-copy">
                <div class="dataset-card-title-row">
                  <h4>${escapeHtml(dataset.display_name)}</h4>
                  ${selected ? '<span class="list-badge ok-badge">selected</span>' : ""}
                </div>
                <p>${escapeHtml(dataset.relative_path)}</p>
                <div class="source-meta">
                  <span class="list-badge">${dataset.total_files} files</span>
                  <span class="list-badge">${dataset.images} img</span>
                  <span class="list-badge">${dataset.audio} aud</span>
                  <span class="list-badge">${dataset.video} vid</span>
                  <span class="list-badge">${dataset.source_count} sources</span>
                  <span class="list-badge ${datasetPreflightClass(dataset.preflight && dataset.preflight.status)}">${escapeHtml(dataset.preflight ? dataset.preflight.badge : "not checked")}</span>
                </div>
              </div>
              <button class="secondary-button" data-use-dataset="${escapeAttribute(dataset.slug)}">
                ${selected ? "Using this dataset" : "Use this dataset"}
              </button>
            </article>
          `;
        })
        .join("")}
    </div>
  `;

  for (const button of elements.curatedDatasetsList.querySelectorAll("[data-use-dataset]")) {
    button.addEventListener("click", () => {
      const slug = button.dataset.useDataset;
      if (!slug) {
        return;
      }
      state.builder.selectedDatasetSlug = slug;
      elements.datasetSelect.value = slug;
      syncProjectNameSuggestion();
      renderBuilder();
    });
  }
}

function renderPreparedProjects(projects) {
  if (!projects.length) {
    elements.preparedProjectsList.innerHTML = `
      <div class="empty-state">
        Saved training plans will appear here after you create one on the Builder page.
      </div>
    `;
    return;
  }

  const orderedProjects = orderPreparedProjects(projects);
  const telemetryHostSlug = firstRunnableProjectSlug(orderedProjects);

  elements.preparedProjectsList.innerHTML = `
    <div class="dataset-card-grid">
      ${orderedProjects
        .map((project) => {
          const activeHere = isTrainingStatusActive(state.training.status)
            && state.training.status.project_slug === project.slug;
          const deleteDisabled = activeHere || state.builder.deletingProjectSlug === project.slug;
          const deleteLabel = state.builder.deletingProjectSlug === project.slug
            ? "Deleting..."
            : "Delete saved plan";
          const deleteTitle = activeHere
            ? "Stop this training run before deleting its saved plan."
            : "Delete this saved plan card and generated handoff folder. Trained outputs are preserved.";
          return `
            <article class="dataset-card">
              <div class="dataset-card-copy">
                <div class="dataset-card-title-row">
                  <h4>${escapeHtml(project.project_name)}</h4>
                  <span class="list-badge">${escapeHtml(project.training_preset)}</span>
                </div>
                <p>${escapeHtml(project.relative_path)}</p>
                ${project.generated_training_relative_path
                  ? `<p class="muted-copy">Generated handoff: ${escapeHtml(project.generated_training_relative_path)}</p>`
                  : ""}
                <div class="source-meta">
                  <span class="list-badge">${escapeHtml(project.dataset_slug)}</span>
                  <span class="list-badge">${escapeHtml(project.concept_type)}</span>
                  <span class="list-badge">${escapeHtml(backendLabelFromDashboard(project.training_backend_id))}</span>
                  <span class="list-badge">${project.resolution}px</span>
                  <span class="list-badge">rank ${project.rank}</span>
                </div>
                <p class="muted-copy">Base model: ${escapeHtml(project.base_model)}</p>
                <div class="inline-actions compact saved-plan-actions">
                  <button class="secondary-button" type="button" data-load-project="${escapeAttribute(project.slug)}">Load into editor</button>
                  <button class="secondary-button" type="button" data-duplicate-project="${escapeAttribute(project.slug)}">Duplicate as new plan</button>
                  <button
                    class="secondary-button saved-plan-delete-button"
                    type="button"
                    title="${escapeAttribute(deleteTitle)}"
                    data-delete-project="${escapeAttribute(project.slug)}"
                    ${deleteDisabled ? "disabled" : ""}
                  >${escapeHtml(deleteLabel)}</button>
                </div>
                ${renderSavedPlanEditorWarning(project)}
                ${renderProjectHandoff(project, project.slug === telemetryHostSlug)}
              </div>
            </article>
          `;
        })
        .join("")}
    </div>
  `;

  for (const button of elements.preparedProjectsList.querySelectorAll("[data-copy-training-command]")) {
    button.addEventListener("click", () => {
      const command = button.dataset.copyTrainingCommand || "";
      const label = button.dataset.copyTrainingLabel || "training command";
      void copyTrainingCommand(command, label);
    });
  }

  for (const button of elements.preparedProjectsList.querySelectorAll("[data-run-training]")) {
    button.addEventListener("click", () => {
      const slug = button.dataset.runTraining || "";
      const mode = button.dataset.runMode || "full";
      void startTrainingRun(slug, mode);
    });
  }

  for (const button of elements.preparedProjectsList.querySelectorAll("[data-load-project]")) {
    button.addEventListener("click", () => {
      loadProjectIntoBuilder(button.dataset.loadProject || "");
    });
  }

  for (const button of elements.preparedProjectsList.querySelectorAll("[data-duplicate-project]")) {
    button.addEventListener("click", () => {
      duplicateProjectIntoBuilder(button.dataset.duplicateProject || "");
    });
  }

  for (const button of elements.preparedProjectsList.querySelectorAll("[data-delete-project]")) {
    button.addEventListener("click", () => {
      void deletePreparedProject(button.dataset.deleteProject || "");
    });
  }

  for (const button of elements.preparedProjectsList.querySelectorAll("[data-copy-training-output]")) {
    button.addEventListener("click", () => {
      const outputPath = button.dataset.copyTrainingOutput || "";
      void copyTrainingOutputPath(outputPath);
    });
  }

  for (const button of elements.preparedProjectsList.querySelectorAll("[data-open-training-output]")) {
    button.addEventListener("click", () => {
      const outputPath = button.dataset.openTrainingOutput || "";
      void openLocalPath(outputPath);
    });
  }

  for (const button of elements.preparedProjectsList.querySelectorAll("[data-stop-training]")) {
    button.addEventListener("click", () => {
      void stopTrainingRun();
    });
  }

  renderSystemTelemetry();
}

function orderPreparedProjects(projects) {
  const activeSlug = isTrainingStatusActive(state.training.status) && state.training.status.project_slug
    ? state.training.status.project_slug
    : "";
  const loadedSlug = state.builder.loadedProjectSlug || "";
  const finishedSlug = state.training.status
    && state.training.status.state !== "idle"
    && state.training.status.project_slug
    ? state.training.status.project_slug
    : "";
  const prioritySlug = activeSlug || loadedSlug || finishedSlug;

  if (!prioritySlug || !projects.some((project) => project.slug === prioritySlug)) {
    return projects;
  }

  return [...projects].sort((left, right) => {
    if (left.slug === prioritySlug && right.slug !== prioritySlug) {
      return -1;
    }
    if (right.slug === prioritySlug && left.slug !== prioritySlug) {
      return 1;
    }
    return 0;
  });
}

function firstRunnableProjectSlug(projects) {
  const preferred = projects.find((project) => project.generated_training_relative_path);
  return preferred ? preferred.slug : "";
}

function renderSavedPlanEditorWarning(project) {
  if (state.builder.loadedProjectSlug !== project.slug || !isBuilderFormDirty()) {
    return "";
  }

  return `
    <div class="plan-card-warning">
      <strong>Editor has unsaved changes.</strong>
      <p>This saved card will still run its original saved values until you click Save training plan and use the new card.</p>
    </div>
  `;
}

function renderProjectHandoff(project, showTelemetry = false) {
  if (!project.generated_training_relative_path) {
    return "";
  }

  const notes = Array.isArray(project.generated_training_notes) ? project.generated_training_notes : [];
  const commands = Array.isArray(project.generated_training_commands) ? project.generated_training_commands : [];
  const statusLabel = project.generated_training_ready ? "baby dragon ready" : "needs attention";
  const statusClass = project.generated_training_ready ? "ok-badge" : "warm-badge";
  const videoBadge = Number.isFinite(project.video_rows)
    ? `<span class="list-badge">${project.video_rows} video row${project.video_rows === 1 ? "" : "s"}</span>`
    : "";
  const imageBadge = Number.isFinite(project.image_rows)
    ? `<span class="list-badge">${project.image_rows} image row${project.image_rows === 1 ? "" : "s"}</span>`
    : "";

  return `
    <div class="baby-dragon-panel ${project.generated_training_ready ? "ready" : ""}">
      <div class="section-heading-row">
        <div>
          <h5>Wan handoff</h5>
          <p>Guided app runner plus manual WSL fallback commands for this Wan/Musubi plan.</p>
        </div>
        <div class="baby-dragon-badges">
          <span class="list-badge ${statusClass}">${statusLabel}</span>
          ${videoBadge}
          ${imageBadge}
        </div>
      </div>
      ${notes.length
        ? `<ul class="bullet-list compact-bullets">${notes.map((note) => `<li>${escapeHtml(note)}</li>`).join("")}</ul>`
        : ""}
      ${renderTrainingRunPanel(project)}
      ${showTelemetry ? renderSystemTelemetryPanel() : ""}
      ${renderSavedTrainingOutputs(project)}
      ${commands.length
        ? `<div class="training-command-list">
            ${commands.map((command) => `
              <article class="training-command-row">
                <div>
                  <strong>${escapeHtml(command.label)}</strong>
                  <p>${escapeHtml(command.description)}</p>
                  <code>${escapeHtml(command.command)}</code>
                </div>
                <button
                  class="secondary-button"
                  type="button"
                  data-copy-training-command="${escapeAttribute(command.command)}"
                  data-copy-training-label="${escapeAttribute(command.label)}"
                >Copy</button>
              </article>
            `).join("")}
          </div>`
        : `<p class="muted-copy">No run commands are available until the handoff folder and scripts exist.</p>`}
    </div>
  `;
}

function renderSystemTelemetryPanel() {
  return `
    <div class="system-telemetry-panel" id="systemTelemetryPanel">
      <div class="system-telemetry-head">
        <div>
          <h5 id="systemTelemetryLabel">ECG Window</h5>
          <p id="systemTelemetryNote">CPU and GPU heartbeat view for split cache/training workloads.</p>
        </div>
        <div class="system-telemetry-readouts">
          <span class="telemetry-readout cpu"><span>CPU</span><strong id="systemTelemetryCpuValue">--%</strong></span>
          <span class="telemetry-readout gpu"><span>GPU</span><strong id="systemTelemetryGpuValue">--%</strong></span>
        </div>
      </div>
      <svg
        id="systemTelemetrySparkline"
        class="system-telemetry-sparkline"
        viewBox="0 0 ${SYSTEM_TELEMETRY_WIDTH} ${SYSTEM_TELEMETRY_HEIGHT}"
        preserveAspectRatio="none"
        aria-hidden="true"
      >
        <polyline id="systemTelemetryCpuLine" class="system-telemetry-line cpu" points=""></polyline>
        <polyline id="systemTelemetryGpuLine" class="system-telemetry-line gpu" points=""></polyline>
      </svg>
      <div class="system-telemetry-legend">
        <span><i class="legend-swatch cpu"></i>CPU cache/prep work</span>
        <span><i class="legend-swatch gpu"></i>GPU training bursts</span>
      </div>
    </div>
  `;
}

function renderSystemTelemetry() {
  const panel = document.getElementById("systemTelemetryPanel");
  if (!panel) {
    return;
  }

  const telemetry = state.systemTelemetry || {
    supported: true,
    label: "ECG Window",
    note: "Warming up CPU/GPU activity sampling...",
    cpu_label: "CPU",
    gpu_label: "GPU",
    current_cpu_percent: 0,
    current_gpu_percent: 0,
    cpu_history: [],
    gpu_history: [],
  };
  const cpuPercent = clampPercent(telemetry.current_cpu_percent);
  const gpuPercent = clampPercent(telemetry.current_gpu_percent);
  const cpuHistory = normalizeTelemetryHistory(telemetry.cpu_history, cpuPercent);
  const gpuHistory = normalizeTelemetryHistory(telemetry.gpu_history, gpuPercent);
  const label = String(telemetry.label || "ECG Window").trim() || "ECG Window";
  const cpuLabel = String(telemetry.cpu_label || "CPU").trim() || "CPU";
  const gpuLabel = String(telemetry.gpu_label || "GPU").trim() || "GPU";
  const note = telemetry.supported === false
    ? String(telemetry.note || "ECG Window sampling is not available on this platform.")
    : String(telemetry.note || "CPU and GPU heartbeat view for split cache/training workloads.");

  panel.classList.toggle("unsupported", telemetry.supported === false);
  setTextById("systemTelemetryLabel", label);
  setTextById("systemTelemetryNote", note);
  setTextById("systemTelemetryCpuValue", `${Math.round(cpuPercent)}%`);
  setTextById("systemTelemetryGpuValue", `${Math.round(gpuPercent)}%`);

  const cpuReadout = panel.querySelector(".telemetry-readout.cpu span");
  const gpuReadout = panel.querySelector(".telemetry-readout.gpu span");
  if (cpuReadout) {
    cpuReadout.textContent = cpuLabel;
  }
  if (gpuReadout) {
    gpuReadout.textContent = gpuLabel.length > 22 ? `${gpuLabel.slice(0, 22)}...` : gpuLabel;
    gpuReadout.title = gpuLabel;
  }

  const cpuLine = document.getElementById("systemTelemetryCpuLine");
  const gpuLine = document.getElementById("systemTelemetryGpuLine");
  if (cpuLine) {
    cpuLine.setAttribute("points", buildTelemetryPoints(cpuHistory));
  }
  if (gpuLine) {
    gpuLine.setAttribute("points", buildTelemetryPoints(gpuHistory));
  }
}

function normalizeTelemetryHistory(history, fallbackPercent) {
  const values = Array.isArray(history)
    ? history.map((value) => clampPercent(value)).filter((value) => Number.isFinite(value))
    : [];

  if (!values.length) {
    return [fallbackPercent, fallbackPercent];
  }

  if (values.length === 1) {
    return [values[0], values[0]];
  }

  return values;
}

function buildTelemetryPoints(history) {
  return history
    .map((value, index) => {
      const x = history.length === 1
        ? SYSTEM_TELEMETRY_WIDTH
        : (index / (history.length - 1)) * SYSTEM_TELEMETRY_WIDTH;
      const y = percentToTelemetryY(value);
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

function percentToTelemetryY(percent) {
  const innerHeight = SYSTEM_TELEMETRY_HEIGHT - 8;
  return 4 + ((100 - clampPercent(percent)) / 100) * innerHeight;
}

function clampPercent(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return 0;
  }
  return Math.max(0, Math.min(100, numeric));
}

function setTextById(id, value) {
  const element = document.getElementById(id);
  if (element) {
    element.textContent = value;
  }
}

function renderSavedTrainingOutputs(project) {
  const outputs = Array.isArray(project.trained_outputs) ? project.trained_outputs : [];
  if (!outputs.length) {
    return "";
  }

  return `
    <div class="training-output-list saved-output-list">
      <div>
        <strong>Saved LoRA output${outputs.length === 1 ? "" : "s"}</strong>
        <p class="muted-copy">Auto-saved for this plan. No separate save button is needed; running this same saved plan again may replace the same LoRA filename.</p>
      </div>
      ${outputs.map((output) => `
        <article class="training-output-row">
          <div>
            <code>${escapeHtml(output.relative_path)}</code>
            <div class="source-meta">
              <span class="list-badge">${escapeHtml(formatBytes(output.bytes))}</span>
              ${output.modified_unix_seconds ? `<span class="list-badge">saved ${escapeHtml(formatUnixSeconds(output.modified_unix_seconds))}</span>` : ""}
            </div>
          </div>
          <div class="inline-actions compact output-row-actions">
            <button
              class="secondary-button"
              type="button"
              data-copy-training-output="${escapeAttribute(output.relative_path)}"
            >Copy path</button>
            <button
              class="secondary-button"
              type="button"
              data-open-training-output="${escapeAttribute(output.relative_path)}"
            >Open folder</button>
          </div>
        </article>
      `).join("")}
    </div>
  `;
}

function renderTrainingRunPanel(project) {
  const status = state.training.status;
  const statusBelongsHere = status && status.project_slug === project.slug && status.state !== "idle";
  const activeSomewhereElse = isTrainingStatusActive(status) && status.project_slug !== project.slug;
  const activeHere = statusBelongsHere && isTrainingStatusActive(status);
  const runDisabled = !project.generated_training_ready || activeHere || activeSomewhereElse || state.training.busy;
  const runTitle = !project.generated_training_ready
    ? "Regenerate or fix the handoff before running."
    : activeSomewhereElse
      ? "Another training plan is currently running."
      : "Run preflight, cache latents, cache text, then train.";

  if (!statusBelongsHere) {
    return `
      <div class="training-run-panel">
        <div>
          <h5>App runner</h5>
          <p>Runs this saved plan card, not unsaved slider or prompt edits above. Save a new training plan first if you changed settings.</p>
          ${state.builder.loadedProjectSlug === project.slug && isBuilderFormDirty()
            ? `<p class="runner-warning">Unsaved editor changes are present. This button still runs the original saved card.</p>`
            : ""}
        </div>
        <button
          class="primary-button"
          type="button"
          title="${escapeAttribute(runTitle)}"
          data-run-training="${escapeAttribute(project.slug)}"
          data-run-mode="full"
          ${runDisabled ? "disabled" : ""}
        >Run this saved plan</button>
      </div>
    `;
  }

  const logs = Array.isArray(status.logs) ? status.logs.slice(-48) : [];
  const outputs = Array.isArray(status.output_files) ? status.output_files : [];
  const stages = Array.isArray(status.stages) ? status.stages : [];
  const stateLabel = readableTrainingState(status.state);
  const logLines = logs.length
    ? logs.map((entry) => `${entry.stage_id || "runner"} ${entry.stream || "log"} | ${entry.line || ""}`).join("\n")
    : "No log lines yet. The runner is warming up.";

  return `
    <div class="training-run-panel expanded">
      <div class="training-run-header">
        <div>
          <h5>App runner</h5>
          <p>${escapeHtml(status.message || "Training runner is active.")}</p>
        </div>
        <span class="list-badge ${trainingStateClass(status.state)}">${escapeHtml(stateLabel)}</span>
      </div>

      <div class="training-stage-grid">
        ${stages.map((stage) => `
          <span class="training-stage-chip ${trainingStateClass(stage.state)}">
            ${escapeHtml(stage.label)}
            ${stage.exit_code === null || stage.exit_code === undefined ? "" : ` (${escapeHtml(String(stage.exit_code))})`}
          </span>
        `).join("")}
      </div>

      ${outputs.length
        ? `<div class="training-output-list">
            <strong>LoRA output</strong>
            ${outputs.map((output) => `<code>${escapeHtml(output)}</code>`).join("")}
          </div>`
        : ""}

      <pre class="training-log-window">${escapeHtml(logLines)}</pre>

      <div class="inline-actions">
        <button
          class="secondary-button"
          type="button"
          data-run-training="${escapeAttribute(project.slug)}"
          data-run-mode="full"
          ${runDisabled ? "disabled" : ""}
        >Run this saved plan again</button>
        <button
          class="secondary-button"
          type="button"
          data-stop-training
          ${activeHere ? "" : "disabled"}
        >Stop run</button>
      </div>
    </div>
  `;
}

function readableTrainingState(value) {
  switch (value) {
    case "running":
      return "running";
    case "stopping":
      return "stopping";
    case "succeeded":
      return "succeeded";
    case "failed":
      return "failed";
    case "cancelled":
      return "cancelled";
    default:
      return "idle";
  }
}

function trainingStateClass(value) {
  switch (value) {
    case "succeeded":
      return "ok-badge";
    case "running":
      return "running-badge";
    case "stopping":
    case "failed":
    case "cancelled":
      return "warm-badge";
    default:
      return "";
  }
}

function renderWanTrainingStatus(status, selectedDataset, selectedBackend) {
  if (!status) {
    elements.wanTrainingStatus.innerHTML = `
      <div class="empty-state">Wan training preflight has not reported in yet.</div>
    `;
    return;
  }

  const defaults = status.recommended_defaults || {};
  const mediaKind = trainingBackendMediaKind(selectedBackend);
  const selectedDatasetNote = selectedDataset
    ? mediaKind === "image"
      ? selectedDataset.images > 0
        ? `Selected dataset has ${selectedDataset.images} image file${selectedDataset.images === 1 ? "" : "s"} ready for the Wan image visual lane.`
        : "Selected dataset has no images yet. The selected Wan image lane needs still images before training."
      : selectedDataset.video > 0
        ? `Selected dataset has ${selectedDataset.video} video file${selectedDataset.video === 1 ? "" : "s"} ready for this first Wan video lane.`
        : "Selected dataset has no videos yet. The selected Wan video lane is video-first, so curate or add videos before training."
    : "Pick a curated dataset to see whether it matches this Wan lane.";

  elements.wanTrainingStatus.innerHTML = `
    <div class="preflight-summary">
      <article class="summary-card ${status.model_bundle_ready ? "ready-card" : ""}">
        <p class="summary-title">Model bundle</p>
        <strong>${status.model_bundle_ready ? "Ready" : "Missing parts"}</strong>
        <span>DiT + T5 + VAE for Wan 2.1 T2V 1.3B</span>
      </article>
      <article class="summary-card ${status.trainer_ready ? "ready-card" : ""}">
        <p class="summary-title">Musubi trainer</p>
        <strong>${status.trainer_ready ? "Ready" : "Not ready"}</strong>
        <span>${escapeHtml(status.wsl_distro)} | ${escapeHtml(status.wsl_musubi_root)}</span>
      </article>
      <article class="summary-card ${status.ready ? "ready-card" : ""}">
        <p class="summary-title">End to end</p>
        <strong>${status.ready ? "Ready to prepare" : "Preflight blocked"}</strong>
        <span>${escapeHtml(selectedDatasetNote)}</span>
      </article>
    </div>

    <div class="wan-default-strip">
      <span class="list-badge">Resolution ${escapeHtml(String(defaults.resolution || 768))}px</span>
      <span class="list-badge">${escapeHtml(String(defaults.target_frames || 17))} frames</span>
      <span class="list-badge">${escapeHtml(String(defaults.source_fps || 16))} FPS source target</span>
      <span class="list-badge">Batch ${escapeHtml(String(defaults.batch_size || 1))}</span>
      <span class="list-badge">Rank ${escapeHtml(String(defaults.rank || 32))}</span>
    </div>

    <div class="wan-memory-note">
      <div>
        <p class="summary-title">Low-VRAM route</p>
        <strong>Designed for cautious 8GB AMD tests</strong>
      </div>
      <ul class="bullet-list compact-bullets">
        <li>Latent and text cache scripts run their heavy encoder work on CPU to avoid early ROCm memory failures.</li>
        <li>Training still uses the Radeon GPU, but enables split attention, FP8-scaled Wan weights, input offload, and swaps 20 of 30 Wan blocks through CPU memory.</li>
        <li>This is slower than a big-VRAM setup, but it proved the 512px, 17-frame, rank 8 Wan video smoke test end to end.</li>
      </ul>
    </div>

    <div class="wan-files-list">
      ${status.files.map((file) => `
        <article class="wan-file-row">
          <div>
            <strong>${escapeHtml(file.label)}</strong>
            <p>${escapeHtml(file.relative_path)}</p>
          </div>
          <div class="wan-file-badges">
            <span class="list-badge ${file.present ? "ok-badge" : "warm-badge"}">${file.present ? "found" : "missing"}</span>
            <span class="list-badge">${file.required ? "required" : "optional"}</span>
            ${file.bytes ? `<span class="list-badge">${escapeHtml(formatBytes(file.bytes))}</span>` : ""}
          </div>
        </article>
      `).join("")}
    </div>

    <ul class="bullet-list compact-bullets">
      ${status.notes.map((note) => `<li>${escapeHtml(note)}</li>`).join("")}
    </ul>
  `;
}

function renderTrainingBackends(backends, selectedBackend) {
  if (!backends.length) {
    elements.trainingBackendList.innerHTML = `
      <div class="empty-state">
        No training backend targets are registered yet. Add a supported trainer lane under runtime/ or finish the Wan/Musubi setup first.
      </div>
    `;
    return;
  }

  elements.trainingBackendList.innerHTML = `
    <div class="dataset-card-grid">
      ${backends
        .map((backend) => `
          <article class="dataset-card ${selectedBackend && selectedBackend.id === backend.id ? "selected" : ""}">
            <div class="dataset-card-copy">
              <div class="dataset-card-title-row">
                <h4>${escapeHtml(backend.name)}</h4>
                <span class="list-badge ${backend.ready ? "ok-badge" : ""}">${backend.ready ? "ready" : "not ready"}</span>
                ${selectedBackend && selectedBackend.id === backend.id ? '<span class="list-badge ok-badge">selected</span>' : ""}
              </div>
              <p>${escapeHtml(backend.description)}</p>
              <div class="source-meta">
                <span class="list-badge">Best for: ${escapeHtml(backend.best_for)}</span>
                <span class="list-badge">${escapeHtml(backend.relative_path || "No local folder detected yet")}</span>
              </div>
              <ul class="bullet-list compact-bullets">
                ${backend.notes.map((note) => `<li>${escapeHtml(note)}</li>`).join("")}
              </ul>
            </div>
          </article>
        `)
        .join("")}
    </div>
  `;
}

function getSelectedDataset() {
  if (!state.dashboard) {
    return null;
  }
  return (
    state.dashboard.builder.curated_datasets.find(
      (dataset) => dataset.slug === state.builder.selectedDatasetSlug,
    ) || null
  );
}

function getSelectedTrainingBackend() {
  if (!state.dashboard) {
    return null;
  }
  return (
    state.dashboard.builder.training_backends.find(
      (backend) => backend.id === state.builder.selectedTrainingBackendId,
    ) || null
  );
}

function getPreparedProject(slug) {
  if (!state.dashboard || !slug) {
    return null;
  }
  return state.dashboard.builder.prepared_projects.find((project) => project.slug === slug) || null;
}

function loadProjectIntoBuilder(slug) {
  const project = getPreparedProject(slug);
  if (!project) {
    elements.prepareProjectNote.textContent = "Could not find that saved plan in the current dashboard scan.";
    return;
  }

  applyProjectToBuilderForm(project);
  state.builder.loadedProjectSlug = project.slug;
  state.builder.loadedProjectName = project.project_name;
  state.builder.loadedSnapshot = projectToBuilderSnapshot(project);
  state.builder.draftMode = "";
  elements.prepareProjectNote.textContent = `Loaded "${project.project_name}" into the editor. Save training plan will create an edited copy, not overwrite it.`;
  renderBuilder();
  elements.projectNameInput.focus();
}

function duplicateProjectIntoBuilder(slug) {
  const project = getPreparedProject(slug);
  if (!project) {
    elements.prepareProjectNote.textContent = "Could not find that saved plan in the current dashboard scan.";
    return;
  }

  applyProjectToBuilderForm({
    ...project,
    project_name: `${project.project_name} copy`,
  });
  state.builder.loadedProjectSlug = "";
  state.builder.loadedProjectName = project.project_name;
  state.builder.loadedSnapshot = null;
  state.builder.draftMode = "copy";
  elements.prepareProjectNote.textContent = `Copied "${project.project_name}" into the editor. Adjust the name/settings, then save it as a new plan.`;
  renderBuilder();
  elements.projectNameInput.focus();
}

async function deletePreparedProject(slug) {
  const project = getPreparedProject(slug);
  if (!project || state.builder.deletingProjectSlug) {
    elements.prepareProjectNote.textContent = "Could not find that saved plan in the current dashboard scan.";
    return;
  }

  const activeHere = isTrainingStatusActive(state.training.status)
    && state.training.status.project_slug === project.slug;
  if (activeHere) {
    elements.prepareProjectNote.textContent = "Stop this training run before deleting its saved plan.";
    return;
  }

  const confirmed = window.confirm(
    `Delete saved training plan "${project.project_name}"?\n\nThis removes the saved card and generated handoff folder. Trained LoRA outputs stay in outputs/training.`,
  );
  if (!confirmed) {
    return;
  }

  state.builder.deletingProjectSlug = project.slug;
  renderBuilder();
  elements.prepareProjectNote.textContent = `Deleting "${project.project_name}"...`;

  try {
    const response = await fetch("/api/builder/delete", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ project_slug: project.slug }),
    });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || `Delete saved plan failed with ${response.status}`);
    }

    if (state.builder.loadedProjectSlug === project.slug) {
      state.builder.loadedProjectSlug = "";
      state.builder.loadedProjectName = project.project_name;
      state.builder.loadedSnapshot = null;
      state.builder.draftMode = "copy";
    }

    await loadDashboard();
    const notes = Array.isArray(payload.notes) ? payload.notes : [];
    elements.prepareProjectNote.textContent = notes.length
      ? notes.join(" ")
      : `Deleted "${project.project_name}".`;
  } catch (error) {
    console.error(error);
    elements.prepareProjectNote.textContent = `Could not delete the saved plan yet: ${String(error.message || error)}`;
  } finally {
    state.builder.deletingProjectSlug = "";
    renderBuilder();
  }
}

function applyProjectToBuilderForm(project) {
  state.builder.selectedDatasetSlug = project.dataset_slug || "";
  state.builder.selectedTrainingBackendId = project.training_backend_id || "";
  elements.projectNameInput.value = project.project_name || "";
  elements.datasetSelect.value = project.dataset_slug || "";
  elements.baseModelSelect.value = project.base_model || "";
  elements.trainingBackendSelect.value = project.training_backend_id || "";
  elements.triggerPhraseInput.value = project.trigger_phrase || "";
  elements.conceptSummaryInput.value = project.concept_summary || "";
  elements.conceptTypeSelect.value = project.concept_type || "style";
  elements.trainingPresetSelect.value = project.training_preset || "balanced";
  elements.captionStrategySelect.value = project.caption_strategy || "source-title";
  elements.rankInput.value = project.rank || 8;
  elements.repeatsInput.value = project.repeats || 1;
  elements.epochsInput.value = project.epochs || 1;
  elements.resolutionSelect.value = project.resolution || 512;
  elements.batchSizeInput.value = project.batch_size || 1;
  elements.learningRateInput.value = project.learning_rate || 0.0001;
  elements.validationSplitInput.value = project.validation_split_percent || 0;
}

function backendLabelFromDashboard(id) {
  if (!state.dashboard) {
    return id || "";
  }
  return state.dashboard.builder.training_backends.find((backend) => backend.id === id)?.name || id || "";
}

function syncProjectNameSuggestion() {
  if (!state.dashboard) {
    return;
  }

  const selectedDataset = getSelectedDataset();
  const suggestedProjectName = selectedDataset
    ? `${selectedDataset.slug}-lora`
    : state.dashboard.builder.project_name_suggestion;

  if (
    !elements.projectNameInput.value
    || elements.projectNameInput.value === state.builder.lastSeededProjectName
  ) {
    elements.projectNameInput.value = suggestedProjectName;
  }

  state.builder.lastSeededProjectName = suggestedProjectName;
}

async function persistSources() {
  try {
    const response = await fetch("/api/sources", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ sources: state.sources }),
    });
    if (!response.ok) {
      throw new Error(`Saving sources failed with ${response.status}`);
    }
    const payload = await response.json();
    state.sources = payload.sources.map((source) => ({ ...source }));
    if (state.dashboard) {
      state.dashboard.materials.source_registry = payload;
    }
    renderMaterialSummary();
    renderSourceRegistry();
    await loadDashboard();
    renderBuilder();
  } catch (error) {
    console.error(error);
    elements.searchWindowNote.textContent = `Could not save source list yet: ${String(error.message || error)}`;
  }
}

async function openSiteFixShell() {
  const sourceId = elements.siteFixSourceSelect.value;
  if (!sourceId) {
    elements.siteFixStatusNote.textContent = "Pick a source entry first.";
    return;
  }

  state.siteFix.loading = true;
  renderSiteFixShell();
  elements.siteFixStatusNote.textContent = "Opening source-specific scope details...";

  try {
    const response = await fetch("/api/source-fix/open", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ source_id: sourceId }),
    });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || `Opening site-fix shell failed with ${response.status}`);
    }

    state.siteFix.currentShell = payload;
    state.siteFix.proposal = null;
    state.siteFix.applyPreview = null;
    elements.siteFixIssueSummary.value = extractSection(payload.existing_note, "Issue Summary");
    elements.siteFixReproductionNotes.value = extractSection(payload.existing_note, "Reproduction Notes");
    elements.siteFixPatchNotes.value = extractSection(payload.existing_note, "Patch Notes");
    elements.siteFixStatusNote.textContent = payload.existing_note.trim()
      ? "Loaded the existing source-specific brief. Update it if the site drifted again."
      : "Source shell opened. You can now capture a scoped note for this source only.";
    renderSiteFixShell();
  } catch (error) {
    console.error(error);
    elements.siteFixStatusNote.textContent = `Could not open source shell: ${String(error.message || error)}`;
  } finally {
    state.siteFix.loading = false;
    renderSiteFixShell();
  }
}

async function proposeSiteFixShell() {
  const sourceId = elements.siteFixSourceSelect.value;
  if (!sourceId) {
    elements.siteFixStatusNote.textContent = "Pick a source entry first.";
    return;
  }

  if (!state.siteFix.currentShell) {
    elements.siteFixStatusNote.textContent = "Open the source shell first so the proposal stays scoped properly.";
    return;
  }

  state.siteFix.proposing = true;
  renderSiteFixShell();
  elements.siteFixStatusNote.textContent = "Drafting a review-first scoped proposal for this adapter...";

  try {
    const response = await fetch("/api/source-fix/propose", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        source_id: sourceId,
        issue_summary: elements.siteFixIssueSummary.value.trim(),
        reproduction_notes: elements.siteFixReproductionNotes.value.trim(),
        patch_notes: elements.siteFixPatchNotes.value.trim(),
      }),
    });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || `Drafting scoped proposal failed with ${response.status}`);
    }

    state.siteFix.proposal = payload;
    state.siteFix.applyPreview = null;
    elements.siteFixStatusNote.textContent =
      "Scoped proposal drafted. Review it carefully before copying any of it into the local source brief.";
    renderSiteFixShell();
  } catch (error) {
    console.error(error);
    elements.siteFixStatusNote.textContent = `Could not draft scoped proposal: ${String(error.message || error)}`;
  } finally {
    state.siteFix.proposing = false;
    renderSiteFixShell();
  }
}

async function saveSiteFixProposalSnapshot() {
  const sourceId = elements.siteFixSourceSelect.value;
  const proposal = state.siteFix.proposal;
  if (!sourceId || !proposal) {
    elements.siteFixStatusNote.textContent = "Draft a scoped proposal first.";
    return;
  }

  elements.siteFixStatusNote.textContent = "Saving scoped proposal snapshot...";

  try {
    const response = await fetch("/api/source-fix/proposal-save", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        source_id: sourceId,
        proposal_title: proposal.proposal_title,
        confidence_label: proposal.confidence_label,
        analysis_points: proposal.analysis_points,
        proposed_patch: proposal.proposed_patch,
        review_checklist: proposal.review_checklist,
      }),
    });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || `Saving scoped proposal failed with ${response.status}`);
    }

    elements.siteFixStatusNote.textContent = payload.notes.join(" ");
    await loadDashboard();
    state.siteFix.selectedSourceId = sourceId;
    await openSiteFixShell();
    state.siteFix.proposal = proposal;
    state.siteFix.applyPreview = null;
    renderSiteFixShell();
  } catch (error) {
    console.error(error);
    elements.siteFixStatusNote.textContent = `Could not save scoped proposal: ${String(error.message || error)}`;
  }
}

async function previewSiteFixApply() {
  const sourceId = elements.siteFixSourceSelect.value;
  if (!sourceId) {
    elements.siteFixStatusNote.textContent = "Pick a source entry first.";
    return;
  }

  if (!state.siteFix.currentShell) {
    elements.siteFixStatusNote.textContent = "Open the source shell first so the review stays anchored to one source.";
    return;
  }

  if (!state.siteFix.proposal && !siteFixHasPatchDraft()) {
    elements.siteFixStatusNote.textContent =
      "Draft a scoped proposal first, or paste selector/profile details into Patch notes before reviewing.";
    return;
  }

  state.siteFix.previewingApply = true;
  renderSiteFixShell();
  elements.siteFixStatusNote.textContent = "Building a review-first adapter patch preview...";

  try {
    const response = await fetch("/api/source-fix/apply-preview", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        source_id: sourceId,
        issue_summary: elements.siteFixIssueSummary.value.trim(),
        reproduction_notes: elements.siteFixReproductionNotes.value.trim(),
        patch_notes: elements.siteFixPatchNotes.value.trim(),
      }),
    });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || `Preparing adapter apply review failed with ${response.status}`);
    }

    state.siteFix.applyPreview = payload;
    elements.siteFixStatusNote.textContent =
      "Adapter patch review generated. Read the diff and backup path before applying anything.";
  } catch (error) {
    console.error(error);
    elements.siteFixStatusNote.textContent = `Could not build adapter patch review: ${String(error.message || error)}`;
  } finally {
    state.siteFix.previewingApply = false;
    renderSiteFixShell();
  }
}

async function applySiteFix() {
  const sourceId = elements.siteFixSourceSelect.value;
  if (!sourceId) {
    elements.siteFixStatusNote.textContent = "Pick a source entry first.";
    return;
  }

  if (!state.siteFix.applyPreview) {
    elements.siteFixStatusNote.textContent = "Generate the adapter patch review first.";
    return;
  }

  state.siteFix.applying = true;
  renderSiteFixShell();
  elements.siteFixStatusNote.textContent = "Applying the scoped adapter patch and writing a backup first...";

  try {
    const response = await fetch("/api/source-fix/apply", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        source_id: sourceId,
        issue_summary: elements.siteFixIssueSummary.value.trim(),
        reproduction_notes: elements.siteFixReproductionNotes.value.trim(),
        patch_notes: elements.siteFixPatchNotes.value.trim(),
      }),
    });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || `Applying adapter patch failed with ${response.status}`);
    }

    state.siteFix.applyPreview = null;
    state.siteFix.proposal = null;
    elements.siteFixStatusNote.textContent = payload.notes.join(" ");
    await loadDashboard();
    state.siteFix.selectedSourceId = sourceId;
    await openSiteFixShell();
  } catch (error) {
    console.error(error);
    elements.siteFixStatusNote.textContent = `Could not apply adapter patch: ${String(error.message || error)}`;
  } finally {
    state.siteFix.applying = false;
    renderSiteFixShell();
  }
}

async function saveSiteFixShell() {
  const sourceId = elements.siteFixSourceSelect.value;
  if (!sourceId) {
    elements.siteFixStatusNote.textContent = "Pick a source entry first.";
    return;
  }

  state.siteFix.saving = true;
  renderSiteFixShell();
  elements.siteFixStatusNote.textContent = "Saving local site-fix brief...";

  try {
    const response = await fetch("/api/source-fix/save", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        source_id: sourceId,
        issue_summary: elements.siteFixIssueSummary.value.trim(),
        reproduction_notes: elements.siteFixReproductionNotes.value.trim(),
        patch_notes: elements.siteFixPatchNotes.value.trim(),
      }),
    });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || `Saving site-fix shell failed with ${response.status}`);
    }

    elements.siteFixStatusNote.textContent = payload.notes.join(" ");
    await loadDashboard();
    state.siteFix.selectedSourceId = sourceId;
    await openSiteFixShell();
  } catch (error) {
    console.error(error);
    elements.siteFixStatusNote.textContent = `Could not save site-fix brief: ${String(error.message || error)}`;
  } finally {
    state.siteFix.saving = false;
    renderSiteFixShell();
  }
}

async function addCustomSource() {
  const name = elements.customSourceNameInput.value.trim();
  const baseUrl = elements.customSourceUrlInput.value.trim();
  const adapterKind = elements.customSourceAdapterSelect.value;
  const mediaKind = elements.customSourceMediaSelect.value;

  if (!name || !baseUrl) {
    elements.searchWindowNote.textContent = "Give the custom source a name and base URL first.";
    return;
  }

  state.sources.push({
    id: slugify(name),
    name,
    base_url: baseUrl,
    adapter_kind: adapterKind,
    media_kind: mediaKind,
    enabled: true,
    user_added: true,
    crawl_delay_ms: 1800,
    pages_per_batch: 3,
    respect_robots_txt: true,
    notes:
      adapterKind === "generic_gallery_html"
        ? "Custom source saved. Generic gallery URLs can use {query} and {page}; blank browse mode scans the source URL/page window without adding q."
        : "Custom source saved locally.",
  });

  elements.customSourceNameInput.value = "";
  elements.customSourceUrlInput.value = "";
  renderSourceRegistry();
  await persistSources();
}

async function runPreviewSearch() {
  const query = elements.webSearchInput.value.trim();
  const previousQuery = state.search.query;
  state.search.query = query;
  if (query !== previousQuery) {
    state.search.selectedKeys.clear();
    state.search.selectedItems.clear();
    elements.curationStatusNote.textContent = "Select preview items, then let Chatty-lora do the download and naming grunt work.";
    if (!elements.datasetNameInput.value.trim() && query) {
      elements.datasetNameInput.value = slugify(query);
    }
  }
  if (state.search.abortController) {
    state.search.abortController.abort();
  }
  const controller = new AbortController();
  state.search.abortController = controller;
  state.search.loading = true;
  updateSearchControls();
  renderSearchMeta();
  elements.searchWindowNote.textContent = query
    ? `Searching selected sources for "${query}".`
    : "Browsing selected sources from their first available media pages.";

  try {
    const selectedSourceIds = state.sources.filter((source) => source.enabled).map((source) => source.id);
    const response = await fetch("/api/search/preview", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      signal: controller.signal,
      body: JSON.stringify({
        query,
        selected_source_ids: selectedSourceIds,
        media_kinds: selectedSearchMediaKinds(),
        batch_index: state.search.batchIndex,
      }),
    });
    if (!response.ok) {
      let errorMessage = `Search preview failed with ${response.status}`;
      try {
        const payload = await response.json();
        if (payload?.error) {
          errorMessage = payload.error;
        }
      } catch (_error) {
        // Keep the generic status message if the response body is not JSON.
      }
      throw new Error(errorMessage);
    }

    state.search.preview = await response.json();
    renderSearchMeta();
    renderPreviewResults();
  } catch (error) {
    if (error.name === "AbortError") {
      elements.searchMetaPanel.innerHTML = `<p>Search cancelled.</p>`;
      elements.searchPreviewResults.innerHTML = `<div class="empty-state">Search cancelled. The previous results were left alone if they had already loaded.</div>`;
      elements.searchWindowNote.textContent = "Search cancelled.";
      return;
    }
    console.error(error);
    state.search.preview = null;
    elements.searchMetaPanel.innerHTML = `<p>Search failed: ${escapeHtml(String(error.message || error))}</p>`;
    elements.searchPreviewResults.innerHTML = `<div class="empty-state">Search preview failed. Check source status, network access, or try fewer sources.</div>`;
  } finally {
    if (state.search.abortController === controller) {
      state.search.abortController = null;
    }
    state.search.loading = false;
    updateSearchControls();
  }
}

function summaryCard(title, value, detail) {
  return `
    <article class="summary-card">
      <p class="summary-title">${escapeHtml(title)}</p>
      <strong>${escapeHtml(value)}</strong>
      <span>${escapeHtml(detail)}</span>
    </article>
  `;
}

function datasetPreflightClass(status) {
  if (status === "ok") {
    return "ok-badge";
  }
  if (status === "blocked" || status === "caution") {
    return "warm-badge";
  }
  return "muted-badge";
}

function formatDuration(seconds) {
  const value = Number(seconds);
  if (!Number.isFinite(value) || value <= 0) {
    return "unknown";
  }
  if (value < 60) {
    return `${value.toFixed(value >= 10 ? 0 : 1)}s`;
  }
  const minutes = Math.floor(value / 60);
  const remainingSeconds = Math.round(value % 60);
  return `${minutes}m ${remainingSeconds}s`;
}

function runtimeCard(title, ready) {
  return `
    <article class="summary-card">
      <p class="summary-title">${escapeHtml(title)}</p>
      <strong>${ready ? "Ready" : "Missing"}</strong>
      <span>${ready ? "Detected locally" : "Needs attention"}</span>
    </article>
  `;
}

function formatBytes(bytes) {
  const value = Number(bytes || 0);
  if (value <= 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  let amount = value;
  let unitIndex = 0;
  while (amount >= 1024 && unitIndex < units.length - 1) {
    amount /= 1024;
    unitIndex += 1;
  }
  return `${amount.toFixed(amount >= 10 || unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
}

function formatUnixSeconds(value) {
  const seconds = Number(value || 0);
  if (!Number.isFinite(seconds) || seconds <= 0) {
    return "unknown time";
  }

  return new Date(seconds * 1000).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

function setDashboardLoading(loading) {
  state.dashboardLoading = loading;
  elements.refreshButton.disabled = loading;
  elements.refreshButton.textContent = loading ? "Refreshing..." : "Refresh scan";
  updateSearchControls();
}

async function createDatasetFromSelection() {
  const datasetName = elements.datasetNameInput.value.trim();
  if (!datasetName) {
    elements.curationStatusNote.textContent = "Give the dataset folder a name first.";
    return;
  }

  const selectedItems = Array.from(state.search.selectedItems.values());
  if (!selectedItems.length) {
    elements.curationStatusNote.textContent = "Select at least one preview item first.";
    return;
  }

  state.search.curating = true;
  elements.curateSelectionButton.disabled = true;
  elements.curateSelectionButton.textContent = "Curating...";
  elements.curationStatusNote.textContent = `Downloading and curating ${selectedItems.length} selected item${selectedItems.length === 1 ? "" : "s"}...`;

  try {
    const response = await fetch("/api/datasets/create", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        dataset_name: datasetName,
        selected_items: selectedItems,
      }),
    });

    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || `Dataset curation failed with ${response.status}`);
    }

    state.search.selectedKeys.clear();
    state.search.selectedItems.clear();
    state.builder.selectedDatasetSlug = payload.dataset_slug;
    elements.curationStatusNote.textContent = payload.notes.join(" ");
    await loadDashboard();
    state.page = "builder";
    renderPage();
    renderSearchMeta();
    renderSelectionTray();
    renderPreviewResults();
  } catch (error) {
    console.error(error);
    elements.curationStatusNote.textContent = `Could not curate dataset yet: ${String(error.message || error)}`;
  } finally {
    state.search.curating = false;
    elements.curateSelectionButton.disabled = false;
    elements.curateSelectionButton.textContent = "Curate selected into dataset";
    renderSearchMeta();
  }
}

async function importLocalDataset() {
  const sourceFolder = elements.localImportSourceSelect.value;
  const datasetName = elements.localImportDatasetNameInput.value.trim();
  if (!sourceFolder) {
    elements.localImportStatusNote.textContent = "Choose a folder from inputs/ first.";
    return;
  }
  if (!datasetName) {
    elements.localImportStatusNote.textContent = "Give the cleaned dataset a name first.";
    return;
  }

  state.localImport.importing = true;
  renderLocalImportControls();
  elements.localImportStatusNote.textContent =
    `Cleaning "${sourceFolder}" into a new dataset. Original files will be left alone.`;
  let finalStatusMessage = "";

  try {
    const response = await fetch("/api/datasets/import-local", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        source_folder: sourceFolder,
        dataset_name: datasetName,
      }),
    });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || `Local folder cleanup failed with ${response.status}`);
    }

    state.builder.selectedDatasetSlug = payload.dataset_slug;
    elements.localImportDatasetNameInput.value = "";
    finalStatusMessage = payload.notes.join(" ");
    await loadDashboard();
    state.page = "builder";
    renderPage();
  } catch (error) {
    console.error(error);
    elements.localImportStatusNote.textContent = `Could not clean local folder yet: ${String(error.message || error)}`;
  } finally {
    state.localImport.importing = false;
    renderLocalImportControls();
    if (finalStatusMessage) {
      elements.localImportStatusNote.textContent = finalStatusMessage;
    }
  }
}

async function prepareBuilderProject() {
  const datasetSlug = elements.datasetSelect.value;
  if (!datasetSlug) {
    elements.prepareProjectNote.textContent = "Pick a curated dataset before saving a training plan.";
    return;
  }

  if (!elements.baseModelSelect.value) {
    elements.prepareProjectNote.textContent = "Choose a base model first.";
    return;
  }

  if (!elements.trainingBackendSelect.value) {
    elements.prepareProjectNote.textContent = "Choose a training backend target first.";
    return;
  }

  const payload = {
    project_name: elements.projectNameInput.value.trim(),
    dataset_slug: datasetSlug,
    base_model: elements.baseModelSelect.value,
    training_backend_id: elements.trainingBackendSelect.value,
    trigger_phrase: elements.triggerPhraseInput.value.trim(),
    concept_summary: elements.conceptSummaryInput.value.trim(),
    concept_type: elements.conceptTypeSelect.value,
    training_preset: elements.trainingPresetSelect.value,
    caption_strategy: elements.captionStrategySelect.value,
    rank: Number(elements.rankInput.value || 0),
    repeats: Number(elements.repeatsInput.value || 0),
    epochs: Number(elements.epochsInput.value || 0),
    resolution: Number(elements.resolutionSelect.value || 0),
    batch_size: Number(elements.batchSizeInput.value || 0),
    learning_rate: Number(elements.learningRateInput.value || 0),
    validation_split_percent: Number(elements.validationSplitInput.value || 0),
  };

  if (!payload.project_name) {
    elements.prepareProjectNote.textContent = "Give the LoRA project a name first.";
    return;
  }

  elements.prepareProjectButton.disabled = true;
  elements.prepareProjectButton.textContent = "Preparing...";
  elements.prepareProjectNote.textContent = "Saving a reusable local training plan...";

  try {
    const response = await fetch("/api/builder/prepare", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    const result = await response.json();
    if (!response.ok) {
      throw new Error(result.error || `Builder prepare failed with ${response.status}`);
    }

    elements.prepareProjectNote.textContent = result.notes.join(" ");
    await loadDashboard();
    const savedProject = getPreparedProject(result.project_slug);
    if (savedProject) {
      state.builder.loadedProjectSlug = savedProject.slug;
      state.builder.loadedProjectName = savedProject.project_name;
      state.builder.loadedSnapshot = projectToBuilderSnapshot(savedProject);
      state.builder.draftMode = "";
      elements.prepareProjectNote.textContent = `${result.notes.join(" ")} Saved as "${savedProject.project_name}" and loaded as the current editor baseline.`;
      renderBuilder();
    }
  } catch (error) {
    console.error(error);
    elements.prepareProjectNote.textContent = `Could not save the training plan yet: ${String(error.message || error)}`;
  } finally {
    elements.prepareProjectButton.disabled = false;
    updatePrepareProjectButtonLabel();
  }
}

async function startTrainingRun(projectSlug, mode = "full") {
  if (!projectSlug || state.training.busy) {
    return;
  }

  state.training.busy = true;
  renderBuilder();

  try {
    const response = await fetch("/api/training/run", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        project_slug: projectSlug,
        mode,
      }),
    });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || `Training runner failed with ${response.status}`);
    }
    state.training.status = payload;
    ensureTrainingPolling();
  } catch (error) {
    console.error(error);
    state.training.status = {
      state: "failed",
      project_slug: projectSlug,
      message: String(error.message || error),
      stages: [],
      logs: [],
      output_files: [],
    };
  } finally {
    state.training.busy = false;
    renderBuilder();
  }
}

async function stopTrainingRun() {
  if (state.training.busy) {
    return;
  }

  state.training.busy = true;
  renderBuilder();

  try {
    const response = await fetch("/api/training/stop", { method: "POST" });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || `Training stop failed with ${response.status}`);
    }
    state.training.status = payload;
    ensureTrainingPolling();
  } catch (error) {
    console.error(error);
  } finally {
    state.training.busy = false;
    renderBuilder();
  }
}

async function askHelper() {
  const question = elements.helperInput.value.trim();
  if (!question || state.helper.loading) {
    return;
  }

  state.helper.loading = true;
  state.helper.messages.push({ role: "user", content: question });
  elements.helperInput.value = "";
  renderHelper();

  try {
    const response = await fetch("/api/helper/query", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(buildHelperPayload(question)),
    });
    if (!response.ok) {
      const errorPayload = await response.json().catch(() => ({}));
      throw new Error(errorPayload.error || `Helper request failed with ${response.status}`);
    }

    const payload = await response.json();
    state.helper.messages.push({
      role: "assistant",
      content: `${payload.context_title} ${payload.answer}`,
      suggestions: payload.suggestions,
    });
  } catch (error) {
    console.error(error);
    state.helper.messages.push({
      role: "assistant",
      content: `I hit a snag while answering that. ${String(error.message || error)}`,
      suggestions: [
        "Try the question again once the dashboard has finished refreshing.",
        "If this keeps failing, we can inspect the helper endpoint next.",
      ],
    });
  } finally {
    state.helper.loading = false;
    renderHelper();
  }
}

function adapterReady(adapterKind) {
  return ["openverse_images", "openverse_audio", "wikimedia_commons", "generic_gallery_html"].includes(adapterKind);
}

function buildHelperPayload(question) {
  return {
    page: state.page,
    question,
    materials: state.page === "materials" ? {
      search_query: state.search.query,
      media_kinds: selectedSearchMediaKinds(),
      enabled_source_names: state.sources.filter((source) => source.enabled).map((source) => source.name),
      selected_preview_count: state.search.selectedItems.size,
      preview_batch_loaded: Boolean(state.search.preview),
      input_file_count: state.dashboard?.materials?.input_summary?.total || 0,
      output_file_count: state.dashboard?.materials?.output_summary?.total || 0,
    } : null,
    builder: state.page === "builder" ? {
      selected_dataset_slug: state.builder.selectedDatasetSlug || null,
      selected_dataset_file_count: getSelectedDataset()?.total_files ?? null,
      selected_dataset_image_count: getSelectedDataset()?.images ?? null,
      selected_dataset_audio_count: getSelectedDataset()?.audio ?? null,
      selected_dataset_video_count: getSelectedDataset()?.video ?? null,
      prepared_project_count: state.dashboard?.builder?.prepared_projects?.length || 0,
      project_name: elements.projectNameInput.value.trim(),
      base_model: elements.baseModelSelect.value,
      training_backend_id: elements.trainingBackendSelect.value,
      concept_type: elements.conceptTypeSelect.value,
      training_preset: elements.trainingPresetSelect.value,
      caption_strategy: elements.captionStrategySelect.value,
      rank: toMaybeNumber(elements.rankInput.value),
      repeats: toMaybeNumber(elements.repeatsInput.value),
      epochs: toMaybeNumber(elements.epochsInput.value),
      resolution: toMaybeNumber(elements.resolutionSelect.value),
      batch_size: toMaybeNumber(elements.batchSizeInput.value),
      learning_rate: toMaybeFloat(elements.learningRateInput.value),
      validation_split_percent: toMaybeNumber(elements.validationSplitInput.value),
    } : null,
  };
}

function helperContextTitle() {
  if (state.page === "builder") {
    const dataset = getSelectedDataset();
    if (dataset) {
      return `Builder is focused on dataset "${dataset.display_name}" with ${dataset.total_files} file${dataset.total_files === 1 ? "" : "s"}.`;
    }
    return "Builder is waiting for a curated dataset before it can give project-specific guidance.";
  }

  const enabledSources = state.sources.filter((source) => source.enabled);
  if (state.search.query) {
    return `Materials is searching “${state.search.query}” across ${enabledSources.length} enabled source${enabledSources.length === 1 ? "" : "s"}.`;
  }
  return `Materials currently has ${enabledSources.length} enabled source${enabledSources.length === 1 ? "" : "s"} and is ready for a respectful preview search.`;
}

function defaultHelperSuggestions() {
  return state.page === "materials"
    ? [
        "Try one or two sources first so it is easier to tell where a bad result is coming from.",
        "Search with direct noun phrases before you get fancy.",
      ]
    : [
        "Get the dataset coherent before you start squeezing the hyperparameters.",
        "A saved Wan plan gives you visible WSL handoff commands before anything launches.",
      ];
}

function extractSection(markdown, heading) {
  if (!markdown) {
    return "";
  }
  const escaped = heading.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`## ${escaped}\\r?\\n([\\s\\S]*?)(?=\\r?\\n## |$)`, "i");
  const match = markdown.match(pattern);
  return match ? match[1].trim() : "";
}

function renderProposalHistory(items) {
  if (!items || !items.length) {
    return `
      <div class="proposal-history-block">
        <h4>Saved proposal history</h4>
        <p class="muted-copy">No saved proposal snapshots for this source yet.</p>
      </div>
    `;
  }

  return `
    <div class="proposal-history-block">
      <h4>Saved proposal history</h4>
      <div class="proposal-history-list">
        ${items
          .map(
            (item) => `
              <article class="proposal-history-item">
                <div>
                  <strong>${escapeHtml(item.title)}</strong>
                  <p>${escapeHtml(item.relative_path)}</p>
                </div>
                <button class="secondary-button" type="button" data-copy-proposal-path="${escapeAttribute(item.relative_path)}">Copy path</button>
              </article>
            `,
          )
          .join("")}
      </div>
    </div>
  `;
}

function renderApplyHistory(items) {
  if (!items || !items.length) {
    return `
      <div class="proposal-history-block">
        <h4>Applied patch history</h4>
        <p class="muted-copy">No applied adapter changes have been recorded for this source yet.</p>
      </div>
    `;
  }

  return `
    <div class="proposal-history-block">
      <h4>Applied patch history</h4>
      <div class="proposal-history-list">
        ${items
          .map(
            (item) => `
              <article class="proposal-history-item">
                <div>
                  <strong>${escapeHtml(item.title)}</strong>
                  <p>${escapeHtml(item.relative_path)}</p>
                </div>
                <button class="secondary-button" type="button" data-copy-apply-path="${escapeAttribute(item.relative_path)}">Copy path</button>
              </article>
            `,
          )
          .join("")}
      </div>
    </div>
  `;
}

function renderApplyPreview(preview) {
  if (!preview) {
    return `
      <div class="apply-preview-block">
        <h4>Apply proposed fix</h4>
        <p class="muted-copy">Review the proposed fix first. Chatty-lora will show the scoped diff, the planned backup path, and the exact file or source profile it wants to touch before the final apply button is enabled.</p>
      </div>
    `;
  }

  return `
    <div class="apply-preview-block">
      <div class="section-heading-row">
        <div>
          <h4>${escapeHtml(preview.review_title)}</h4>
          <p class="muted-copy">Target: <strong>${escapeHtml(preview.adapter_file_path)}</strong></p>
        </div>
        <button class="primary-button" type="button" id="siteFixApplyButton">${state.siteFix.applying ? "Applying..." : "Apply proposed fix"}</button>
      </div>
      <p><strong>Backup path:</strong> ${escapeHtml(preview.backup_relative_path)}</p>
      <ul class="bullet-list">
        ${preview.apply_notes.map((note) => `<li>${escapeHtml(note)}</li>`).join("")}
      </ul>
      <div class="apply-preview-grid">
        <div>
          <h5>Before</h5>
          <pre class="proposal-code"><code>${escapeHtml(preview.before_excerpt)}</code></pre>
        </div>
        <div>
          <h5>After</h5>
          <pre class="proposal-code"><code>${escapeHtml(preview.after_excerpt)}</code></pre>
        </div>
      </div>
      <h5>Scoped diff summary</h5>
      <pre class="proposal-code"><code>${escapeHtml(preview.diff_lines.join("\n"))}</code></pre>
    </div>
  `;
}

async function copyTextToClipboard(value, successMessage) {
  try {
    await navigator.clipboard.writeText(value);
    elements.siteFixStatusNote.textContent = successMessage;
  } catch (error) {
    console.error(error);
    elements.siteFixStatusNote.textContent = "Could not copy that path automatically. You can still copy it manually from the panel.";
  }
}

async function copyTrainingCommand(value, label) {
  if (!value) {
    elements.prepareProjectNote.textContent = "No training command was available to copy.";
    return;
  }

  try {
    await navigator.clipboard.writeText(value);
    elements.prepareProjectNote.textContent = `Copied ${label}. Paste it into Windows PowerShell when you are ready.`;
  } catch (error) {
    console.error(error);
    elements.prepareProjectNote.textContent = "Could not copy automatically. You can still select the command text from the saved plan card.";
  }
}

async function copyTrainingOutputPath(value) {
  if (!value) {
    elements.prepareProjectNote.textContent = "No LoRA output path was available to copy.";
    return;
  }

  try {
    await navigator.clipboard.writeText(value);
    elements.prepareProjectNote.textContent = "Copied the LoRA output path.";
  } catch (error) {
    console.error(error);
    elements.prepareProjectNote.textContent = "Could not copy automatically. You can still copy the path from the saved output row.";
  }
}

async function openLocalPath(relativePath) {
  if (!relativePath) {
    elements.prepareProjectNote.textContent = "No local output path was available to open.";
    return;
  }

  try {
    const response = await fetch("/api/open-local-path", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ relative_path: relativePath }),
    });
    const payload = await response.json();
    if (!response.ok) {
      throw new Error(payload.error || `Open path failed with ${response.status}`);
    }
    elements.prepareProjectNote.textContent = `Opened ${payload.opened_path || relativePath} in Windows Explorer.`;
  } catch (error) {
    console.error(error);
    elements.prepareProjectNote.textContent = `Could not open that path automatically: ${String(error.message || error)}`;
  }
}

function lookupPreviewItem(key) {
  if (!state.search.preview) {
    return null;
  }
  for (const batch of state.search.preview.source_batches) {
    for (const page of batch.pages) {
      for (const item of page.items) {
        if (item.key === key) {
          return item;
        }
      }
    }
  }
  return null;
}

function slugify(value) {
  return String(value)
    .toLowerCase()
    .replaceAll(/[^a-z0-9]+/g, "-")
    .replaceAll(/^-+|-+$/g, "");
}

function toMaybeNumber(value) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function toMaybeFloat(value) {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function escapeAttribute(value) {
  return escapeHtml(value);
}
