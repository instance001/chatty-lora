"use strict";

(function attachHelperLaneModule() {
  const HELPER_LANE_EXPORT_KIND = "chatty-lora-helper-lanes";
  const HELPER_LANE_EXPORT_SCHEMA_VERSION = 2;
  const HELPER_LANE_TRANSFER_DRAFT_KEY = "chatty-lora.helperLaneTransferDraft.v1";
  const HELPER_LANE_TRANSFER_AGING_MS = 12 * 60 * 60 * 1000;
  const HELPER_LANE_TRANSFER_STALE_MS = 3 * 24 * 60 * 60 * 1000;

  function withContext(context) {
    const { state, elements, escapeHtml, escapeAttribute, renderHelper } = context;
    const providerRegistry = window.ChattyLoraHelperLaneProviders;

    function helperProviderCatalog() {
      return state.dashboard?.helper?.provider_catalog || null;
    }

    function normalizedImportPreviewMode(value) {
      return String(value || "").trim().toUpperCase() === "REPLACE" ? "REPLACE" : "MERGE";
    }

    function loadTransferDraft() {
      try {
        const raw = window.localStorage.getItem(HELPER_LANE_TRANSFER_DRAFT_KEY);
        if (!raw) {
          return null;
        }
        const parsed = JSON.parse(raw);
        return {
          transferText: String(parsed?.transfer_text || ""),
          importPreviewMode: normalizedImportPreviewMode(parsed?.import_preview_mode),
          savedAtUnixMs: Number(parsed?.saved_at_unix_ms || 0),
          baselineSelectedLaneId: String(parsed?.baseline_selected_lane_id || ""),
          baselineLaneFingerprint: String(parsed?.baseline_lane_fingerprint || ""),
          baselineLaneCount: Number(parsed?.baseline_lane_count || 0),
        };
      } catch (error) {
        console.error(error);
        return null;
      }
    }

    function currentLocalLaneBaseline() {
      const payload = collectHelperLaneDrafts();
      const lanes = Array.isArray(payload?.lanes) ? payload.lanes : [];
      const laneFingerprint = lanes
        .map((lane) => [
          String(lane?.id || "").trim(),
          importLaneFingerprint(lane),
        ].join("|"))
        .sort()
        .join("||");

      return {
        selectedLaneId: String(payload?.selected_lane_id || ""),
        laneFingerprint,
        laneCount: lanes.length,
      };
    }

    function saveTransferDraft() {
      try {
        const baseline = currentLocalLaneBaseline();
        window.localStorage.setItem(
          HELPER_LANE_TRANSFER_DRAFT_KEY,
          JSON.stringify({
            transfer_text: String(elements.helperLaneTransferInput.value || ""),
            import_preview_mode: normalizedImportPreviewMode(state.helper.importPreviewMode),
            saved_at_unix_ms: Date.now(),
            baseline_selected_lane_id: baseline.selectedLaneId,
            baseline_lane_fingerprint: baseline.laneFingerprint,
            baseline_lane_count: baseline.laneCount,
          }),
        );
      } catch (error) {
        console.error(error);
      }
    }

    function clearTransferDraft() {
      try {
        window.localStorage.removeItem(HELPER_LANE_TRANSFER_DRAFT_KEY);
      } catch (error) {
        console.error(error);
      }
      elements.helperLaneTransferInput.value = "";
      state.helper.importPreviewMode = "MERGE";
      elements.helperLaneStatusNote.textContent = "Cleared the saved transfer draft from this browser.";
      renderHelper();
    }

    function rebaseTransferDraft() {
      const transferText = String(elements.helperLaneTransferInput.value || "").trim();
      if (!transferText) {
        elements.helperLaneStatusNote.textContent = "Paste or export helper lane JSON before rebasing the saved draft baseline.";
        renderHelper();
        return;
      }
      saveTransferDraft();
      elements.helperLaneStatusNote.textContent =
        "Rebased the saved transfer draft against the current local helper lanes.";
      renderHelper();
    }

    function hydrateTransferDraft() {
      const draft = loadTransferDraft();
      if (!draft) {
        state.helper.importPreviewMode = normalizedImportPreviewMode(state.helper.importPreviewMode);
        return;
      }
      if (!String(elements.helperLaneTransferInput.value || "").trim() && draft.transferText) {
        elements.helperLaneTransferInput.value = draft.transferText;
      }
      state.helper.importPreviewMode = normalizedImportPreviewMode(draft.importPreviewMode);
    }

    function transferDraftSavedAtLabel() {
      const draft = loadTransferDraft();
      const savedAtUnixMs = Number(draft?.savedAtUnixMs || 0);
      if (!Number.isFinite(savedAtUnixMs) || savedAtUnixMs <= 0) {
        return "No saved transfer draft yet.";
      }
      return `Last saved draft: ${new Date(savedAtUnixMs).toLocaleString()}.`;
    }

    function transferDraftAgeState() {
      const draft = loadTransferDraft();
      const savedAtUnixMs = Number(draft?.savedAtUnixMs || 0);
      if (!Number.isFinite(savedAtUnixMs) || savedAtUnixMs <= 0) {
        return {
          kind: "none",
          note: "",
        };
      }

      const ageMs = Date.now() - savedAtUnixMs;
      if (ageMs >= HELPER_LANE_TRANSFER_STALE_MS) {
        return {
          kind: "stale",
          note: "Saved transfer draft looks stale. Re-check the diff before applying it.",
        };
      }
      if (ageMs >= HELPER_LANE_TRANSFER_AGING_MS) {
        return {
          kind: "aging",
          note: "Saved transfer draft is getting older. A quick re-check is a good idea.",
        };
      }
      return {
        kind: "fresh",
        note: "",
      };
    }

    function transferDraftDriftState() {
      const draft = loadTransferDraft();
      if (!draft || !draft.baselineLaneFingerprint) {
        return {
          kind: "unknown",
          note: "",
        };
      }

      const current = currentLocalLaneBaseline();
      const selectedChanged = draft.baselineSelectedLaneId !== current.selectedLaneId;
      const lanesChanged = draft.baselineLaneFingerprint !== current.laneFingerprint;
      const countChanged = Number(draft.baselineLaneCount || 0) !== current.laneCount;

      if (lanesChanged || selectedChanged || countChanged) {
        return {
          kind: "drifted",
          note: "Local helper lane config changed since this transfer draft was saved. Re-check the preview before applying it.",
        };
      }

      return {
        kind: "aligned",
        note: "",
      };
    }

    function cloneMetadata(value) {
      if (!value || typeof value !== "object") {
        return null;
      }
      try {
        return JSON.parse(JSON.stringify(value));
      } catch (_error) {
        return null;
      }
    }

    function laneProviderSnapshot(providerKind) {
      const providerMeta = providerRegistry.getProviderMeta(providerKind, helperProviderCatalog());
      return {
        kind: providerRegistry.normalizeProviderKind(providerKind),
        label: providerMeta.label,
        live_helper_supported: Boolean(providerMeta.liveHelperSupported),
        base_url_mode: providerMeta.baseUrlMode || "optional",
        api_key_mode: providerMeta.apiKeyMode || "recommended",
        default_base_url: providerMeta.suggestedBaseUrl || null,
        example_model: providerMeta.exampleModel || null,
      };
    }

    function exportLaneMetadata(lane) {
      const existing = cloneMetadata(lane?.metadata) || {};
      existing.provider_snapshot = laneProviderSnapshot(lane?.provider_kind);
      existing.export = {
        kind: HELPER_LANE_EXPORT_KIND,
        schema_version: HELPER_LANE_EXPORT_SCHEMA_VERSION,
      };
      return existing;
    }

    function normalizeImportedLane(inputLane) {
      if (!inputLane || typeof inputLane !== "object") {
        return null;
      }
      return {
        id: String(inputLane.id || "").trim(),
        label: String(inputLane.label || "").trim(),
        lane_mode: String(inputLane.lane_mode || "local").trim(),
        provider_kind: providerRegistry.normalizeProviderKind(inputLane.provider_kind),
        model_name: String(inputLane.model_name || "").trim(),
        base_url: inputLane.base_url == null ? null : String(inputLane.base_url).trim(),
        api_key: inputLane.api_key == null ? null : String(inputLane.api_key).trim(),
        metadata: cloneMetadata(inputLane.metadata),
        enabled: inputLane.enabled !== false,
      };
    }

    function countMetadataBearingLanes(lanes) {
      return (Array.isArray(lanes) ? lanes : []).filter((lane) => lane?.metadata && typeof lane.metadata === "object").length;
    }

    function normalizedMetadataFingerprint(metadata) {
      const cloned = cloneMetadata(metadata);
      if (!cloned) {
        return "";
      }
      try {
        return JSON.stringify(cloned);
      } catch (_error) {
        return "";
      }
    }

    function importLaneFingerprint(lane) {
      return [
        String(lane?.label || "").trim(),
        String(lane?.lane_mode || "").trim(),
        String(lane?.provider_kind || "").trim(),
        String(lane?.model_name || "").trim(),
        String(lane?.base_url || "").trim(),
        String(Boolean(lane?.enabled)),
        normalizedMetadataFingerprint(lane?.metadata),
      ].join("|");
    }

    function summarizeDiffCounts(beforeLanes, afterLanes) {
      const beforeMap = new Map((beforeLanes || []).map((lane) => [lane.id, lane]));
      const afterMap = new Map((afterLanes || []).map((lane) => [lane.id, lane]));
      let added = 0;
      let removed = 0;
      let changed = 0;
      let metadataUpdated = 0;

      for (const [laneId, afterLane] of afterMap.entries()) {
        const beforeLane = beforeMap.get(laneId);
        if (!beforeLane) {
          added += 1;
          continue;
        }
        const beforeFingerprint = importLaneFingerprint(beforeLane);
        const afterFingerprint = importLaneFingerprint(afterLane);
        if (beforeFingerprint !== afterFingerprint) {
          changed += 1;
        }
        if (normalizedMetadataFingerprint(beforeLane.metadata) !== normalizedMetadataFingerprint(afterLane.metadata)) {
          metadataUpdated += 1;
        }
      }

      for (const laneId of beforeMap.keys()) {
        if (!afterMap.has(laneId)) {
          removed += 1;
        }
      }

      return { added, removed, changed, metadataUpdated };
    }

    function renderDiffPreviewRows(label, diff) {
      return `
        <div class="helper-lane-import-diff">
          <div class="helper-lane-import-diff-row">
            <strong>${escapeHtml(label)}</strong>
            <span class="list-badge">${escapeHtml(`${diff.added} add / ${diff.changed} change / ${diff.removed} remove`)}</span>
          </div>
          <p class="muted-copy">Metadata updates preserved: ${escapeHtml(String(diff.metadataUpdated))}</p>
        </div>
      `;
    }

    function parsedTransferPayload(raw) {
      const trimmed = String(raw || "").trim();
      if (!trimmed) {
        return { ok: false, reason: "empty" };
      }

      let parsed;
      try {
        parsed = JSON.parse(trimmed);
      } catch (error) {
        return { ok: false, reason: "parse", error };
      }

      const schemaVersion = Number(parsed?.schema_version || 1);
      const exportKind = String(parsed?.export_kind || "");
      if (schemaVersion >= 2 && exportKind && exportKind !== HELPER_LANE_EXPORT_KIND) {
        return { ok: false, reason: "kind", schemaVersion, exportKind };
      }

      const lanes = Array.isArray(parsed?.lanes)
        ? parsed.lanes.map(normalizeImportedLane).filter(Boolean)
        : [];
      if (!lanes.length) {
        return { ok: false, reason: "lanes", schemaVersion, exportKind };
      }

      return {
        ok: true,
        parsed,
        schemaVersion,
        exportKind,
        lanes,
        selectedLaneId: String(parsed?.selected_lane_id || ""),
      };
    }

    function previewImportPayload(importMode) {
      const parsed = parsedTransferPayload(elements.helperLaneTransferInput.value);
      if (!parsed.ok) {
        return null;
      }

      const currentPayload = collectHelperLaneDrafts();
      const importPayload = {
        selected_lane_id: parsed.selectedLaneId,
        lanes: parsed.lanes,
      };
      const finalPayload = importMode === "REPLACE"
        ? buildReplaceImportPayload(importPayload)
        : buildMergeImportPayload(currentPayload, importPayload);
      const diff = summarizeDiffCounts(currentPayload.lanes, finalPayload.lanes);

      return {
        importMode,
        parsed,
        finalPayload,
        diff,
      };
    }

    function inspectTransferPayload(raw) {
      const trimmed = String(raw || "").trim();
      if (!trimmed) {
        return {
          kind: "empty",
          summary: "Paste or export helper lane JSON to inspect it.",
          details: [
            "Schema version, export kind, lane count, and metadata preservation estimates will appear here.",
          ],
          diffHtml: "",
        };
      }

      const parsedResult = parsedTransferPayload(trimmed);
      if (!parsedResult.ok && parsedResult.reason === "parse") {
        return {
          kind: "invalid",
          summary: "Transfer JSON is not valid yet.",
          details: [`Parse error: ${String(parsedResult.error?.message || parsedResult.error)}`],
          diffHtml: "",
        };
      }
      if (!parsedResult.ok && parsedResult.reason === "empty") {
        return {
          kind: "empty",
          summary: "Paste or export helper lane JSON to inspect it.",
          details: [
            "Schema version, export kind, lane count, and metadata preservation estimates will appear here.",
          ],
          diffHtml: "",
        };
      }
      if (!parsedResult.ok && parsedResult.reason === "kind") {
        return {
          kind: "warning",
          summary: "Transfer JSON parsed, but this export kind is not supported here.",
          details: [
            `Schema version: ${parsedResult.schemaVersion}.`,
            `Export kind: ${parsedResult.exportKind || "(missing)"}.`,
            `Use only exports with kind "${HELPER_LANE_EXPORT_KIND}" for schema v2+ imports here.`,
          ],
          diffHtml: "",
        };
      }
      if (!parsedResult.ok) {
        return {
          kind: "invalid",
          summary: "Transfer JSON parsed, but no lanes were found.",
          details: [
            `Schema version: ${parsedResult.schemaVersion || 1}.`,
            `Export kind: ${parsedResult.exportKind || "legacy-v1"}.`,
          ],
          diffHtml: "",
        };
      }

      const { schemaVersion, exportKind, lanes, selectedLaneId } = parsedResult;
      const metadataCount = countMetadataBearingLanes(lanes);
      const providerSnapshots = lanes.filter((lane) => lane?.metadata?.provider_snapshot).length;
      const selectedExists = lanes.some((lane) => lane.id === selectedLaneId);
      const currentPayload = collectHelperLaneDrafts();
      const mergePreview = buildMergeImportPayload(currentPayload, {
        selected_lane_id: selectedLaneId,
        lanes,
      });
      const replacePreview = buildReplaceImportPayload({
        selected_lane_id: selectedLaneId,
        lanes,
      });
      const mergeDiff = summarizeDiffCounts(currentPayload.lanes, mergePreview.lanes);
      const replaceDiff = summarizeDiffCounts(currentPayload.lanes, replacePreview.lanes);
      const activeMode = state.helper.importPreviewMode === "REPLACE" ? "REPLACE" : "MERGE";
      const activePreview = activeMode === "REPLACE" ? replaceDiff : mergeDiff;

      return {
        kind: "ready",
        summary: `Detected ${lanes.length} lane${lanes.length === 1 ? "" : "s"} ready for import inspection.`,
        details: [
          `Schema version: ${schemaVersion}.`,
          `Export kind: ${exportKind || "legacy-v1"}.`,
          `Selected lane id: ${selectedLaneId || "(none supplied)"}${selectedLaneId ? (selectedExists ? " (present)." : " (not present in lane list).") : "."}`,
          `Metadata-bearing lanes: ${metadataCount}/${lanes.length}.`,
          `Provider snapshot metadata found on ${providerSnapshots} lane${providerSnapshots === 1 ? "" : "s"}.`,
          `Selected apply mode: ${activeMode}.`,
          `Selected preview impact: ${activePreview.added} add / ${activePreview.changed} change / ${activePreview.removed} remove.`,
          "Import path can preserve staged metadata fields that are already present in this export.",
        ],
        diffHtml: `${renderDiffPreviewRows("MERGE preview", mergeDiff)}${renderDiffPreviewRows("REPLACE preview", replaceDiff)}`,
      };
    }

    function renderTransferInspection() {
      const inspection = inspectTransferPayload(elements.helperLaneTransferInput.value);
      const ageState = transferDraftAgeState();
      const driftState = transferDraftDriftState();
      elements.helperLaneTransferSavedAt.textContent = transferDraftSavedAtLabel();
      elements.helperLaneTransferSummary.textContent = inspection.summary;
      elements.helperLaneTransferInspector.dataset.transferKind = inspection.kind;
      elements.helperLaneTransferInspector.dataset.transferAge = ageState.kind;
      elements.helperLaneTransferInspector.dataset.transferDrift = driftState.kind;
      elements.helperLanePreviewMergeButton.disabled = inspection.kind !== "ready" || state.helper.importingLanes;
      elements.helperLanePreviewReplaceButton.disabled = inspection.kind !== "ready" || state.helper.importingLanes;
      elements.helperLaneApplyImportButton.disabled = inspection.kind !== "ready" || state.helper.importingLanes;
      elements.helperLaneRebaseTransferDraftButton.disabled =
        state.helper.importingLanes || !String(elements.helperLaneTransferInput.value || "").trim() || driftState.kind !== "drifted";
      elements.helperLaneClearTransferDraftButton.disabled = state.helper.importingLanes;
      state.helper.importPreviewMode = normalizedImportPreviewMode(state.helper.importPreviewMode);
      elements.helperLanePreviewMergeButton.textContent = state.helper.importPreviewMode === "MERGE" ? "Merge selected" : "Preview merge";
      elements.helperLanePreviewReplaceButton.textContent = state.helper.importPreviewMode === "REPLACE" ? "Replace selected" : "Preview replace";
      elements.helperLaneApplyImportButton.textContent = state.helper.importingLanes
        ? "Applying..."
        : `Apply ${state.helper.importPreviewMode === "REPLACE" ? "replace" : "merge"}`;
      elements.helperLaneRebaseTransferDraftButton.textContent = "Rebase draft baseline";
      elements.helperLaneClearTransferDraftButton.textContent = "Clear transfer draft";
      const ageNoteHtml = ageState.note
        ? `<p class="helper-lane-transfer-warning">${escapeHtml(ageState.note)}</p>`
        : "";
      const driftNoteHtml = driftState.note
        ? `<p class="helper-lane-transfer-warning">${escapeHtml(driftState.note)}</p>`
        : "";
      elements.helperLaneTransferDetails.innerHTML = ageNoteHtml + driftNoteHtml + inspection.details
        .map((line) => `<p class="muted-copy">${escapeHtml(line)}</p>`)
        .join("") + inspection.diffHtml;
    }

    function helperLaneRegistry() {
      return state.dashboard?.helper?.lane_registry || null;
    }

    function helperLaneEntries() {
      const registry = helperLaneRegistry();
      return Array.isArray(registry?.lanes) ? registry.lanes : [];
    }

    function getEditingHelperLane() {
      const lanes = helperLaneEntries();
      if (!lanes.length) {
        return null;
      }
      return lanes.find((lane) => lane.id === state.helper.editingLaneId) || lanes[0];
    }

    function formatUnixSeconds(value) {
      const numeric = Number(value || 0);
      if (!Number.isFinite(numeric) || numeric <= 0) {
        return "recently";
      }
      return new Date(numeric * 1000).toLocaleString();
    }

    function helperLaneSeverity(kind) {
      switch (kind) {
        case "failed":
          return 4;
        case "stale":
          return 3;
        case "needs-setup":
          return 2;
        case "verified":
          return 1;
        default:
          return 0;
      }
    }

    function compactHelperLaneStatus(lane) {
      const status = String(lane?.verification_status || "").toLowerCase();
      const mode = String(lane?.lane_mode || "local").toLowerCase();
      const verifiedAt = Number(lane?.last_verified_unix_seconds || 0);
      const ageMs = verifiedAt > 0 ? Date.now() - (verifiedAt * 1000) : Number.POSITIVE_INFINITY;
      const staleMs = 7 * 24 * 60 * 60 * 1000;

      if (mode === "local" || lane?.id === "local-rule-based") {
        return { kind: "verified", label: "offline ready" };
      }
      if (status === "failed") {
        return { kind: "failed", label: "failed" };
      }
      if (status === "ready" && Number.isFinite(ageMs) && ageMs <= staleMs) {
        return { kind: "verified", label: "verified" };
      }
      if (status === "ready" && verifiedAt > 0) {
        return { kind: "stale", label: "stale" };
      }
      return { kind: "needs-setup", label: "needs setup" };
    }

    function helperLaneBadgeClass(kind) {
      switch (kind) {
        case "verified":
          return "ok-badge";
        case "failed":
          return "warm-badge";
        case "stale":
          return "";
        default:
          return "muted-badge";
      }
    }

    function helperLanePassesFilter(status, filterValue) {
      switch (filterValue) {
        case "attention":
          return status.kind === "failed" || status.kind === "stale" || status.kind === "needs-setup";
        case "verified":
          return status.kind === "verified";
        default:
          return true;
      }
    }

    function compareHelperLaneRows(left, right, selectedLaneId, sortValue) {
      const activeDelta = Number(right.lane.id === selectedLaneId) - Number(left.lane.id === selectedLaneId);
      if (activeDelta !== 0) {
        return activeDelta;
      }

      if (sortValue === "recent-first") {
        const verifiedDelta = Number(right.lane.last_verified_unix_seconds || 0) - Number(left.lane.last_verified_unix_seconds || 0);
        if (verifiedDelta !== 0) {
          return verifiedDelta;
        }
      } else if (sortValue === "name") {
        const nameDelta = String(left.lane.label || "").localeCompare(String(right.lane.label || ""));
        if (nameDelta !== 0) {
          return nameDelta;
        }
      } else {
        const severityDelta = helperLaneSeverity(right.status.kind) - helperLaneSeverity(left.status.kind);
        if (severityDelta !== 0) {
          return severityDelta;
        }
        const recencyDelta = Number(right.lane.last_verified_unix_seconds || 0) - Number(left.lane.last_verified_unix_seconds || 0);
        if (recencyDelta !== 0) {
          return recencyDelta;
        }
      }

      return String(left.lane.label || "").localeCompare(String(right.lane.label || ""));
    }

    function populateHelperLaneEditor() {
      const editingLane = getEditingHelperLane();
      if (!editingLane) {
        return;
      }
      elements.helperLaneLabelInput.value = editingLane.label || "";
      elements.helperLaneModeSelect.value = editingLane.lane_mode || "local";
      elements.helperLaneProviderSelect.value = providerRegistry.normalizeProviderKind(editingLane.provider_kind);
      elements.helperLaneModelInput.value = editingLane.model_name || "";
      elements.helperLaneBaseUrlInput.value = editingLane.base_url || "";
      elements.helperLaneApiKeyInput.value = "";
    }

    function renderProviderOptions(selectedProviderKind) {
      const providers = providerRegistry.listProviders(helperProviderCatalog());
      elements.helperLaneProviderSelect.innerHTML = providers
        .map((provider) => `<option value="${escapeAttribute(provider.kind)}">${escapeHtml(provider.label)}</option>`)
        .join("");
      elements.helperLaneProviderSelect.value = providerRegistry.normalizeProviderKind(selectedProviderKind);
      if (!elements.helperLaneProviderSelect.value) {
        elements.helperLaneProviderSelect.value = "other";
      }
    }

    function helperLaneApiKeyStatusText(lane, fieldState) {
      if (!fieldState.apiKeyVisible || !lane || lane.id === "local-rule-based") {
        return "This lane does not need a remote API key.";
      }
      if (state.helper.clearStoredKeyRequested) {
        return "Saved key will be cleared when you save this lane.";
      }
      if (String(elements.helperLaneApiKeyInput.value || "").trim()) {
        return lane.has_api_key
          ? "A replacement API key is ready to save for this lane."
          : "A new API key is ready to save for this lane.";
      }
      return lane.has_api_key
        ? "A key is already stored for this lane. Leave blank to keep it, paste a new key to replace it, or clear it explicitly."
        : "No key is stored yet. Paste a key here if this cloud lane needs one.";
    }

    function applyProviderFieldState(lane) {
      const providerMeta = providerRegistry.getProviderMeta(
        lane?.provider_kind,
        helperProviderCatalog(),
      );
      const fieldState = providerRegistry.providerFieldState(
        lane?.provider_kind,
        elements.helperLaneModeSelect.value || lane?.lane_mode || "local",
        helperProviderCatalog(),
      );

      elements.helperLaneModelInput.placeholder = fieldState.modelPlaceholder;
      elements.helperLaneBaseUrlInput.placeholder = fieldState.baseUrlPlaceholder;
      elements.helperLaneBaseUrlInput.closest(".field-block")?.classList.toggle("is-hidden", !fieldState.baseUrlVisible);
      elements.helperLaneApiKeyInput.closest(".field-block")?.classList.toggle("is-hidden", !fieldState.apiKeyVisible);

      return { ...fieldState, providerMeta };
    }

    function seedExampleModel() {
      const editingLane = getEditingHelperLane();
      if (!editingLane) {
        return;
      }

      const providerMeta = providerRegistry.getProviderMeta(
        elements.helperLaneProviderSelect.value || editingLane.provider_kind,
        helperProviderCatalog(),
      );
      const exampleModel = String(providerMeta.exampleModel || "").trim();
      if (!exampleModel) {
        elements.helperLaneStatusNote.textContent = `No example model is published for ${providerMeta.label} yet.`;
        return;
      }

      elements.helperLaneModelInput.value = exampleModel;
      elements.helperLaneStatusNote.textContent = `Seeded "${exampleModel}" as the example model for ${providerMeta.label}.`;
      renderHelper();
    }

    function fillDefaultBaseUrl() {
      const editingLane = getEditingHelperLane();
      if (!editingLane) {
        return;
      }

      const providerMeta = providerRegistry.getProviderMeta(
        elements.helperLaneProviderSelect.value || editingLane.provider_kind,
        helperProviderCatalog(),
      );
      const defaultBaseUrl = String(providerMeta.suggestedBaseUrl || "").trim();
      if (!defaultBaseUrl) {
        elements.helperLaneStatusNote.textContent = `No default base URL is published for ${providerMeta.label}.`;
        return;
      }

      elements.helperLaneBaseUrlInput.value = defaultBaseUrl;
      elements.helperLaneStatusNote.textContent = `Filled the default base URL for ${providerMeta.label}.`;
      renderHelper();
    }

    function selectFallbackHelperLaneId(lanes) {
      const enabled = (lanes || []).find((lane) => lane.enabled !== false);
      return enabled?.id || lanes?.[0]?.id || "";
    }

    function nextHelperLaneId(prefix, lanes) {
      const existing = new Set((lanes || []).map((lane) => lane.id));
      let candidate = prefix;
      let index = 2;
      while (existing.has(candidate)) {
        candidate = `${prefix}-${index}`;
        index += 1;
      }
      return candidate;
    }

    function collectHelperLaneDrafts() {
      const registry = helperLaneRegistry();
      const lanes = helperLaneEntries().map((lane) => ({ ...lane }));
      const editingLane = lanes.find((lane) => lane.id === state.helper.editingLaneId);
      if (editingLane) {
        editingLane.label = elements.helperLaneLabelInput.value.trim() || editingLane.label;
        editingLane.lane_mode = elements.helperLaneModeSelect.value || editingLane.lane_mode;
        editingLane.provider_kind = providerRegistry.normalizeProviderKind(
          elements.helperLaneProviderSelect.value || editingLane.provider_kind,
        );
        editingLane.model_name = elements.helperLaneModelInput.value.trim();
        editingLane.base_url = elements.helperLaneBaseUrlInput.value.trim();
        const apiKeyDraft = elements.helperLaneApiKeyInput.value.trim();
        if (apiKeyDraft) {
          editingLane.api_key = apiKeyDraft;
          editingLane.has_api_key = true;
        } else if (state.helper.clearStoredKeyRequested) {
          editingLane.api_key = "__CLEAR__";
          editingLane.has_api_key = false;
        }
      }
      return {
        selected_lane_id: state.helper.editingLaneId || registry?.selected_lane_id || "",
        lanes: lanes.map((lane) => ({
          id: lane.id,
          label: lane.label,
          lane_mode: lane.lane_mode,
          provider_kind: providerRegistry.normalizeProviderKind(lane.provider_kind),
          model_name: lane.model_name,
          base_url: lane.base_url || null,
          api_key: lane.api_key || null,
          metadata: cloneMetadata(lane.metadata),
          enabled: lane.enabled !== false,
        })),
      };
    }

    function collectEditingHelperLaneDraft() {
      const payload = collectHelperLaneDrafts();
      return payload.lanes.find((lane) => lane.id === payload.selected_lane_id) || null;
    }

    function helperLaneKeySaveSummary(beforeLane, afterLane, apiKeyDraft, clearRequested) {
      if (!beforeLane && !afterLane) {
        return "";
      }
      const label = afterLane?.label || beforeLane?.label || "helper lane";
      if (clearRequested) {
        return `Saved helper lanes. Cleared the stored key for "${label}".`;
      }
      if (apiKeyDraft) {
        return beforeLane?.has_api_key
          ? `Saved helper lanes. Replaced the stored key for "${label}".`
          : `Saved helper lanes. Saved a new key for "${label}".`;
      }
      if (beforeLane?.has_api_key && afterLane?.has_api_key) {
        return `Saved helper lanes. Stored key for "${label}" was unchanged.`;
      }
      if (!beforeLane?.has_api_key && !afterLane?.has_api_key) {
        return `Saved helper lanes. "${label}" still has no stored key.`;
      }
      return `Saved helper lanes for "${label}".`;
    }

    function syncEditingLaneFromDashboard() {
      if (!state.helper.editingLaneId) {
        state.helper.editingLaneId = state.dashboard?.helper?.lane_registry?.selected_lane_id || "";
      }
    }

    function renderHelperLaneActivity(items) {
      if (!Array.isArray(items) || !items.length) {
        elements.helperLaneActivityList.innerHTML = `<p class="muted-copy">No helper lane activity recorded yet.</p>`;
        return;
      }

      elements.helperLaneActivityList.innerHTML = items
        .slice(0, 8)
        .map((item) => `
          <article class="helper-lane-activity-row">
            <div class="helper-lane-activity-top">
              <strong>${escapeHtml(item.lane_label || "Helper lane")}</strong>
              <span class="list-badge muted-badge">${escapeHtml(String(item.action || "event").replaceAll("_", " "))}</span>
            </div>
            <p class="muted-copy">${escapeHtml(item.note || "No activity note recorded.")}</p>
            <p class="muted-copy">${escapeHtml(formatUnixSeconds(item.unix_seconds))}</p>
          </article>
        `)
        .join("");
    }

    function renderManagedHelperLaneList(lanes, selectedLaneId) {
      if (!Array.isArray(lanes) || !lanes.length) {
        elements.helperLaneList.innerHTML = `<p class="muted-copy">No helper lanes yet.</p>`;
        return;
      }

      const prepared = lanes.map((lane) => ({
        lane,
        status: compactHelperLaneStatus(lane),
      }));
      const filtered = prepared.filter(({ status }) => helperLanePassesFilter(status, state.helper.laneScanFilter));
      const sorted = filtered.sort((left, right) => compareHelperLaneRows(left, right, selectedLaneId, state.helper.laneScanSort));

      if (!sorted.length) {
        elements.helperLaneList.innerHTML = `<p class="muted-copy">No lanes match the current filter.</p>`;
        return;
      }

      elements.helperLaneList.innerHTML = sorted
        .map((entry) => {
          const lane = entry.lane;
          const status = entry.status;
          const statusClass = helperLaneBadgeClass(status.kind);
          const verifiedLine = lane.last_verified_unix_seconds
            ? `Last verified ${escapeHtml(formatUnixSeconds(lane.last_verified_unix_seconds))}`
            : "No successful verification saved yet";
          const isBuiltinLocal = lane.id === "local-rule-based";
          const providerMeta = providerRegistry.getProviderMeta(lane.provider_kind, helperProviderCatalog());
          const capabilityBadgeClass = providerMeta.liveHelperSupported ? "ok-badge" : "muted-badge";
          const capabilityBadgeLabel = providerMeta.liveHelperSupported ? "wired" : "staged";
          const capabilityLine = lane.lane_mode === "cloud"
            ? (providerMeta.liveHelperSupported
              ? `${providerMeta.label} can answer through the live cloud helper adapter on this host.`
              : `${providerMeta.label} is staged here, but the live cloud helper adapter is not wired yet.`)
            : "This lane stays local and does not need a cloud provider adapter.";
          return `
            <article class="helper-lane-row ${lane.id === selectedLaneId ? "active" : ""}">
              <div class="helper-lane-row-top">
                <strong>${escapeHtml(lane.label)}</strong>
                <div class="helper-lane-row-badges">
                  ${lane.id === selectedLaneId ? `<span class="list-badge">active</span>` : ""}
                  <span class="list-badge ${statusClass}">${escapeHtml(status.label)}</span>
                  <span class="list-badge muted-badge">${escapeHtml(providerMeta.label)}</span>
                  <span class="list-badge ${capabilityBadgeClass}">${escapeHtml(capabilityBadgeLabel)}</span>
                </div>
              </div>
              <p class="muted-copy">${escapeHtml(lane.model_name || "(model not set yet)")} · ${escapeHtml(verifiedLine)}</p>
              <p class="helper-lane-provider-note">${escapeHtml(capabilityLine)}</p>
              <div class="helper-lane-row-actions">
                ${isBuiltinLocal
                  ? `<span class="muted-copy">Built-in lane cannot be disabled or deleted.</span>`
                  : `<button class="secondary-button" type="button" data-helper-lane-toggle="${escapeAttribute(lane.id)}">${lane.enabled === false ? "Enable" : "Disable"}</button>
                     <button class="secondary-button" type="button" data-helper-lane-delete="${escapeAttribute(lane.id)}">Delete</button>`}
              </div>
            </article>
          `;
        })
        .join("");

      for (const button of elements.helperLaneList.querySelectorAll("[data-helper-lane-toggle]")) {
        button.addEventListener("click", () => {
          const laneId = button.dataset.helperLaneToggle;
          if (!laneId) {
            return;
          }
          void toggleHelperLaneEnabled(laneId);
        });
      }

      for (const button of elements.helperLaneList.querySelectorAll("[data-helper-lane-delete]")) {
        button.addEventListener("click", () => {
          const laneId = button.dataset.helperLaneDelete;
          if (!laneId) {
            return;
          }
          void deleteHelperLane(laneId);
        });
      }
    }

    function renderManager() {
      const registry = helperLaneRegistry();
      const lanes = helperLaneEntries();
      const editingLane = getEditingHelperLane();

      if (!registry || !editingLane) {
        elements.helperLaneSummary.textContent = "Helper lanes will appear once the dashboard has loaded.";
        elements.helperLaneSelect.innerHTML = `<option value="">Loading helper lanes...</option>`;
        elements.helperLaneList.innerHTML = `<p class="muted-copy">Helper lanes will appear here once the dashboard has loaded.</p>`;
        elements.helperLaneActivityList.innerHTML = `<p class="muted-copy">Recent helper lane events will appear here.</p>`;
        elements.helperLaneSaveButton.disabled = true;
        elements.helperLaneNewButton.disabled = true;
        return;
      }

      state.helper.editingLaneId = editingLane.id;
      elements.helperLaneFilterSelect.value = state.helper.laneScanFilter;
      elements.helperLaneSortSelect.value = state.helper.laneScanSort;
      elements.helperLaneSelect.innerHTML = lanes
        .map((lane) => `<option value="${escapeAttribute(lane.id)}">${escapeHtml(`${lane.label} (${lane.lane_mode})`)}</option>`)
        .join("");
      elements.helperLaneSelect.value = editingLane.id;
      renderProviderOptions(editingLane.provider_kind);
      populateHelperLaneEditor();

      const isBuiltinLocal = editingLane.id === "local-rule-based";
      const fieldState = applyProviderFieldState(editingLane);
      elements.helperLaneLabelInput.disabled = isBuiltinLocal || state.helper.savingLaneConfig;
      elements.helperLaneModeSelect.disabled = isBuiltinLocal || state.helper.savingLaneConfig;
      elements.helperLaneProviderSelect.disabled = isBuiltinLocal || state.helper.savingLaneConfig;
      elements.helperLaneModelInput.disabled = isBuiltinLocal || state.helper.savingLaneConfig;
      elements.helperLaneBaseUrlInput.disabled = isBuiltinLocal || state.helper.savingLaneConfig || !fieldState.baseUrlVisible;
      elements.helperLaneApiKeyInput.disabled = isBuiltinLocal || state.helper.savingLaneConfig || !fieldState.apiKeyVisible;
      elements.helperLaneSeedModelButton.disabled = isBuiltinLocal
        || state.helper.savingLaneConfig
        || !fieldState.providerMeta.exampleModel;
      elements.helperLaneFillBaseUrlButton.disabled = isBuiltinLocal
        || state.helper.savingLaneConfig
        || !fieldState.baseUrlVisible
        || !fieldState.providerMeta.suggestedBaseUrl;
      elements.helperLaneSelect.disabled = state.helper.savingLaneConfig;
      elements.helperLaneNewButton.disabled = state.helper.savingLaneConfig || state.helper.verifyingLane || state.helper.importingLanes;
      elements.helperLaneVerifyButton.disabled = state.helper.savingLaneConfig || state.helper.verifyingLane || state.helper.importingLanes;
      elements.helperLaneClearKeyButton.disabled = isBuiltinLocal || state.helper.savingLaneConfig || state.helper.verifyingLane || state.helper.importingLanes || !fieldState.apiKeyVisible;
      elements.helperLaneExportButton.disabled = state.helper.savingLaneConfig || state.helper.verifyingLane || state.helper.importingLanes;
      elements.helperLaneImportButton.disabled = state.helper.savingLaneConfig || state.helper.verifyingLane || state.helper.importingLanes;
      elements.helperLaneSaveButton.disabled = state.helper.savingLaneConfig || state.helper.verifyingLane || state.helper.importingLanes;
      elements.helperLaneVerifyButton.textContent = state.helper.verifyingLane ? "Verifying..." : "Verify lane";
      elements.helperLaneClearKeyButton.textContent = state.helper.clearStoredKeyRequested ? "Key will clear on save" : "Clear saved key";
      elements.helperLaneImportButton.textContent = state.helper.importingLanes ? "Importing..." : "Import lanes";
      elements.helperLaneSaveButton.textContent = state.helper.savingLaneConfig ? "Saving..." : "Save helper lanes";
      elements.helperLaneSeedModelButton.textContent = fieldState.providerMeta.exampleModel ? "Seed example model" : "No example model";
      elements.helperLaneFillBaseUrlButton.textContent = fieldState.providerMeta.suggestedBaseUrl ? "Fill default base URL" : "No default base URL";
      elements.helperLaneApiKeyStatus.textContent = helperLaneApiKeyStatusText(editingLane, fieldState);

      const verifiedSuffix = editingLane.last_verified_unix_seconds
        ? ` Last verified ${formatUnixSeconds(editingLane.last_verified_unix_seconds)}.`
        : "";
      elements.helperLaneSummary.textContent = `${registry.selected_lane_label} is the active ${registry.selected_lane_mode} helper lane. ${editingLane.verification_note || ""}${verifiedSuffix}`;
      renderManagedHelperLaneList(lanes, registry.selected_lane_id);
      renderHelperLaneActivity(registry.recent_activity || []);
      renderTransferInspection();
      elements.helperLaneStatusNote.textContent = editingLane.id === registry.selected_lane_id
        ? (editingLane.verification_note || "Host-owned helper lane settings live in config/helper_lanes.json.")
        : `Editing "${editingLane.label}". ${fieldState.setupNote} Saving will make this the active helper lane.`;
    }

    function createDraftCloudHelperLane() {
      if (!state.dashboard?.helper?.lane_registry) {
        return;
      }
      const registry = state.dashboard.helper.lane_registry;
      const lanes = Array.isArray(registry.lanes) ? registry.lanes.slice() : [];
      const draftId = nextHelperLaneId("cloud-helper", lanes);
      lanes.push({
        id: draftId,
        label: "New cloud helper",
        lane_mode: "cloud",
        provider_kind: "openai",
        model_name: "",
        base_url: "",
        enabled: true,
        supports_remote_inference: false,
        has_api_key: false,
        verification_status: "needs-setup",
        verification_note: providerRegistry.cloudLaneDraftVerificationNote("openai", helperProviderCatalog()),
        last_verified_unix_seconds: null,
        source: "host-managed",
      });
      state.dashboard.helper.lane_registry = {
        ...registry,
        lanes,
      };
      state.helper.editingLaneId = draftId;
      renderHelper();
    }

    function requestClearStoredHelperLaneKey() {
      const editingLane = getEditingHelperLane();
      if (!editingLane || editingLane.id === "local-rule-based") {
        return;
      }
      state.helper.clearStoredKeyRequested = true;
      elements.helperLaneApiKeyInput.value = "";
      elements.helperLaneStatusNote.textContent = `Saved key for "${editingLane.label}" will be cleared when you save helper lanes.`;
      renderHelper();
    }

    async function persistHelperLanes() {
      const registry = helperLaneRegistry();
      if (!registry || state.helper.savingLaneConfig) {
        return;
      }

      const beforeLane = getEditingHelperLane();
      const apiKeyDraft = String(elements.helperLaneApiKeyInput.value || "").trim();
      const clearRequested = state.helper.clearStoredKeyRequested;

      state.helper.savingLaneConfig = true;
      renderHelper();

      try {
        const response = await fetch("/api/helper/lanes", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify(collectHelperLaneDrafts()),
        });
        const result = await response.json();
        if (!response.ok) {
          throw new Error(result.error || `Helper lanes request failed with ${response.status}`);
        }
        state.dashboard.helper.lane_registry = result;
        state.helper.editingLaneId = result.selected_lane_id || state.helper.editingLaneId;
        const afterLane = (result.lanes || []).find((lane) => lane.id === state.helper.editingLaneId) || null;
        state.helper.clearStoredKeyRequested = false;
        elements.helperLaneApiKeyInput.value = "";
        elements.helperLaneStatusNote.textContent = `${helperLaneKeySaveSummary(beforeLane, afterLane, apiKeyDraft, clearRequested)} Active lane: ${result.selected_lane_label}.`;
      } catch (error) {
        console.error(error);
        elements.helperLaneStatusNote.textContent = `Could not save helper lanes yet. ${String(error.message || error)}`;
      } finally {
        state.helper.savingLaneConfig = false;
        renderHelper();
      }
    }

    async function verifyHelperLane() {
      const lane = collectEditingHelperLaneDraft();
      if (!lane || state.helper.verifyingLane || state.helper.savingLaneConfig) {
        return;
      }

      const validation = providerRegistry.validateCloudLaneDraft(lane, helperProviderCatalog());
      if (!validation.ok) {
        elements.helperLaneStatusNote.textContent = validation.message;
        renderHelper();
        return;
      }

      state.helper.verifyingLane = true;
      elements.helperLaneStatusNote.textContent = `Verifying "${lane.label}"...`;
      renderHelper();

      try {
        const response = await fetch("/api/helper/lanes/verify", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({ lane }),
        });
        const result = await response.json();
        if (!response.ok) {
          throw new Error(result.error || `Helper lane verify failed with ${response.status}`);
        }

        const registry = helperLaneRegistry();
        const editingLane = getEditingHelperLane();
        if (editingLane) {
          editingLane.verification_status = result.verification_status;
          editingLane.verification_note = result.verification_note;
          editingLane.last_verified_unix_seconds = result.last_verified_unix_seconds || null;
          if (registry && editingLane.id === registry.selected_lane_id) {
            registry.selected_lane_label = result.lane_label || registry.selected_lane_label;
          }
        }
        elements.helperLaneStatusNote.textContent = result.verification_note;
      } catch (error) {
        console.error(error);
        elements.helperLaneStatusNote.textContent = `Could not verify this helper lane yet. ${String(error.message || error)}`;
      } finally {
        state.helper.verifyingLane = false;
        renderHelper();
      }
    }

    async function toggleHelperLaneEnabled(laneId) {
      const registry = helperLaneRegistry();
      if (!registry || state.helper.savingLaneConfig || state.helper.verifyingLane || state.helper.importingLanes) {
        return;
      }

      const lanes = helperLaneEntries().map((lane) => ({ ...lane }));
      const lane = lanes.find((entry) => entry.id === laneId);
      if (!lane || lane.id === "local-rule-based") {
        return;
      }

      lane.enabled = lane.enabled === false;
      const fallbackId = selectFallbackHelperLaneId(lanes);
      if (state.helper.editingLaneId === laneId && lane.enabled === false) {
        state.helper.editingLaneId = fallbackId;
      }

      state.dashboard.helper.lane_registry = {
        ...registry,
        lanes,
        selected_lane_id: registry.selected_lane_id === laneId && lane.enabled === false
          ? fallbackId
          : registry.selected_lane_id,
      };
      elements.helperLaneStatusNote.textContent = `${lane.enabled === false ? "Disabled" : "Enabled"} "${lane.label}". Saving helper lanes...`;
      renderHelper();
      await persistHelperLanes();
    }

    async function deleteHelperLane(laneId) {
      const registry = helperLaneRegistry();
      if (!registry || state.helper.savingLaneConfig || state.helper.verifyingLane || state.helper.importingLanes) {
        return;
      }

      const lanes = helperLaneEntries().map((lane) => ({ ...lane }));
      const lane = lanes.find((entry) => entry.id === laneId);
      if (!lane || lane.id === "local-rule-based") {
        return;
      }

      const confirmed = window.confirm(`Delete helper lane "${lane.label}" from this local host?`);
      if (!confirmed) {
        return;
      }

      const nextLanes = lanes.filter((entry) => entry.id !== laneId);
      const fallbackId = selectFallbackHelperLaneId(nextLanes);
      state.dashboard.helper.lane_registry = {
        ...registry,
        lanes: nextLanes,
        selected_lane_id: registry.selected_lane_id === laneId ? fallbackId : registry.selected_lane_id,
      };
      if (state.helper.editingLaneId === laneId) {
        state.helper.editingLaneId = fallbackId;
      }
      elements.helperLaneStatusNote.textContent = `Deleted "${lane.label}". Saving helper lanes...`;
      renderHelper();
      await persistHelperLanes();
    }

    async function copyHelperLaneExport(text) {
      try {
        await navigator.clipboard.writeText(text);
      } catch (error) {
        console.error(error);
        elements.helperLaneStatusNote.textContent = "Could not copy that export automatically. The transfer box still has the JSON.";
      }
    }

    async function exportHelperLanes() {
      const payload = collectHelperLaneDrafts();
      const safeExport = {
        export_kind: HELPER_LANE_EXPORT_KIND,
        schema_version: HELPER_LANE_EXPORT_SCHEMA_VERSION,
        exported_at_unix_seconds: Math.floor(Date.now() / 1000),
        note: "API keys are intentionally omitted from helper lane exports. Paste keys again after import on the new machine.",
        provider_catalog_snapshot: cloneMetadata(helperProviderCatalog()),
        selected_lane_id: payload.selected_lane_id,
        lanes: payload.lanes.map((lane) => ({
          id: lane.id,
          label: lane.label,
          lane_mode: lane.lane_mode,
          provider_kind: lane.provider_kind,
          model_name: lane.model_name,
          base_url: lane.base_url,
          metadata: exportLaneMetadata(lane),
          enabled: lane.enabled,
        })),
      };
      const text = JSON.stringify(safeExport, null, 2);
      elements.helperLaneTransferInput.value = text;
      saveTransferDraft();
      await copyHelperLaneExport(text);
      elements.helperLaneStatusNote.textContent = "Exported helper lanes into the transfer box and clipboard. API keys were intentionally omitted.";
    }

    function buildReplaceImportPayload(importPayload) {
      const importedLanes = Array.isArray(importPayload?.lanes) ? importPayload.lanes.map((lane) => ({ ...lane })) : [];
      if (!importedLanes.some((lane) => lane.id === "local-rule-based")) {
        importedLanes.unshift({
          id: "local-rule-based",
          label: "Local page-aware helper",
          lane_mode: "local",
          provider_kind: "local_builtin",
          model_name: "Rule-based guidance",
          base_url: null,
          api_key: null,
          metadata: null,
          enabled: true,
        });
      }

      return {
        selected_lane_id: importPayload?.selected_lane_id || selectFallbackHelperLaneId(importedLanes),
        lanes: importedLanes,
      };
    }

    function buildMergeImportPayload(currentPayload, importPayload) {
      const merged = new Map();
      for (const lane of Array.isArray(currentPayload?.lanes) ? currentPayload.lanes : []) {
        merged.set(lane.id, { ...lane });
      }
      for (const lane of Array.isArray(importPayload?.lanes) ? importPayload.lanes : []) {
        const existing = merged.get(lane.id) || {};
        merged.set(lane.id, {
          ...existing,
          ...lane,
          api_key: existing.api_key || null,
          metadata: lane.metadata ?? existing.metadata ?? null,
        });
      }

      const mergedLanes = Array.from(merged.values());
      if (!mergedLanes.some((lane) => lane.id === "local-rule-based")) {
        mergedLanes.unshift({
          id: "local-rule-based",
          label: "Local page-aware helper",
          lane_mode: "local",
          provider_kind: "local_builtin",
          model_name: "Rule-based guidance",
          base_url: null,
          api_key: null,
          metadata: null,
          enabled: true,
        });
      }

      const selectedLaneId = merged.has(importPayload?.selected_lane_id)
        ? importPayload.selected_lane_id
        : currentPayload?.selected_lane_id || selectFallbackHelperLaneId(mergedLanes);

      return {
        selected_lane_id: selectedLaneId,
        lanes: mergedLanes,
      };
    }

    async function importHelperLanes() {
      const raw = String(elements.helperLaneTransferInput.value || "").trim();
      if (!raw || state.helper.savingLaneConfig || state.helper.verifyingLane || state.helper.importingLanes) {
        return;
      }
      const importMode = normalizedImportPreviewMode(state.helper.importPreviewMode);
      const preview = previewImportPayload(importMode);
      if (!preview) {
        elements.helperLaneStatusNote.textContent = "Transfer JSON is not ready to import yet.";
        renderTransferInspection();
        return;
      }
      const finalPayload = preview.finalPayload;

      state.helper.importingLanes = true;
      renderHelper();

      try {
        const response = await fetch("/api/helper/lanes", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify(finalPayload),
        });
        const result = await response.json();
        if (!response.ok) {
          throw new Error(result.error || `Helper lane import failed with ${response.status}`);
        }
        state.dashboard.helper.lane_registry = result;
        state.helper.editingLaneId = result.selected_lane_id || state.helper.editingLaneId;
        state.helper.clearStoredKeyRequested = false;
        state.helper.importPreviewMode = importMode;
        saveTransferDraft();
        elements.helperLaneStatusNote.textContent = importMode === "REPLACE"
          ? "Replaced the local helper lane list from the transfer box. Paste API keys again where needed."
          : "Merged helper lanes from the transfer box into the local host settings. Paste API keys again where needed.";
      } catch (error) {
        console.error(error);
        elements.helperLaneStatusNote.textContent = `Could not import helper lanes yet. ${String(error.message || error)}`;
      } finally {
        state.helper.importingLanes = false;
        renderHelper();
      }
    }

    return {
      syncEditingLaneFromDashboard,
      hydrateTransferDraft,
      renderManager,
      renderTransferInspection,
      populateHelperLaneEditor,
      createDraftCloudHelperLane,
      seedExampleModel,
      fillDefaultBaseUrl,
      clearTransferDraft,
      rebaseTransferDraft,
      requestClearStoredHelperLaneKey,
      persistHelperLanes,
      verifyHelperLane,
      exportHelperLanes,
      importHelperLanes,
    };
  }

  function bind(context) {
    const api = withContext(context);

    context.elements.helperLaneSelect.addEventListener("change", () => {
      context.state.helper.editingLaneId = context.elements.helperLaneSelect.value || "";
      context.state.helper.clearStoredKeyRequested = false;
      api.populateHelperLaneEditor();
      context.renderHelper();
    });

    context.elements.helperLaneModeSelect.addEventListener("change", () => {
      context.renderHelper();
    });

    context.elements.helperLaneProviderSelect.addEventListener("change", () => {
      context.state.helper.clearStoredKeyRequested = false;
      context.renderHelper();
    });

    context.elements.helperLaneSeedModelButton.addEventListener("click", () => {
      api.seedExampleModel();
    });

    context.elements.helperLaneFillBaseUrlButton.addEventListener("click", () => {
      api.fillDefaultBaseUrl();
    });

    context.elements.helperLaneNewButton.addEventListener("click", () => {
      api.createDraftCloudHelperLane();
    });

    context.elements.helperLaneVerifyButton.addEventListener("click", () => {
      void api.verifyHelperLane();
    });

    context.elements.helperLaneClearKeyButton.addEventListener("click", () => {
      api.requestClearStoredHelperLaneKey();
    });

    context.elements.helperLaneExportButton.addEventListener("click", () => {
      void api.exportHelperLanes();
    });

    context.elements.helperLaneImportButton.addEventListener("click", () => {
      void api.importHelperLanes();
    });

    context.elements.helperLaneFilterSelect.addEventListener("change", () => {
      context.state.helper.laneScanFilter = context.elements.helperLaneFilterSelect.value || "all";
      context.renderHelper();
    });

    context.elements.helperLaneSortSelect.addEventListener("change", () => {
      context.state.helper.laneScanSort = context.elements.helperLaneSortSelect.value || "attention-first";
      context.renderHelper();
    });

    context.elements.helperLaneTransferInput.addEventListener("input", () => {
      saveTransferDraft();
      api.renderTransferInspection();
    });

    context.elements.helperLanePreviewMergeButton.addEventListener("click", () => {
      context.state.helper.importPreviewMode = "MERGE";
      saveTransferDraft();
      api.renderTransferInspection();
    });

    context.elements.helperLanePreviewReplaceButton.addEventListener("click", () => {
      context.state.helper.importPreviewMode = "REPLACE";
      saveTransferDraft();
      api.renderTransferInspection();
    });

    context.elements.helperLaneApplyImportButton.addEventListener("click", () => {
      void api.importHelperLanes();
    });

    context.elements.helperLaneRebaseTransferDraftButton.addEventListener("click", () => {
      api.rebaseTransferDraft();
    });

    context.elements.helperLaneClearTransferDraftButton.addEventListener("click", () => {
      api.clearTransferDraft();
    });

    context.elements.helperLaneSaveButton.addEventListener("click", () => {
      void api.persistHelperLanes();
    });
  }

  function buildApi(context) {
    return withContext(context);
  }

  window.ChattyLoraHelperLanes = {
    bind,
    buildApi,
  };
}());
