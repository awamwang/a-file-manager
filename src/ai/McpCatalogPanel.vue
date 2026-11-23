<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/** 公共 MCP 目录：导入区按来源分段；多 Agent 一次扫描合并；添加到指定单个项目。 */
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
import {
  mergeMcpCandidates,
  toMcpCandidate,
  type DuplicateReason,
  type HarvestMcpCandidateView,
} from "./harvestDedupe";
import { openConfirmDialog } from "../state/confirmDialog";
import { toastSuccess } from "../state/toast";
import { listProjectOptions, type ProjectOption } from "./projectOptions";
import type {
  AgentTarget,
  AiConfigDocument,
  HarvestMcpScanReport,
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
const agents = ref<AgentTarget[]>([]);
const harvestAgentIds = ref<string[]>([]);
const harvestCandidates = ref<HarvestMcpCandidateView[]>([]);
const harvestSelected = ref<Record<string, boolean>>({});
const harvestRename = ref<Record<string, string>>({});
const harvestScanning = ref(false);
const sourceOptions = [
  { source: "agent", labelKey: "ai.mcp.fromAgent" },
  { source: "file", labelKey: "ai.mcp.fromFile" },
] as const;
type HarvestSource = (typeof sourceOptions)[number]["source"];
const harvestSource = ref<HarvestSource>("agent");

/** Agent Target 是源头：只有中央 enabled 的 Agent 才能作为导入来源。 */
const enabledAgents = computed(() => agents.value.filter((a) => a.enabled));
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
  AI_HARVEST_AGENT_NOT_ENABLED: "ai.harvest.errorAgentNotEnabled",
  AI_HARVEST_PATH: "ai.harvest.errorPath",
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

function duplicateLabel(reason: DuplicateReason): string {
  if (reason === "existing-id") return t("ai.harvest.reasonExistingId");
  if (reason === "existing-server") return t("ai.harvest.reasonExistingServer");
  if (reason === "name-conflict") return t("ai.harvest.reasonNameConflict");
  return "";
}

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

function deriveSlugFromJson(json: string): { command: string; id: string } | null {
  try {
    const o = JSON.parse(json) as Record<string, unknown>;
    const command = typeof o.command === "string" ? o.command : "";
    if (!command) return null;
    const args = Array.isArray(o.args)
      ? o.args.filter((a): a is string => typeof a === "string")
      : [];
    const meaningful = args.find((a) => /^[a-z@][\w@.-]+$/i.test(a)) ?? command;
    const id = meaningful
      .toLowerCase()
      .replace(/^@/, "")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
    return { command, id };
  } catch {
    return null;
  }
}

// Auto-derive id/displayName from the pasted server JSON for new entries, so the
// free-text id field is usually left untouched (AIA — MCP first-time-user lens).
watch(draftJson, (json) => {
  if (selectedId.value) return;
  if (draftId.value) return;
  const d = deriveSlugFromJson(json);
  if (d?.id) {
    draftId.value = d.id;
    if (!draftName.value) draftName.value = d.command;
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
    const validIds = enabledAgents.value.map((a) => a.id);
    const stillValid = harvestAgentIds.value.filter((id) => validIds.includes(id));
    harvestAgentIds.value = stillValid.length ? stillValid : validIds;
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
  try {
    const report = await aiMcpDeploy(selected.value.id, null);
    toastSuccess(
      t("ai.mcp.noticeDeployed", {
        count: report.written.length,
        skipped: report.skipped.length,
      }),
    );
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
  try {
    const report = await aiMcpDeploy(selected.value.id, ws);
    const project =
      projects.value.find((p) => p.path === ws)?.displayName ?? ws;
    toastSuccess(
      t("ai.mcp.noticeDeployedToProject", {
        project,
        count: report.written.length,
        skipped: report.skipped.length,
      }),
    );
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
    toastSuccess(t("ai.mcp.noticeUndeployed", { count: report.written.length }));
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
  try {
    const [next, report] = await aiMcpCatalogImport(path);
    document.value = next;
    if (!selectedId.value && next.mcpCatalog?.length) {
      selectedId.value = next.mcpCatalog[0]?.id ?? null;
    }
    toastSuccess(
      t("ai.mcp.noticeImported", {
        added: report.added.length,
        skipped: report.skipped.length,
      }),
    );
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

function toggleHarvestAgent(id: string) {
  const set = new Set(harvestAgentIds.value);
  if (set.has(id)) set.delete(id);
  else set.add(id);
  harvestAgentIds.value = [...set];
}

async function scanHarvest() {
  if (!harvestAgentIds.value.length) return;
  error.value = null;
  harvestScanning.value = true;
  try {
    const results = await Promise.allSettled(
      harvestAgentIds.value.map((id) => aiHarvestMcpScan(id)),
    );
    const reports: HarvestMcpScanReport[] = [];
    let failed = 0;
    for (const result of results) {
      if (result.status === "fulfilled") reports.push(result.value);
      else failed += 1;
    }
    harvestCandidates.value = mergeMcpCandidates(reports, document.value?.mcpCatalog ?? []);
    const picked: Record<string, boolean> = {};
    harvestRename.value = {};
    harvestCandidates.value.forEach((c, index) => {
      picked[String(index)] = c.duplicateReason === "none";
    });
    harvestSelected.value = picked;
    if (failed) error.value = t("ai.harvest.partialFailure", { count: failed });
  } catch (err) {
    error.value = mapError(err);
  } finally {
    harvestScanning.value = false;
  }
}

async function importHarvest() {
  const picks = harvestCandidates.value.filter((_c, index) => harvestSelected.value[String(index)]);
  if (!picks.length) return;
  error.value = null;
  try {
    const [next, report] = await aiHarvestMcpImport(
      picks.map((c, index) => toMcpCandidate(c, harvestRename.value[String(index)])),
    );
    document.value = next;
    toastSuccess(
      t("ai.harvest.noticeMcpImported", {
        added: report.added.length,
        skipped: report.skipped.length,
      }),
    );
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
    <p v-if="loading" class="fluent-hint">{{ t("common.loading") }}</p>
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
      <h3 class="mcp-subhead">{{ t("ai.mcp.toolsTitle") }}</h3>
      <div class="harvest-segment" role="tablist">
        <button
          v-for="opt in sourceOptions"
          :key="opt.source"
          type="button"
          class="fluent-control harvest-segment-item"
          :class="{ 'harvest-segment-item--active': harvestSource === opt.source }"
          :aria-selected="harvestSource === opt.source"
          role="tab"
          @click="harvestSource = opt.source"
        >
          {{ t(opt.labelKey) }}
        </button>
      </div>

      <div v-if="harvestSource === 'agent'" class="harvest-block">
        <div class="harvest-agent-head">
          <span class="harvest-label">{{ t("ai.harvest.agentsLabel") }}</span>
          <span class="mcp-meta">{{ t("ai.harvest.multiSelectHint") }}</span>
        </div>
        <p v-if="!enabledAgents.length" class="mcp-meta">
          {{ t("ai.harvest.noSources") }}
        </p>
        <div v-else class="harvest-agents">
          <label v-for="agent in enabledAgents" :key="agent.id" class="harvest-agent-chip">
            <input
              type="checkbox"
              :checked="harvestAgentIds.includes(agent.id)"
              @change="toggleHarvestAgent(agent.id)"
            />
            <span>{{ agent.displayName }}</span>
          </label>
        </div>
        <div class="mcp-row harvest-scan-row">
          <button
            type="button"
            class="fluent-control"
            :disabled="!harvestAgentIds.length || harvestScanning"
            @click="scanHarvest"
          >
            {{ harvestScanning ? t("common.loading") : t("ai.harvest.scan") }}
          </button>
          <button
            type="button"
            class="fluent-control"
            @click="harvestAgentIds = enabledAgents.map((a) => a.id)"
          >
            {{ t("ai.harvest.allAgents") }}
          </button>
          <button type="button" class="fluent-control" @click="harvestAgentIds = []">
            {{ t("ai.harvest.clearAgents") }}
          </button>
        </div>

        <p v-if="!harvestCandidates.length" class="mcp-meta harvest-empty">
          {{ t("ai.harvest.scanEmpty") }}
        </p>
        <div v-else class="harvest-candidates">
          <div v-for="(c, index) in harvestCandidates" :key="`${c.id}-${index}`" class="harvest-candidate">
            <label class="harvest-candidate-main">
              <input v-model="harvestSelected[String(index)]" type="checkbox" />
              <strong>{{ c.id }}</strong>
              <span class="mcp-meta">{{ t("ai.harvest.fromCount", { count: c.sources.length }) }}</span>
            </label>
            <span v-if="c.duplicateReason !== 'none'" class="harvest-reason">
              {{ duplicateLabel(c.duplicateReason) }}
            </span>
            <input
              v-if="c.duplicateReason !== 'none'"
              v-model="harvestRename[String(index)]"
              class="fluent-control harvest-rename"
              type="text"
              :placeholder="t('ai.harvest.renamePlaceholder')"
            />
          </div>
        </div>
        <div class="harvest-footer">
          <button type="button" class="fluent-control" @click="showTools = false">
            {{ t("common.cancel") }}
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
      </div>

      <div v-else class="harvest-block">
        <div class="harvest-file-row">
          <input
            v-model="importPath"
            class="fluent-control"
            type="text"
            :placeholder="t('ai.mcp.importPathHint')"
          />
          <button
            type="button"
            class="fluent-control fluent-control--accent"
            :disabled="!importPath.trim()"
            @click="importMcpJson"
          >
            {{ t("ai.mcp.importFile") }}
          </button>
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
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}
.harvest-segment {
  display: inline-flex;
  gap: var(--fluent-space-xs);
  align-self: flex-start;
}
.harvest-segment-item--active {
  background: var(--fluent-color-fill-selected);
  color: var(--fluent-color-stroke-accent-soft);
  font-weight: var(--fluent-font-weight-semibold);
}
.harvest-block {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  padding-top: var(--fluent-space-xs);
}
.harvest-agent-head {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
  align-items: baseline;
}
.harvest-label {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
.harvest-agents {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
}
.harvest-agent-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
  font-size: var(--fluent-font-size-caption);
  cursor: pointer;
}
.harvest-scan-row {
  align-items: center;
}
.harvest-candidates {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 11rem;
  overflow: auto;
}
.harvest-candidate {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
  align-items: center;
  padding: 4px 6px;
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
}
.harvest-candidate-main {
  display: flex;
  flex: 1;
  min-width: 10rem;
  gap: var(--fluent-space-sm);
  align-items: center;
  cursor: pointer;
}
.harvest-reason {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
.harvest-rename {
  min-width: 10rem;
}
.harvest-file-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
}
.harvest-file-row input {
  flex: 1;
  min-width: 12rem;
}
.harvest-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--fluent-space-sm);
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
  -webkit-user-select: text;
  user-select: text;
}
.mcp-subhead {
  font-size: var(--fluent-font-size-body);
}
</style>
