function defaultConceptBlock() {
  return {
    role: "primary",
    concept_type: "style",
    trigger_phrase: "",
    concept_summary: "",
    training_intent: "",
    expanded: true,
  };
}

function normalizedConceptBlock(block, { expanded } = {}) {
  return {
    role: normalizeConceptRole(block?.role),
    concept_type: String(block?.concept_type || "style"),
    trigger_phrase: String(block?.trigger_phrase || "").trim(),
    concept_summary: String(block?.concept_summary || "").trim(),
    training_intent: String(block?.training_intent || "").trim(),
    expanded: expanded ?? Boolean(block?.expanded),
  };
}

function normalizeConceptRole(value) {
  const normalized = String(value || "").trim().toLowerCase();
  if (normalized === "supporting" || normalized === "avoid") {
    return normalized;
  }
  return "primary";
}

function normalizedConceptBlocks(blocks) {
  return (Array.isArray(blocks) ? blocks : [])
    .map((block) => normalizedConceptBlock(block))
    .filter((block) => block.trigger_phrase || block.concept_summary || block.training_intent)
    .map((block) => ({
      ...block,
      expanded: Boolean(block.expanded),
    }));
}

function serializeConceptBlocks(blocks) {
  return normalizedConceptBlocks(blocks).map((block) => ({
    role: block.role,
    concept_type: block.concept_type,
    trigger_phrase: block.trigger_phrase,
    concept_summary: block.concept_summary,
    training_intent: block.training_intent,
  }));
}

function leadConceptBlock(blocks) {
  return serializeConceptBlocks(blocks).find((block) => block.role === "primary")
    || serializeConceptBlocks(blocks)[0]
    || defaultConceptBlock();
}

function projectConceptBlocks(project) {
  const fromProject = Array.isArray(project?.concept_blocks) ? project.concept_blocks : [];
  const blocks = normalizedConceptBlocks(fromProject);
  if (blocks.length) {
    return blocks;
  }
  if (project?.concept_type || project?.trigger_phrase || project?.concept_summary) {
    return [
      normalizedConceptBlock({
        role: project.concept_role || "primary",
        concept_type: project.concept_type || "style",
        trigger_phrase: project.trigger_phrase || "",
        concept_summary: project.concept_summary || "",
        training_intent: "",
      }, { expanded: true }),
    ];
  }
  return [defaultConceptBlock()];
}

function conceptGuidance(value) {
  const guidance = {
    style: {
      title: "Style / aesthetic LoRA",
      body: "Best when the dataset shares a look: color language, texture, composition, rendering style, or camera mood.",
      help: "Use this when the look matters more than one exact subject.",
    },
    character: {
      title: "Character / likeness LoRA",
      body: "Best when identity consistency matters across the whole person: recognizable subject shape, body language, markings, and recurring visual identity.",
      help: "Use this when the same subject needs to stay recognizable beyond one exact portrait.",
    },
    portrait: {
      title: "Face / portrait LoRA",
      body: "Best when facial identity, portrait framing, and face-level consistency matter most.",
      help: "Use this when the face is the lesson, not the full-body styling.",
    },
    outfit: {
      title: "Outfit / costume LoRA",
      body: "Best when a recurring wardrobe, silhouette, accessory set, or costume language is the concept being taught.",
      help: "Use this when clothing language matters more than one exact person.",
    },
    motion: {
      title: "Motion pattern LoRA",
      body: "Best for Wan video concepts where movement pattern matters: how a subject turns, flies, walks, gestures, or animates.",
      help: "Use this for video-first concepts where the movement is the lesson.",
    },
    pose: {
      title: "Pose / action LoRA",
      body: "Best when the still-frame pose vocabulary matters: stances, hand placement, body angles, and recurring action framing.",
      help: "Use this when the posture or action silhouette is the main lesson.",
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
    composition: {
      title: "Composition / camera language LoRA",
      body: "Best when the visual lesson is about framing, lens feel, shot distance, angle bias, or repeated composition patterns.",
      help: "Use this when the camera language is more important than the subject itself.",
    },
    expression: {
      title: "Expression / mood LoRA",
      body: "Best when the concept is emotional read, facial mood, or a recurring affective tone across the dataset.",
      help: "Use this when the emotional signal is the thing being taught.",
    },
    assistant: {
      title: "Assistant / persona LoRA",
      body: "Useful later for text/persona lanes. For the current Wan lane, prefer style, character, object, location, or motion.",
      help: "Future-facing for non-video personality or assistant training lanes.",
    },
  };
  return guidance[value] || guidance.style;
}

function conceptRoleGuidance(value) {
  const role = normalizeConceptRole(value);
  if (role === "supporting") {
    return {
      label: "supporting detail",
      body: "Supporting blocks add context after the primary lesson. Use them for outfit, environment, pose, or composition details that should help without dominating.",
      help: "Useful for secondary details that should support, not lead.",
    };
  }
  if (role === "avoid") {
    return {
      label: "avoid guardrail",
      body: "Avoid blocks are saved as reminders about patterns you do not want to reinforce. They stay out of the positive caption recipe for the current Wan lane.",
      help: "Use this to record recurring distractions or bad habits in the dataset.",
    };
  }
  return {
    label: "primary lesson",
    body: "Primary blocks lead the caption recipe. Put the main identity, style, or core concept here first.",
    help: "Use this for the main thing the LoRA should learn.",
  };
}

function triggerPhraseHelp(value) {
  const trimmed = String(value || "").trim();
  if (!trimmed) {
    return "Use a short rare phrase. This is the handle you prompt with later to call the LoRA.";
  }
  if (trimmed.length < 4) {
    return "This trigger is very short. It may collide with common prompt text.";
  }
  if (/\s/.test(trimmed)) {
    return "This trigger includes spaces. Short underscored terms are usually easier to reuse consistently.";
  }
  if (/^[a-z]+$/.test(trimmed) && trimmed.length <= 8) {
    return "This trigger is simple and readable, but make sure it is rare enough not to collide with normal language.";
  }
  return "This trigger looks distinct enough for a starter handle.";
}

function currentConceptComposerBlock() {
  return normalizedConceptBlock({
    role: elements.conceptRoleSelect.value,
    concept_type: elements.conceptTypeSelect.value,
    trigger_phrase: elements.triggerPhraseInput.value,
    concept_summary: elements.conceptSummaryInput.value,
    training_intent: elements.trainingIntentInput.value,
  }, { expanded: true });
}

function addConceptBlockFromComposer() {
  const block = currentConceptComposerBlock();
  if (!block.trigger_phrase && !block.concept_summary && !block.training_intent) {
    elements.conceptStackNote.textContent = "Add a trigger term, concept details, or training intent before stacking a block.";
    return;
  }
  state.builder.conceptBlocks = normalizedConceptBlocks([
    ...state.builder.conceptBlocks.map((item) => ({ ...item, expanded: false })),
    block,
  ]).map((item, index, items) => ({ ...item, expanded: index === items.length - 1 }));
  elements.conceptStackNote.textContent = `Added ${conceptLabel(block)} to the concept stack.`;
  clearConceptComposer({ keepType: true });
  renderBuilderGuidance();
}

function clearConceptComposer({ keepType = false } = {}) {
  if (!keepType) {
    elements.conceptTypeSelect.value = "style";
  }
  elements.conceptRoleSelect.value = "primary";
  elements.triggerPhraseInput.value = "";
  elements.trainingIntentInput.value = "";
  elements.conceptSummaryInput.value = "";
  renderBuilderGuidance();
}

function conceptStackTransferPayload() {
  return {
    version: 1,
    exported_at_unix_seconds: Math.floor(Date.now() / 1000),
    concept_blocks: serializeConceptBlocks(state.builder.conceptBlocks),
  };
}

async function exportConceptStack() {
  const payload = conceptStackTransferPayload();
  const json = JSON.stringify(payload, null, 2);
  elements.conceptTransferInput.value = json;

  try {
    await navigator.clipboard.writeText(json);
    elements.conceptStackNote.textContent = "Exported the concept stack to the transfer box and copied it to the clipboard.";
  } catch (_error) {
    elements.conceptStackNote.textContent = "Exported the concept stack to the transfer box. Clipboard copy was not available.";
  }
}

function importConceptStack(mode) {
  const raw = elements.conceptTransferInput.value.trim();
  if (!raw) {
    elements.conceptStackNote.textContent = "Paste exported concept stack JSON into the transfer box first.";
    return;
  }

  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (_error) {
    elements.conceptStackNote.textContent = "The transfer box does not contain valid JSON yet.";
    return;
  }

  const importedBlocks = normalizeImportedConceptBlocks(parsed);
  if (!importedBlocks.length) {
    elements.conceptStackNote.textContent = "No usable concept blocks were found in that JSON.";
    return;
  }

  const nextBlocks = mode === "append"
    ? normalizedConceptBlocks([...state.builder.conceptBlocks, ...importedBlocks])
    : normalizedConceptBlocks(importedBlocks);

  state.builder.conceptBlocks = nextBlocks.map((block, index) => ({
    ...block,
    expanded: mode === "append" ? index >= nextBlocks.length - importedBlocks.length : index === 0,
  }));
  state.builder.editingConceptIndex = -1;
  elements.conceptStackNote.textContent = `${mode === "append" ? "Appended" : "Imported"} ${importedBlocks.length} concept block${importedBlocks.length === 1 ? "" : "s"} from the transfer box.`;
  renderBuilderGuidance();
}

function normalizeImportedConceptBlocks(parsed) {
  const sourceBlocks = Array.isArray(parsed)
    ? parsed
    : Array.isArray(parsed?.concept_blocks)
      ? parsed.concept_blocks
      : [];

  return normalizedConceptBlocks(
    sourceBlocks.map((block) => normalizedConceptBlock({
      role: block?.role,
      concept_type: block?.concept_type,
      trigger_phrase: block?.trigger_phrase,
      training_intent: block?.training_intent,
      concept_summary: block?.concept_summary,
      expanded: true,
    })),
  );
}

function startEditingConceptBlock(index) {
  state.builder.editingConceptIndex = index;
  state.builder.conceptBlocks = normalizedConceptBlocks(
    state.builder.conceptBlocks.map((block, itemIndex) => ({
      ...block,
      expanded: itemIndex === index ? true : block.expanded,
    })),
  );
  renderConceptBlockList();
}

function cancelEditingConceptBlock() {
  state.builder.editingConceptIndex = -1;
  renderConceptBlockList();
}

function saveEditingConceptBlock(index) {
  const prefix = `concept-edit-${index}`;
  const roleSelect = document.getElementById(`${prefix}-role`);
  const typeSelect = document.getElementById(`${prefix}-type`);
  const triggerInput = document.getElementById(`${prefix}-trigger`);
  const intentInput = document.getElementById(`${prefix}-intent`);
  const summaryInput = document.getElementById(`${prefix}-summary`);

  if (!roleSelect || !typeSelect || !triggerInput || !intentInput || !summaryInput) {
    elements.conceptStackNote.textContent = "Could not read that concept block editor.";
    return;
  }

  const updatedBlock = normalizedConceptBlock({
    role: roleSelect.value,
    concept_type: typeSelect.value,
    trigger_phrase: triggerInput.value,
    training_intent: intentInput.value,
    concept_summary: summaryInput.value,
    expanded: true,
  });

  if (!updatedBlock.trigger_phrase && !updatedBlock.concept_summary && !updatedBlock.training_intent) {
    elements.conceptStackNote.textContent = "Edited blocks still need a trigger, concept details, or training intent.";
    return;
  }

  state.builder.conceptBlocks = normalizedConceptBlocks(
    state.builder.conceptBlocks.map((block, itemIndex) => (
      itemIndex === index ? updatedBlock : block
    )),
  );
  state.builder.editingConceptIndex = -1;
  elements.conceptStackNote.textContent = `Updated ${conceptLabel(updatedBlock)}.`;
  renderBuilderGuidance();
}

function removeConceptBlock(index) {
  state.builder.conceptBlocks = normalizedConceptBlocks(
    state.builder.conceptBlocks.filter((_, itemIndex) => itemIndex !== index),
  ).map((item, itemIndex) => ({ ...item, expanded: itemIndex === 0 ? true : item.expanded }));
  if (state.builder.editingConceptIndex === index) {
    state.builder.editingConceptIndex = -1;
  } else if (state.builder.editingConceptIndex > index) {
    state.builder.editingConceptIndex -= 1;
  }
  elements.conceptStackNote.textContent = "Removed that concept block from the stack.";
  renderBuilderGuidance();
}

function toggleConceptBlockExpanded(index) {
  state.builder.conceptBlocks = normalizedConceptBlocks(
    state.builder.conceptBlocks.map((block, itemIndex) => ({
      ...block,
      expanded: itemIndex === index ? !block.expanded : block.expanded,
    })),
  );
  renderConceptBlockList();
}

function moveConceptBlock(index, direction) {
  const blocks = normalizedConceptBlocks(state.builder.conceptBlocks).map((block) => ({ ...block }));
  const targetIndex = index + direction;
  if (targetIndex < 0 || targetIndex >= blocks.length) {
    return;
  }

  const [moved] = blocks.splice(index, 1);
  blocks.splice(targetIndex, 0, moved);
  state.builder.conceptBlocks = blocks;
  if (state.builder.editingConceptIndex === index) {
    state.builder.editingConceptIndex = targetIndex;
  } else if (direction < 0 && state.builder.editingConceptIndex >= targetIndex && state.builder.editingConceptIndex < index) {
    state.builder.editingConceptIndex += 1;
  } else if (direction > 0 && state.builder.editingConceptIndex <= targetIndex && state.builder.editingConceptIndex > index) {
    state.builder.editingConceptIndex -= 1;
  }
  elements.conceptStackNote.textContent = `Moved ${conceptLabel(moved)} ${direction < 0 ? "up" : "down"} in the stack.`;
  renderBuilderGuidance();
}

function duplicateConceptBlock(index) {
  const blocks = normalizedConceptBlocks(state.builder.conceptBlocks).map((block) => ({ ...block }));
  const source = blocks[index];
  if (!source) {
    return;
  }

  const duplicate = {
    ...source,
    expanded: true,
  };
  blocks.splice(index + 1, 0, duplicate);
  state.builder.conceptBlocks = blocks.map((block, itemIndex) => ({
    ...block,
    expanded: itemIndex === index + 1,
  }));

  if (state.builder.editingConceptIndex > index) {
    state.builder.editingConceptIndex += 1;
  }
  state.builder.editingConceptIndex = index + 1;
  elements.conceptStackNote.textContent = `Duplicated ${conceptLabel(source)}. Tweak the copy and save it.`;
  renderBuilderGuidance();
}

function conceptLabel(block) {
  const guidance = conceptGuidance(block.concept_type);
  const role = conceptRoleGuidance(block.role);
  return `${guidance.title.replace(" LoRA", "")} (${role.label})`;
}

function renderConceptTypeOptions(selectedValue) {
  return [
    ["style", "Style / aesthetic"],
    ["character", "Character / likeness"],
    ["portrait", "Face / portrait"],
    ["outfit", "Outfit / costume"],
    ["object", "Object / product"],
    ["location", "Location / environment"],
    ["pose", "Pose / action"],
    ["motion", "Motion pattern"],
    ["composition", "Composition / camera language"],
    ["expression", "Expression / mood"],
    ["assistant", "Assistant / persona"],
  ]
    .map(([value, label]) => `<option value="${escapeAttribute(value)}" ${selectedValue === value ? "selected" : ""}>${escapeHtml(label)}</option>`)
    .join("");
}

function renderConceptBlockEditor(block, index) {
  const prefix = `concept-edit-${index}`;
  return `
    <div class="concept-block-editor">
      <div class="builder-settings-grid">
        <label class="field-block compact">
          <span class="field-title">Role</span>
          <select id="${prefix}-role">
            <option value="primary" ${block.role === "primary" ? "selected" : ""}>Primary lesson</option>
            <option value="supporting" ${block.role === "supporting" ? "selected" : ""}>Supporting detail</option>
            <option value="avoid" ${block.role === "avoid" ? "selected" : ""}>Avoid / don't reinforce</option>
          </select>
        </label>
        <label class="field-block compact">
          <span class="field-title">Type</span>
          <select id="${prefix}-type">
            ${renderConceptTypeOptions(block.concept_type)}
          </select>
        </label>
      </div>
      <div class="builder-settings-grid">
        <label class="field-block compact">
          <span class="field-title">Trigger term</span>
          <input id="${prefix}-trigger" type="text" value="${escapeAttribute(block.trigger_phrase)}" />
        </label>
        <label class="field-block compact">
          <span class="field-title">Training intent</span>
          <input id="${prefix}-intent" type="text" value="${escapeAttribute(block.training_intent)}" />
        </label>
      </div>
      <label class="field-block">
        <span class="field-title">Concept details</span>
        <textarea id="${prefix}-summary" rows="4">${escapeHtml(block.concept_summary)}</textarea>
      </label>
      <div class="inline-actions compact">
        <button class="primary-button" type="button" data-save-concept-block="${index}">Save</button>
        <button class="secondary-button" type="button" data-cancel-concept-block="${index}">Cancel</button>
      </div>
    </div>
  `;
}

function conceptBlockSummary(block) {
  const parts = [];
  if (block.training_intent) {
    parts.push(block.training_intent);
  }
  if (block.concept_summary) {
    parts.push(block.concept_summary);
  }
  if (!parts.length) {
    return "No extra detail yet.";
  }
  const summary = parts.join(" ").replaceAll(/\s+/g, " ").trim();
  return summary.length > 140 ? `${summary.slice(0, 137).trimEnd()}...` : summary;
}

function conceptBlockBadgeClass(role) {
  if (role === "avoid") {
    return "warm-badge";
  }
  if (role === "supporting") {
    return "muted-badge";
  }
  return "ok-badge";
}

function renderConceptBlockMeta(block) {
  const typeLabel = conceptGuidance(block.concept_type).title.replace(" LoRA", "");
  const roleLabel = conceptRoleGuidance(block.role).label;
  return `
    <div class="concept-block-meta">
      <span class="list-badge ${conceptBlockBadgeClass(block.role)}">${escapeHtml(roleLabel)}</span>
      <span class="list-badge">${escapeHtml(typeLabel)}</span>
      ${block.trigger_phrase ? `<span class="list-badge">${escapeHtml(block.trigger_phrase)}</span>` : ""}
    </div>
  `;
}

function renderConceptBlockReadOnly(block) {
  return `
    <div class="concept-block-readout">
      <p><strong>Role:</strong> ${escapeHtml(conceptRoleGuidance(block.role).body)}</p>
      <p><strong>Training intent:</strong> ${escapeHtml(block.training_intent || "Not specified")}</p>
      <p><strong>Concept details:</strong> ${escapeHtml(block.concept_summary || "Not specified")}</p>
    </div>
  `;
}

function renderConceptBlockActions(blocks, index) {
  return `
    <div class="concept-block-toolbar">
      <button class="secondary-button concept-order-button" type="button" data-move-concept-block-up="${index}" ${index === 0 ? "disabled" : ""}>Up</button>
      <button class="secondary-button concept-order-button" type="button" data-move-concept-block-down="${index}" ${index === blocks.length - 1 ? "disabled" : ""}>Down</button>
      <button class="secondary-button concept-edit-button" type="button" data-edit-concept-block="${index}">Edit</button>
      <button class="secondary-button concept-duplicate-button" type="button" data-duplicate-concept-block="${index}">Duplicate</button>
      <button class="secondary-button saved-plan-delete-button concept-remove-button" type="button" data-remove-concept-block="${index}">X</button>
    </div>
  `;
}

function bindConceptBlockListEvents() {
  for (const button of elements.conceptBlockList.querySelectorAll("[data-toggle-concept-block]")) {
    button.addEventListener("click", () => {
      toggleConceptBlockExpanded(Number(button.dataset.toggleConceptBlock));
    });
  }
  for (const button of elements.conceptBlockList.querySelectorAll("[data-move-concept-block-up]")) {
    button.addEventListener("click", () => {
      moveConceptBlock(Number(button.dataset.moveConceptBlockUp), -1);
    });
  }
  for (const button of elements.conceptBlockList.querySelectorAll("[data-move-concept-block-down]")) {
    button.addEventListener("click", () => {
      moveConceptBlock(Number(button.dataset.moveConceptBlockDown), 1);
    });
  }
  for (const button of elements.conceptBlockList.querySelectorAll("[data-edit-concept-block]")) {
    button.addEventListener("click", () => {
      startEditingConceptBlock(Number(button.dataset.editConceptBlock));
    });
  }
  for (const button of elements.conceptBlockList.querySelectorAll("[data-duplicate-concept-block]")) {
    button.addEventListener("click", () => {
      duplicateConceptBlock(Number(button.dataset.duplicateConceptBlock));
    });
  }
  for (const button of elements.conceptBlockList.querySelectorAll("[data-save-concept-block]")) {
    button.addEventListener("click", () => {
      saveEditingConceptBlock(Number(button.dataset.saveConceptBlock));
    });
  }
  for (const button of elements.conceptBlockList.querySelectorAll("[data-cancel-concept-block]")) {
    button.addEventListener("click", () => {
      cancelEditingConceptBlock();
    });
  }
  for (const button of elements.conceptBlockList.querySelectorAll("[data-remove-concept-block]")) {
    button.addEventListener("click", () => {
      removeConceptBlock(Number(button.dataset.removeConceptBlock));
    });
  }
}

function renderConceptBlockList() {
  const blocks = normalizedConceptBlocks(state.builder.conceptBlocks);
  if (!blocks.length) {
    elements.conceptBlockList.innerHTML = `<div class="empty-state">No concept blocks yet. Add one to describe what this LoRA should learn.</div>`;
    return;
  }

  elements.conceptBlockList.innerHTML = blocks
    .map((block, index) => `
      <article class="concept-block-card ${state.builder.editingConceptIndex === index ? "editing" : ""}">
        <div class="concept-block-header">
          <button class="secondary-button concept-toggle-button" type="button" data-toggle-concept-block="${index}">
            ${block.expanded ? "-" : "+"}
          </button>
          <div class="concept-block-copy">
            <strong>${escapeHtml(conceptLabel(block))}</strong>
            ${renderConceptBlockMeta(block)}
            <p>${escapeHtml(conceptBlockSummary(block))}</p>
          </div>
          ${renderConceptBlockActions(blocks, index)}
        </div>
        ${block.expanded ? `
          <div class="concept-block-details">
            ${state.builder.editingConceptIndex === index
              ? renderConceptBlockEditor(block, index)
              : renderConceptBlockReadOnly(block)}
          </div>
        ` : ""}
      </article>
    `)
    .join("");

  bindConceptBlockListEvents();
}
