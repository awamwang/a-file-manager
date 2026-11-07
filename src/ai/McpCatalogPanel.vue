<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/** AIA-218 / A5 — Public MCP catalog + merge deploy (afm__* keys only). */
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { toastSuccess } from "../state/toast";
import type { AiConfigDocument, McpCatalogEntry } from "./types";

const { t } = useI18n();

const document = ref<AiConfigDocument | null>(null);
const error = ref<string | null>(null);
const loading = ref(false);
const selectedId = ref<string | null>(null);
const projectWorkspace = ref("");
const draftName = ref("");
const draftId = ref("");
const draftJson = ref('{\n  "command": "npx",\n  "args": ["-y", "example"]\n}');
const lastNote = ref<string | null>(null);

const ERROR_KEYS: Record<string, string> = {
  AI_MCP_ID_EMPTY: "ai.mcp.errorIdEmpty",
  AI_MCP_ID_INVALID: "ai.mcp.errorIdInvalid",
  AI_MCP_UNKNOWN: "ai.mcp.errorUnknown",
  AI_MCP_DEPLOY_NO_ENABLED: "ai.mcp.errorDeployNoEnabled",
  AI_MCP_TARGET_INVALID: "ai.mcp.errorTargetInvalid",
};

function mapError(err: unknown): string {
  const raw = err instanceof Error ? err.message : String(err);
  for (const [code, key] of Object.entries(ERROR_KEYS)) {
    if (raw.includes(code)) return t(key);
  }
  return raw;
}

const selected = computed(
  () => document.value?.mcpCatalog?.find((e) => e.id === selectedId.value) ?? null,
);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    document.value = await invoke<AiConfigDocument>("ai_mcp_catalog_list");
    if (!selectedId.value && document.value.mcpCatalog?.length) {
      selectedId.value = document.value.mcpCatalog[0]?.id ?? null;
    }
  } catch (err) {
    error.value = mapError(err);
  } finally {
    loading.value = false;
  }
}

async function saveEntry() {
  error.value = null;
  try {
    document.value = await invoke<AiConfigDocument>("ai_mcp_catalog_upsert", {
      id: draftId.value.trim() || selected.value?.id || null,
      displayName: draftName.value,
      serverJson: draftJson.value,
    });
    selectedId.value =
      document.value.mcpCatalog?.find((e) => e.displayName === draftName.value.trim())?.id ??
      selectedId.value;
    toastSuccess(t("ai.mcp.noticeSaved"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function removeSelected() {
  if (!selected.value) return;
  error.value = null;
  try {
    document.value = await invoke<AiConfigDocument>("ai_mcp_catalog_remove", {
      id: selected.value.id,
    });
    selectedId.value = document.value.mcpCatalog?.[0]?.id ?? null;
    toastSuccess(t("ai.mcp.noticeRemoved"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function deploySelected() {
  if (!selected.value) return;
  error.value = null;
  lastNote.value = null;
  try {
    const report = await invoke<{ written: string[]; skipped: string[] }>("ai_mcp_deploy", {
      entryId: selected.value.id,
      projectWorkspace: projectWorkspace.value.trim() || null,
    });
    lastNote.value = t("ai.mcp.noticeDeployed", {
      count: report.written.length,
      skipped: report.skipped.length,
    });
    toastSuccess(lastNote.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function undeploySelected() {
  if (!selected.value) return;
  error.value = null;
  try {
    const report = await invoke<{ written: string[] }>("ai_mcp_undeploy", {
      entryId: selected.value.id,
      projectWorkspace: projectWorkspace.value.trim() || null,
    });
    lastNote.value = t("ai.mcp.noticeUndeployed", { count: report.written.length });
    toastSuccess(lastNote.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

function selectEntry(entry: McpCatalogEntry) {
  selectedId.value = entry.id;
  draftId.value = entry.id;
  draftName.value = entry.displayName;
  try {
    draftJson.value = JSON.stringify(JSON.parse(entry.serverJson), null, 2);
  } catch {
    draftJson.value = entry.serverJson;
  }
}

function startNew() {
  selectedId.value = null;
  draftId.value = "";
  draftName.value = "";
  draftJson.value = '{\n  "command": "npx",\n  "args": ["-y", "example"]\n}';
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="mcp-panel" data-ai-mcp-panel>
    <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>
    <p class="mcp-hint">{{ t("ai.mcp.mergeHint") }}</p>

    <div class="mcp-toolbar">
      <button type="button" class="fluent-control" @click="startNew">
        {{ t("ai.mcp.add") }}
      </button>
      <input
        v-model="projectWorkspace"
        class="fluent-control mcp-grow"
        type="text"
        :placeholder="t('ai.mcp.projectWorkspaceHint')"
      />
    </div>

    <div class="mcp-layout">
      <ul class="mcp-list" role="listbox" :aria-label="t('ai.sections.mcp.title')">
        <li
          v-for="entry in document?.mcpCatalog ?? []"
          :key="entry.id"
          class="mcp-list-item"
          :class="{ 'mcp-list-item--active': entry.id === selectedId }"
          role="option"
          :aria-selected="entry.id === selectedId"
          @click="selectEntry(entry)"
        >
          <strong>{{ entry.displayName }}</strong>
          <span class="mcp-meta">{{ entry.id }}</span>
        </li>
        <li v-if="!(document?.mcpCatalog?.length)" class="mcp-meta">
          {{ t("ai.mcp.empty") }}
        </li>
      </ul>

      <div class="fluent-surface mcp-detail">
        <label class="mcp-field">
          <span>{{ t("ai.mcp.fieldName") }}</span>
          <input v-model="draftName" class="fluent-control" type="text" />
        </label>
        <label class="mcp-field">
          <span>{{ t("ai.mcp.fieldId") }}</span>
          <input v-model="draftId" class="fluent-control" type="text" />
        </label>
        <label class="mcp-field">
          <span>{{ t("ai.mcp.fieldJson") }}</span>
          <textarea v-model="draftJson" class="fluent-control mcp-json" rows="8" />
        </label>
        <div class="mcp-actions">
          <button type="button" class="fluent-control fluent-control--accent" @click="saveEntry">
            {{ t("ai.mcp.save") }}
          </button>
          <button type="button" class="fluent-control" :disabled="!selected" @click="deploySelected">
            {{ t("ai.mcp.deploy") }}
          </button>
          <button
            type="button"
            class="fluent-control"
            :disabled="!selected"
            @click="undeploySelected"
          >
            {{ t("ai.mcp.undeploy") }}
          </button>
          <button type="button" class="fluent-control" :disabled="!selected" @click="removeSelected">
            {{ t("common.remove") }}
          </button>
        </div>
        <p v-if="lastNote" class="mcp-hint">{{ lastNote }}</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mcp-panel {
  margin-top: var(--fluent-space-md);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}
.mcp-hint,
.mcp-meta {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}
.mcp-toolbar {
  display: flex;
  gap: var(--fluent-space-sm);
  flex-wrap: wrap;
}
.mcp-grow {
  flex: 1;
  min-width: 12rem;
}
.mcp-layout {
  display: grid;
  grid-template-columns: minmax(10rem, 14rem) minmax(0, 1fr);
  gap: var(--fluent-space-md);
}
@media (max-width: 720px) {
  .mcp-layout {
    grid-template-columns: 1fr;
  }
}
.mcp-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
}
.mcp-list-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
  color: var(--fluent-color-text-primary);
  cursor: pointer;
}
.mcp-list-item:hover {
  background: var(--fluent-color-fill-control-hover);
}
.mcp-list-item--active {
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
  box-shadow: inset 3px 0 0 var(--fluent-color-stroke-accent-soft);
}
.mcp-detail {
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}
.mcp-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
.mcp-json {
  font-family: var(--fluent-font-family-mono);
  resize: vertical;
}
.mcp-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
}
</style>
