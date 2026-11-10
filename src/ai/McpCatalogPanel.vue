<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/** 公共 MCP 目录：横向主从布局；添加到指定单个项目。 */
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
  aiAgentTargetsList,
  aiHarvestMcpImport,
  aiHarvestMcpScan,
  aiMcpCatalogImport,
  aiMcpCatalogList,
  aiMcpCatalogRemove,
  aiMcpCatalogUpsert,
  aiMcpDeploy,
  aiMcpUndeploy,
} from "./client";
import { openConfirmDialog } from "../state/confirmDialog";
import { toastSuccess } from "../state/toast";
import { listProjectOptions, type ProjectOption } from "./projectOptions";
import type {
  AgentTarget,
  AiConfigDocument,
  HarvestMcpCandidate,
  McpCatalogEntry,
} from "./types";

const { t } = useI18n();

const document = ref<AiConfigDocument | null>(null);
const error = ref<string | null>(null);
const loading = ref(false);
const selectedId = ref<string | null>(null);
const filterText = ref("");
const importPath = ref("");
const draftName = ref("");
const draftId = ref("");
const draftJson = ref('{\n  "command": "npx",\n  "args": ["-y", "example"]\n}');
const lastNote = ref<string | null>(null);
const agents = ref<AgentTarget[]>([]);
const harvestAgentId = ref("");
const harvestCandidates = ref<HarvestMcpCandidate[]>([]);
const harvestSelected = ref<Record<string, boolean>>({});
const showTools = ref(false);
const projects = ref<ProjectOption[]>([]);
const targetProjectPath = ref("");

const ERROR_KEYS: Record<string, string> = {
  AI_MCP_ID_EMPTY: "ai.mcp.errorIdEmpty",
  AI_MCP_ID_INVALID: "ai.mcp.errorIdInvalid",
  AI_MCP_UNKNOWN: "ai.mcp.errorUnknown",
  AI_MCP_DEPLOY_NO_ENABLED: "ai.mcp.errorDeployNoEnabled",
  AI_MCP_TARGET_INVALID: "ai.mcp.errorTargetInvalid",
  AI_MCP_IMPORT_INVALID: "ai.mcp.errorImportInvalid",
  AI_HARVEST_AGENT_UNKNOWN: "ai.harvest.errorAgentUnknown",
  AI_HARVEST_PATH_INVALID: "ai.harvest.errorPath",
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

const harvestPickCount = computed(
  () => Object.values(harvestSelected.value).filter(Boolean).length,
);

const filteredEntries = computed(() => {
  const q = filterText.value.trim().toLowerCase();
  const list = document.value?.mcpCatalog ?? [];
  if (!q) return list;
  return list.filter(
    (e) =>
      e.id.toLowerCase().includes(q) ||
      e.displayName.toLowerCase().includes(q) ||
      e.serverJson.toLowerCase().includes(q),
  );
});

watch(selected, (entry) => {
  if (!entry) return;
  draftId.value = entry.id;
  draftName.value = entry.displayName;
  try {
    draftJson.value = JSON.stringify(JSON.parse(entry.serverJson), null, 2);
  } catch {
    draftJson.value = entry.serverJson;
  }
});

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    const [mcpDoc, agentDoc, projectList] = await Promise.all([
      aiMcpCatalogList(),
      aiAgentTargetsList(),
      listProjectOptions(),
    ]);
    document.value = mcpDoc;
    agents.value = agentDoc.agentTargets;
    projects.value = projectList.options;
    if (!targetProjectPath.value) {
      const active = projectList.options.find((p) => p.slug === projectList.activeSlug);
      targetProjectPath.value = active?.path ?? projectList.options[0]?.path ?? "";
    }
    if (!harvestAgentId.value && agents.value.length) {
      harvestAgentId.value = agents.value[0]?.id ?? "";
    }
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
    document.value = await aiMcpCatalogUpsert({
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
  const result = await openConfirmDialog({
    title: t("ai.mcp.confirmRemoveTitle"),
    message: t("ai.mcp.confirmRemoveMessage", { name: selected.value.displayName }),
    confirmLabel: t("common.remove"),
    cancelLabel: t("common.cancel"),
    danger: true,
  });
  if (!result.confirmed) return;
  error.value = null;
  try {
    document.value = await aiMcpCatalogRemove(selected.value.id);
    selectedId.value = document.value.mcpCatalog?.[0]?.id ?? null;
    toastSuccess(t("ai.mcp.noticeRemoved"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function deployGlobal() {
  if (!selected.value) return;
  error.value = null;
  lastNote.value = null;
  try {
    const report = await aiMcpDeploy(selected.value.id, null);
    lastNote.value = t("ai.mcp.noticeDeployed", {
      count: report.written.length,
      skipped: report.skipped.length,
    });
    toastSuccess(lastNote.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function deployToSelectedProject() {
  if (!selected.value) return;
  const ws = targetProjectPath.value.trim();
  if (!ws) {
    error.value = t("ai.mcp.errorPickProject");
    return;
  }
  error.value = null;
  lastNote.value = null;
  try {
    const report = await aiMcpDeploy(selected.value.id, ws);
    const project =
      projects.value.find((p) => p.path === ws)?.displayName ?? ws;
    lastNote.value = t("ai.mcp.noticeDeployedToProject", {
      project,
      count: report.written.length,
      skipped: report.skipped.length,
    });
    toastSuccess(lastNote.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function undeployFromSelectedProject() {
  if (!selected.value) return;
  const ws = targetProjectPath.value.trim();
  if (!ws) {
    error.value = t("ai.mcp.errorPickProject");
    return;
  }
  error.value = null;
  try {
    const report = await aiMcpUndeploy(selected.value.id, ws);
    lastNote.value = t("ai.mcp.noticeUndeployed", { count: report.written.length });
    toastSuccess(lastNote.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function importMcpJson() {
  const path = importPath.value.trim();
  if (!path) {
    error.value = t("ai.mcp.errorImportInvalid");
    return;
  }
  error.value = null;
  lastNote.value = null;
  try {
    const [next, report] = await aiMcpCatalogImport(path);
    document.value = next;
    if (!selectedId.value && next.mcpCatalog?.length) {
      selectedId.value = next.mcpCatalog[0]?.id ?? null;
    }
    lastNote.value = t("ai.mcp.noticeImported", {
      added: report.added.length,
      skipped: report.skipped.length,
    });
    toastSuccess(lastNote.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

function selectEntry(entry: McpCatalogEntry) {
  selectedId.value = entry.id;
}

function startNew() {
  selectedId.value = null;
  draftId.value = "";
  draftName.value = "";
  draftJson.value = '{\n  "command": "npx",\n  "args": ["-y", "example"]\n}';
}

async function scanHarvest() {
  if (!harvestAgentId.value) return;
  error.value = null;
  try {
    const report = await aiHarvestMcpScan(harvestAgentId.value);
    harvestCandidates.value = report.candidates;
    const selectedMap: Record<string, boolean> = {};
    for (const c of report.candidates) {
      selectedMap[c.id] = !c.alreadyInCatalog;
    }
    harvestSelected.value = selectedMap;
  } catch (err) {
    error.value = mapError(err);
  }
}

async function importHarvest() {
  const picks = harvestCandidates.value.filter((c) => harvestSelected.value[c.id]);
  if (!picks.length) return;
  error.value = null;
  try {
    const [next, report] = await aiHarvestMcpImport(picks);
    document.value = next;
    lastNote.value = t("ai.harvest.noticeMcpImported", {
      added: report.added.length,
      skipped: report.skipped.length,
    });
    toastSuccess(lastNote.value);
    await scanHarvest();
  } catch (err) {
    error.value = mapError(err);
  }
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="mcp-panel" data-ai-mcp-panel>
    <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>

    <div class="mcp-toolbar">
      <input
        v-model="filterText"
        class="fluent-control mcp-filter"
        type="search"
        :placeholder="t('ai.mcp.filterPlaceholder')"
      />
      <button type="button" class="fluent-control" @click="startNew">
        {{ t("ai.mcp.add") }}
      </button>
      <button type="button" class="fluent-control" @click="showTools = !showTools">
        {{ showTools ? t("ai.mcp.hideTools") : t("ai.mcp.showTools") }}
      </button>
    </div>

    <div v-if="showTools" class="fluent-surface mcp-tools">
      <div class="mcp-tools-grid">
        <div class="mcp-tool-col">
          <h3 class="mcp-subhead">{{ t("ai.harvest.mcpTitle") }}</h3>
          <div class="mcp-row">
            <select v-model="harvestAgentId" class="fluent-control mcp-grow">
              <option v-for="agent in agents" :key="agent.id" :value="agent.id">
                {{ agent.displayName }}
              </option>
            </select>
            <button type="button" class="fluent-control" @click="scanHarvest">
              {{ t("ai.harvest.scan") }}
            </button>
            <button
              type="button"
              class="fluent-control fluent-control--accent"
              :disabled="!harvestPickCount"
              @click="importHarvest"
            >
              {{ t("ai.harvest.importSelected", { count: harvestPickCount }) }}
            </button>
          </div>
          <div v-if="harvestCandidates.length" class="mcp-harvest-scroll">
            <label v-for="c in harvestCandidates" :key="c.id" class="mcp-check">
              <input
                v-model="harvestSelected[c.id]"
                type="checkbox"
                :disabled="c.alreadyInCatalog"
              />
              <span>{{ c.displayName }}</span>
            </label>
          </div>
        </div>
        <div class="mcp-tool-col">
          <h3 class="mcp-subhead">{{ t("ai.mcp.import") }}</h3>
          <div class="mcp-row">
            <input
              v-model="importPath"
              class="fluent-control mcp-grow"
              type="text"
              :placeholder="t('ai.mcp.importPathHint')"
            />
            <button type="button" class="fluent-control" @click="importMcpJson">
              {{ t("ai.mcp.import") }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <div class="mcp-split">
      <aside class="fluent-surface mcp-list-pane">
        <ul
          v-if="filteredEntries.length"
          class="mcp-list"
          role="listbox"
          :aria-label="t('ai.sections.mcp.title')"
        >
          <li
            v-for="entry in filteredEntries"
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
        </ul>
        <p v-else class="mcp-meta">{{ t("ai.mcp.empty") }}</p>
      </aside>

      <article class="fluent-surface mcp-detail">
        <label class="mcp-field">
          <span>{{ t("ai.mcp.fieldName") }}</span>
          <input v-model="draftName" class="fluent-control" type="text" />
        </label>
        <label class="mcp-field">
          <span>{{ t("ai.mcp.fieldId") }}</span>
          <input v-model="draftId" class="fluent-control" type="text" />
        </label>
        <label class="mcp-field mcp-field--grow">
          <span>{{ t("ai.mcp.fieldJson") }}</span>
          <textarea v-model="draftJson" class="fluent-control mcp-json" rows="10" />
        </label>
        <div class="mcp-actions">
          <button type="button" class="fluent-control fluent-control--accent" @click="saveEntry">
            {{ t("ai.mcp.save") }}
          </button>
          <button type="button" class="fluent-control" :disabled="!selected" @click="deployGlobal">
            {{ t("ai.mcp.deployGlobal") }}
          </button>
          <label class="mcp-field mcp-project-pick">
            <span>{{ t("ai.mcp.targetProject") }}</span>
            <select v-model="targetProjectPath" class="fluent-control">
              <option value="" disabled>{{ t("ai.mcp.pickProject") }}</option>
              <option v-for="p in projects" :key="p.path" :value="p.path">
                {{ p.displayName }}
              </option>
            </select>
          </label>
          <button
            type="button"
            class="fluent-control"
            :disabled="!selected || !targetProjectPath"
            @click="deployToSelectedProject"
          >
            {{ t("ai.mcp.addToProject") }}
          </button>
          <button
            type="button"
            class="fluent-control"
            :disabled="!selected || !targetProjectPath"
            @click="undeployFromSelectedProject"
          >
            {{ t("ai.mcp.removeFromProject") }}
          </button>
          <button type="button" class="fluent-control" :disabled="!selected" @click="removeSelected">
            {{ t("common.remove") }}
          </button>
        </div>
        <p class="mcp-hint">{{ t("ai.mcp.projectDefaultHint") }}</p>
        <p v-if="lastNote" class="mcp-hint">{{ lastNote }}</p>
      </article>
    </div>
  </div>
</template>

<style scoped>
.mcp-panel {
  margin-top: var(--fluent-space-md);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  min-width: 0;
}
.mcp-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
  align-items: center;
}
.mcp-filter {
  flex: 1;
  min-width: 12rem;
}
.mcp-tools {
  padding: var(--fluent-space-md);
}
.mcp-tools-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--fluent-space-md);
}
@media (max-width: 900px) {
  .mcp-tools-grid {
    grid-template-columns: 1fr;
  }
}
.mcp-tool-col {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  min-width: 0;
}
.mcp-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
}
.mcp-grow {
  flex: 1;
  min-width: 8rem;
}
.mcp-harvest-scroll {
  max-height: 8rem;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.mcp-check {
  display: flex;
  gap: var(--fluent-space-sm);
  align-items: center;
  font-size: var(--fluent-font-size-caption);
}
.mcp-split {
  display: grid;
  grid-template-columns: minmax(12rem, 16rem) minmax(0, 1fr);
  gap: var(--fluent-space-md);
  min-height: 22rem;
}
@media (max-width: 720px) {
  .mcp-split {
    grid-template-columns: 1fr;
  }
}
.mcp-list-pane {
  padding: var(--fluent-space-sm);
  max-height: 32rem;
  overflow: auto;
}
.mcp-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.mcp-list-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-radius: var(--fluent-radius-md);
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
  min-width: 0;
}
.mcp-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
.mcp-field--grow {
  flex: 1;
}
.mcp-json {
  font-family: var(--fluent-font-family-mono);
  resize: vertical;
  min-height: 10rem;
}
.mcp-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
  align-items: flex-end;
}
.mcp-project-pick {
  min-width: 14rem;
  flex: 1;
}
.mcp-hint,
.mcp-meta,
.mcp-subhead {
  margin: 0;
}
.mcp-hint,
.mcp-meta {
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}
.mcp-subhead {
  font-size: var(--fluent-font-size-body);
}
</style>
