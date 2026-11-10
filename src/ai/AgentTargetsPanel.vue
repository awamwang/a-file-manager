<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * AIA-215 / A2 — Agent Target registry UI.
 * Discover / enable / customize; MCP usage prompt is stored only (not injected).
 */
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toastSuccess } from "../state/toast";
import {
  aiAgentTargetsDiscover,
  aiAgentTargetsList,
  aiAgentTargetsRemove,
  aiAgentTargetsSetDefaultInProjects,
  aiAgentTargetsSetEnabled,
  aiAgentTargetsUpdate,
  aiAgentTargetsUpsertCustom,
} from "./client";
import type { AgentTarget, AiConfigDocument } from "./types";

const { t } = useI18n();

const document = ref<AiConfigDocument | null>(null);
const error = ref<string | null>(null);
const loading = ref(false);
const selectedId = ref<string | null>(null);
const showAdd = ref(false);

const draftName = ref("");
const draftGlobalSkills = ref("");
const draftProjectSkills = ref("");
const draftGlobalMcp = ref("");
const draftProjectMcp = ref("");
const draftPrompt = ref("");

const customName = ref("");
const customId = ref("");
const customGlobalSkills = ref("");
const customProjectSkills = ref("");
const customGlobalMcp = ref("");
const customProjectMcp = ref("");
const customPrompt = ref("");

const AGENT_ERROR_KEYS: Record<string, string> = {
  AI_AGENT_ID_EMPTY: "ai.agents.errorIdEmpty",
  AI_AGENT_ID_INVALID: "ai.agents.errorIdInvalid",
  AI_AGENT_NAME_EMPTY: "ai.agents.errorNameEmpty",
  AI_AGENT_UNKNOWN: "ai.agents.errorUnknown",
  AI_AGENT_BUILTIN_REMOVE: "ai.agents.errorBuiltinRemove",
  AI_AGENT_ID_CONFLICT: "ai.agents.errorIdConflict",
};

function mapError(err: unknown): string {
  const raw = err instanceof Error ? err.message : String(err);
  for (const [code, key] of Object.entries(AGENT_ERROR_KEYS)) {
    if (raw.includes(code)) return t(key);
  }
  return raw;
}

const selected = computed(
  () => document.value?.agentTargets.find((a) => a.id === selectedId.value) ?? null,
);

const discoveredCount = computed(
  () => document.value?.agentTargets.filter((a) => a.discovered).length ?? 0,
);

const enabledCount = computed(
  () => document.value?.agentTargets.filter((a) => a.enabled).length ?? 0,
);

function syncDraft(target: AgentTarget | null) {
  if (!target) {
    draftName.value = "";
    draftGlobalSkills.value = "";
    draftProjectSkills.value = "";
    draftGlobalMcp.value = "";
    draftProjectMcp.value = "";
    draftPrompt.value = "";
    return;
  }
  draftName.value = target.displayName;
  draftGlobalSkills.value = target.globalSkillsDir;
  draftProjectSkills.value = target.projectSkillsDir;
  draftGlobalMcp.value = target.globalMcpPath;
  draftProjectMcp.value = target.projectMcpPath;
  draftPrompt.value = target.mcpUsagePrompt;
}

watch(selected, (value) => syncDraft(value));

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    document.value = await aiAgentTargetsList();
    if (
      selectedId.value &&
      !document.value.agentTargets.some((a) => a.id === selectedId.value)
    ) {
      selectedId.value = null;
    }
    if (!selectedId.value && document.value.agentTargets.length) {
      selectedId.value = document.value.agentTargets[0]?.id ?? null;
    }
  } catch (err) {
    error.value = mapError(err);
  } finally {
    loading.value = false;
  }
}

async function discover() {
  error.value = null;
  try {
    document.value = await aiAgentTargetsDiscover();
    toastSuccess(t("ai.agents.noticeDiscovered"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function toggleEnabled(target: AgentTarget) {
  error.value = null;
  try {
    document.value = await aiAgentTargetsSetEnabled(target.id, !target.enabled);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function toggleDefaultInProjects(target: AgentTarget) {
  error.value = null;
  try {
    document.value = await aiAgentTargetsSetDefaultInProjects(
      target.id,
      !target.defaultInProjects,
    );
  } catch (err) {
    error.value = mapError(err);
  }
}

async function saveSelected() {
  if (!selected.value) return;
  error.value = null;
  try {
    document.value = await aiAgentTargetsUpdate(selected.value.id, {
      displayName: draftName.value,
      globalSkillsDir: draftGlobalSkills.value,
      projectSkillsDir: draftProjectSkills.value,
      globalMcpPath: draftGlobalMcp.value,
      projectMcpPath: draftProjectMcp.value,
      mcpUsagePrompt: draftPrompt.value,
    });
    toastSuccess(t("ai.agents.noticeSaved"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function addCustom() {
  error.value = null;
  try {
    document.value = await aiAgentTargetsUpsertCustom({
      id: customId.value.trim() || null,
      displayName: customName.value,
      globalSkillsDir: customGlobalSkills.value,
      projectSkillsDir: customProjectSkills.value,
      globalMcpPath: customGlobalMcp.value,
      projectMcpPath: customProjectMcp.value,
      mcpUsagePrompt: customPrompt.value,
    });
    const created =
      document.value.agentTargets.find(
        (a) => a.displayName === customName.value.trim() && a.kind === "custom",
      ) ?? document.value.agentTargets.find((a) => a.kind === "custom");
    selectedId.value = created?.id ?? selectedId.value;
    showAdd.value = false;
    customName.value = "";
    customId.value = "";
    customGlobalSkills.value = "";
    customProjectSkills.value = "";
    customGlobalMcp.value = "";
    customProjectMcp.value = "";
    customPrompt.value = "";
    toastSuccess(t("ai.agents.noticeCustomAdded"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function removeSelected() {
  if (!selected.value || selected.value.kind !== "custom") return;
  error.value = null;
  try {
    const id = selected.value.id;
    document.value = await aiAgentTargetsRemove(id);
    selectedId.value = document.value.agentTargets[0]?.id ?? null;
    toastSuccess(t("ai.agents.noticeRemoved"));
  } catch (err) {
    error.value = mapError(err);
  }
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="agents-panel" data-ai-agents-panel>
    <div class="agents-toolbar">
      <p class="agents-summary">
        {{
          t("ai.agents.summary", {
            total: document?.agentTargets.length ?? 0,
            discovered: discoveredCount,
            enabled: enabledCount,
          })
        }}
      </p>
      <div class="agents-toolbar-actions">
        <button type="button" class="fluent-control" :disabled="loading" @click="discover">
          {{ t("ai.agents.discover") }}
        </button>
        <button type="button" class="fluent-control" @click="showAdd = !showAdd">
          {{ t("ai.agents.addCustom") }}
        </button>
      </div>
    </div>

    <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>

    <div v-if="showAdd" class="fluent-surface agents-add">
      <h3 class="agents-subhead">{{ t("ai.agents.addCustom") }}</h3>
      <label class="agents-field">
        <span>{{ t("ai.agents.fieldName") }}</span>
        <input v-model="customName" class="fluent-control" type="text" />
      </label>
      <label class="agents-field">
        <span>{{ t("ai.agents.fieldId") }}</span>
        <input
          v-model="customId"
          class="fluent-control"
          type="text"
          :placeholder="t('ai.agents.fieldIdHint')"
        />
      </label>
      <label class="agents-field">
        <span>{{ t("ai.agents.fieldGlobalSkills") }}</span>
        <input v-model="customGlobalSkills" class="fluent-control" type="text" />
      </label>
      <label class="agents-field">
        <span>{{ t("ai.agents.fieldProjectSkills") }}</span>
        <input v-model="customProjectSkills" class="fluent-control" type="text" />
      </label>
      <label class="agents-field">
        <span>{{ t("ai.agents.fieldGlobalMcp") }}</span>
        <input v-model="customGlobalMcp" class="fluent-control" type="text" />
      </label>
      <label class="agents-field">
        <span>{{ t("ai.agents.fieldProjectMcp") }}</span>
        <input v-model="customProjectMcp" class="fluent-control" type="text" />
      </label>
      <label class="agents-field">
        <span>{{ t("ai.agents.fieldMcpPrompt") }}</span>
        <textarea
          v-model="customPrompt"
          class="fluent-control agents-textarea"
          rows="2"
          :placeholder="t('ai.agents.fieldMcpPromptHint')"
        />
      </label>
      <div class="agents-row-actions">
        <button type="button" class="fluent-control fluent-control--accent" @click="addCustom">
          {{ t("common.create") }}
        </button>
        <button type="button" class="fluent-control" @click="showAdd = false">
          {{ t("common.cancel") }}
        </button>
      </div>
    </div>

    <div v-if="document" class="agents-layout">
      <ul class="agents-list" role="listbox" :aria-label="t('ai.sections.agents.title')">
        <li
          v-for="agent in document.agentTargets"
          :key="agent.id"
          class="agents-list-item"
          :class="{ 'agents-list-item--active': agent.id === selectedId }"
          role="option"
          :aria-selected="agent.id === selectedId"
          @click="selectedId = agent.id"
        >
          <div class="agents-list-main">
            <strong>{{ agent.displayName }}</strong>
            <span class="agents-list-meta">
              {{ agent.id }}
              ·
              {{
                agent.kind === "custom"
                  ? t("ai.agents.kindCustom")
                  : t("ai.agents.kindBuiltin")
              }}
              <template v-if="agent.discovered"> · {{ t("ai.agents.badgeDiscovered") }}</template>
            </span>
          </div>
          <div class="agents-list-toggles">
            <button
              type="button"
              class="fluent-control agents-enable"
              @click.stop="toggleEnabled(agent)"
            >
              {{ agent.enabled ? t("ai.agents.disable") : t("ai.agents.enable") }}
            </button>
            <button
              type="button"
              class="fluent-control agents-enable"
              @click.stop="toggleDefaultInProjects(agent)"
            >
              {{
                agent.defaultInProjects
                  ? t("ai.agents.defaultInProjectsOn")
                  : t("ai.agents.defaultInProjectsOff")
              }}
            </button>
          </div>
        </li>
      </ul>

      <div v-if="selected" class="fluent-surface agents-detail">
        <h3 class="agents-subhead">{{ selected.displayName }}</h3>
        <p class="agents-hint">{{ t("ai.agents.defaultInProjectsHint") }}</p>
        <label class="agents-field">
          <span>{{ t("ai.agents.fieldName") }}</span>
          <input v-model="draftName" class="fluent-control" type="text" />
        </label>
        <label class="agents-field">
          <span>{{ t("ai.agents.fieldGlobalSkills") }}</span>
          <input v-model="draftGlobalSkills" class="fluent-control" type="text" />
        </label>
        <label class="agents-field">
          <span>{{ t("ai.agents.fieldProjectSkills") }}</span>
          <input v-model="draftProjectSkills" class="fluent-control" type="text" />
        </label>
        <label class="agents-field">
          <span>{{ t("ai.agents.fieldGlobalMcp") }}</span>
          <input v-model="draftGlobalMcp" class="fluent-control" type="text" />
        </label>
        <label class="agents-field">
          <span>{{ t("ai.agents.fieldProjectMcp") }}</span>
          <input v-model="draftProjectMcp" class="fluent-control" type="text" />
        </label>
        <label class="agents-field">
          <span>{{ t("ai.agents.fieldMcpPrompt") }}</span>
          <textarea
            v-model="draftPrompt"
            class="fluent-control agents-textarea"
            rows="3"
            :placeholder="t('ai.agents.fieldMcpPromptHint')"
          />
          <span class="agents-hint">{{ t("ai.agents.mcpPromptNote") }}</span>
        </label>
        <div class="agents-row-actions">
          <button
            type="button"
            class="fluent-control fluent-control--accent"
            @click="saveSelected"
          >
            {{ t("ai.agents.save") }}
          </button>
          <button
            v-if="selected.kind === 'custom'"
            type="button"
            class="fluent-control"
            @click="removeSelected"
          >
            {{ t("common.delete") }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.agents-panel {
  margin-top: var(--fluent-space-md);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}

.agents-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-sm);
}

.agents-summary {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.agents-toolbar-actions {
  display: flex;
  gap: var(--fluent-space-sm);
}

.agents-layout {
  display: grid;
  grid-template-columns: minmax(12rem, 16rem) minmax(0, 1fr);
  gap: var(--fluent-space-md);
}

@media (max-width: 720px) {
  .agents-layout {
    grid-template-columns: 1fr;
  }
}

.agents-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
  max-height: 28rem;
  overflow: auto;
}

.agents-list-item {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
  color: var(--fluent-color-text-primary);
  cursor: pointer;
}

.agents-list-item:hover {
  background: var(--fluent-color-fill-control-hover);
}

.agents-list-item--active {
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
  box-shadow: inset 3px 0 0 var(--fluent-color-stroke-accent-soft);
}

.agents-list-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.agents-list-meta {
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.agents-list-toggles {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex-shrink: 0;
}

.agents-enable {
  flex-shrink: 0;
  font-size: var(--fluent-font-size-caption);
}

.agents-detail,
.agents-add {
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.agents-subhead {
  margin: 0 0 var(--fluent-space-xs);
  font-size: var(--fluent-font-size-body);
}

.agents-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.agents-textarea {
  resize: vertical;
  min-height: 3.5rem;
  font-family: var(--fluent-font-family-mono);
}

.agents-hint {
  font-size: var(--fluent-font-size-caption);
  opacity: 0.85;
}

.agents-row-actions {
  display: flex;
  gap: var(--fluent-space-sm);
  margin-top: var(--fluent-space-xs);
}
</style>
