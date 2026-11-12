<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import FluentIcon from "../../components/icons/FluentIcon.vue";
import { openConfirmDialog } from "../../state/confirmDialog";
import { selectedPaths } from "../../state/navigatorSelection";
import { toastSuccess } from "../../state/toast";
import CapabilityForm from "./CapabilityForm.vue";
import {
  asRecord,
  callCapability,
  ensureGateway,
  formatPayload,
  gatewayStatus,
  getConfig,
  invokeErrorMessage,
  isDestructiveBlocked,
  isToolRuntimeDisabled,
  listCapabilities,
  listTools,
  parseCapabilityList,
  parseToolList,
  setConfig,
  type GatewayStatus,
  type JsonSchema,
  type RuntimeCapability,
  type RuntimeTool,
  type ToolRuntimeConfig,
} from "./client";

const { t } = useI18n();

const DEFAULT_BASE_URL = "http://127.0.0.1:8766";

const status = ref<GatewayStatus | null>(null);
const tools = ref<RuntimeTool[]>([]);
const capabilities = ref<RuntimeCapability[]>([]);
const selectedToolId = ref<string | null>(null);
const selectedCapabilityId = ref<string | null>(null);
const formValues = ref<Record<string, unknown>>({});
const filterText = ref("");
const error = ref<string | null>(null);
const loadingStatus = ref(false);
const loadingTools = ref(false);
const loadingCaps = ref(false);
const calling = ref(false);
const attaching = ref(false);
const previewPayload = ref<unknown>(null);
const resultPayload = ref<unknown>(null);
const awaitingExecute = ref(false);
const runtimeConfig = ref<ToolRuntimeConfig | null>(null);
const baseUrlDraft = ref(DEFAULT_BASE_URL);
const savingBaseUrl = ref(false);

const selectedTool = computed(
  () => tools.value.find((item) => item.id === selectedToolId.value) ?? null,
);
const selectedCapability = computed(
  () =>
    capabilities.value.find((item) => item.capabilityId === selectedCapabilityId.value) ??
    null,
);
const filteredTools = computed(() => {
  const needle = filterText.value.trim().toLowerCase();
  if (!needle) return tools.value;
  return tools.value.filter((item) => {
    const haystack = `${item.name} ${item.id} ${item.kind} ${item.description}`.toLowerCase();
    return haystack.includes(needle);
  });
});
const statusVersion = computed(() => {
  const version = status.value?.version?.trim();
  return version ? ` · v${version}` : "";
});

onMounted(() => {
  void (async () => {
    await loadConfig();
    await refreshStatus({ loadToolsIfReachable: true });
  })();
});

async function loadConfig(): Promise<void> {
  try {
    runtimeConfig.value = await getConfig();
    baseUrlDraft.value = runtimeConfig.value.baseUrl || DEFAULT_BASE_URL;
  } catch (err) {
    error.value = friendlyError(err);
  }
}

async function saveBaseUrl(): Promise<void> {
  const current = runtimeConfig.value ?? {
    baseUrl: DEFAULT_BASE_URL,
    preferAttach: true,
    autoStartSidecar: false,
    lastSidecarPid: null,
  };
  savingBaseUrl.value = true;
  error.value = null;
  try {
    runtimeConfig.value = await setConfig({
      ...current,
      baseUrl: baseUrlDraft.value.trim() || DEFAULT_BASE_URL,
    });
    baseUrlDraft.value = runtimeConfig.value.baseUrl;
    toastSuccess(t("toolRuntime.gatewayUrlSaved"));
    await refreshStatus({ loadToolsIfReachable: true });
  } catch (err) {
    error.value = friendlyError(err);
  } finally {
    savingBaseUrl.value = false;
  }
}

async function refreshStatus(options?: { loadToolsIfReachable?: boolean }): Promise<void> {
  loadingStatus.value = true;
  error.value = null;
  try {
    status.value = await gatewayStatus();
    if (options?.loadToolsIfReachable && status.value.reachable) {
      await loadTools();
    }
  } catch (err) {
    error.value = friendlyError(err);
  } finally {
    loadingStatus.value = false;
  }
}

async function attachOrStart(): Promise<void> {
  attaching.value = true;
  error.value = null;
  try {
    const result = await ensureGateway();
    status.value = await gatewayStatus();
    if (result.spawned) {
      toastSuccess(t("toolRuntime.spawned", { pid: result.pid }));
    } else {
      toastSuccess(t("toolRuntime.attached"));
    }
    if (status.value.reachable) {
      await loadTools();
    }
  } catch (err) {
    error.value = friendlyError(err);
  } finally {
    attaching.value = false;
  }
}

async function loadTools(): Promise<void> {
  loadingTools.value = true;
  error.value = null;
  try {
    tools.value = parseToolList(await listTools());
  } catch (err) {
    tools.value = [];
    error.value = friendlyError(err);
  } finally {
    loadingTools.value = false;
  }
}

async function selectTool(tool: RuntimeTool): Promise<void> {
  if (selectedToolId.value === tool.id) return;
  selectedToolId.value = tool.id;
  selectedCapabilityId.value = null;
  capabilities.value = [];
  formValues.value = {};
  previewPayload.value = null;
  resultPayload.value = null;
  awaitingExecute.value = false;
  await loadCapabilities(false);
}

async function loadCapabilities(refresh: boolean): Promise<void> {
  const toolId = selectedToolId.value;
  if (!toolId) return;
  loadingCaps.value = true;
  error.value = null;
  try {
    capabilities.value = parseCapabilityList(await listCapabilities(toolId, refresh));
  } catch (err) {
    capabilities.value = [];
    error.value = friendlyError(err);
  } finally {
    loadingCaps.value = false;
  }
}

function selectCapability(capability: RuntimeCapability): void {
  selectedCapabilityId.value = capability.capabilityId;
  formValues.value = prefillArguments(capability.inputSchema);
  previewPayload.value = null;
  resultPayload.value = null;
  awaitingExecute.value = false;
}

function prefillArguments(schema: JsonSchema | null): Record<string, unknown> {
  const values: Record<string, unknown> = {};
  const properties = schema?.properties;
  const paths = selectedPaths.value;
  if (properties?.path && paths[0]) {
    values.path = paths[0];
  }
  if (properties?.paths && paths.length) {
    values.paths = [...paths];
  }
  if (!properties) return values;
  for (const [key, prop] of Object.entries(properties)) {
    if (values[key] !== undefined) continue;
    if (prop.default !== undefined) values[key] = prop.default;
  }
  return values;
}

function isReadCapability(capability: RuntimeCapability | null): boolean {
  return capability?.risk === "read";
}

async function runPrimary(): Promise<void> {
  const tool = selectedTool.value;
  const capability = selectedCapability.value;
  if (!tool || !capability) return;
  calling.value = true;
  error.value = null;
  resultPayload.value = null;
  try {
    if (isReadCapability(capability)) {
      resultPayload.value = await invokeCall(true, false);
      awaitingExecute.value = false;
      previewPayload.value = null;
      return;
    }
    previewPayload.value = await invokeCall(false, false);
    awaitingExecute.value = true;
  } catch (err) {
    error.value = friendlyError(err);
  } finally {
    calling.value = false;
  }
}

async function confirmExecute(): Promise<void> {
  const tool = selectedTool.value;
  const capability = selectedCapability.value;
  if (!tool || !capability) return;
  const previewRisk = asRecord(previewPayload.value)?.risk;
  const needsDestructive =
    capability.risk === "destructive" || previewRisk === "destructive";
  let allowDestructive = false;
  if (needsDestructive) {
    const confirmed = await confirmDestructive();
    if (!confirmed) return;
    allowDestructive = true;
  }
  calling.value = true;
  error.value = null;
  try {
    resultPayload.value = await invokeCall(true, allowDestructive);
    awaitingExecute.value = false;
  } catch (err) {
    const message = invokeErrorMessage(err);
    if (!allowDestructive && isDestructiveBlocked(message)) {
      const confirmed = await confirmDestructive();
      if (confirmed) {
        try {
          resultPayload.value = await invokeCall(true, true);
          awaitingExecute.value = false;
          return;
        } catch (retryErr) {
          error.value = friendlyError(retryErr);
          return;
        }
      }
    }
    error.value = friendlyError(err);
  } finally {
    calling.value = false;
  }
}

async function invokeCall(execute: boolean, allowDestructive: boolean): Promise<unknown> {
  const tool = selectedTool.value;
  const capability = selectedCapability.value;
  if (!tool || !capability) {
    throw new Error(t("toolRuntime.selectCapability"));
  }
  return callCapability({
    toolId: tool.id,
    capabilityId: capability.capabilityId,
    arguments: formValues.value,
    execute,
    allowDestructive: allowDestructive || undefined,
  });
}

async function confirmDestructive(): Promise<boolean> {
  const result = await openConfirmDialog({
    title: t("toolRuntime.destructiveTitle"),
    message: t("toolRuntime.destructiveMessage"),
    confirmLabel: t("toolRuntime.execute"),
    danger: true,
  });
  return result.confirmed;
}

function friendlyError(err: unknown): string {
  const message = invokeErrorMessage(err);
  if (isToolRuntimeDisabled(message)) return t("toolRuntime.disabled");
  return message;
}

function riskLabel(risk: string): string {
  if (risk === "read") return t("toolRuntime.riskRead");
  if (risk === "destructive") return t("toolRuntime.riskDestructive");
  if (risk === "write") return t("toolRuntime.riskWrite");
  return risk;
}

function riskChipClass(risk: string): string {
  if (risk === "read") return "fluent-chip--ok";
  if (risk === "destructive") return "fluent-chip--down";
  return "fluent-chip--stub";
}
</script>

<template>
  <section class="fluent-page runtime">
    <header class="fluent-page-header">
      <h1 class="fluent-page-title">{{ t("toolRuntime.title") }}</h1>
      <p class="fluent-page-meta">{{ t("toolRuntime.meta") }}</p>
    </header>

    <div class="fluent-surface card status-card">
      <div class="row-head">
        <h2 class="section-title">
          <span class="fluent-icon-wash"><FluentIcon name="cloud" size="md" /></span>
          {{ t("toolRuntime.title") }}
        </h2>
        <div class="actions">
          <button
            class="fluent-control fluent-control--accent"
            type="button"
            :disabled="attaching"
            @click="attachOrStart"
          >
            {{ attaching ? t("toolRuntime.attaching") : t("toolRuntime.attachStart") }}
          </button>
          <button
            class="fluent-control"
            type="button"
            :disabled="loadingStatus || loadingTools"
            :title="t('common.refresh')"
            :aria-label="t('common.refresh')"
            @click="refreshStatus({ loadToolsIfReachable: true })"
          >
            <FluentIcon name="refresh" size="sm" />
            {{ t("common.refresh") }}
          </button>
        </div>
      </div>
      <p
        v-if="status"
        class="fluent-banner"
        :class="status.reachable ? 'fluent-banner--success' : 'fluent-banner--warning'"
      >
        <span
          class="fluent-chip status-chip"
          :class="status.reachable ? 'fluent-chip--ok' : 'fluent-chip--down'"
        >{{ status.reachable ? t("common.yes") : t("common.no") }}</span>
        {{
          status.reachable
            ? t("toolRuntime.statusReachable", { version: statusVersion })
            : t("toolRuntime.statusUnreachable", { detail: status.detail })
        }}
      </p>
      <p v-else-if="loadingStatus" class="fluent-banner">{{ t("common.loading") }}</p>
      <div class="gateway-url-row">
        <label class="gateway-url-label" for="tool-runtime-base-url">{{
          t("toolRuntime.gatewayUrlLabel")
        }}</label>
        <input
          id="tool-runtime-base-url"
          v-model="baseUrlDraft"
          class="fluent-control gateway-url-input"
          type="text"
          spellcheck="false"
          autocomplete="off"
          :disabled="savingBaseUrl"
          :placeholder="t('toolRuntime.gatewayUrlPlaceholder')"
          @keydown.enter.prevent="saveBaseUrl"
        />
        <button
          class="fluent-control fluent-control--accent"
          type="button"
          :disabled="savingBaseUrl"
          @click="saveBaseUrl"
        >
          {{
            savingBaseUrl
              ? t("toolRuntime.gatewayUrlSaving")
              : t("toolRuntime.gatewayUrlSave")
          }}
        </button>
      </div>
    </div>

    <p v-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>

    <div class="layout">
      <section class="fluent-surface card pane">
        <div class="row-head">
          <h2 class="section-title">
            <span class="fluent-icon-wash"><FluentIcon name="puzzle" size="md" /></span>
            {{ t("toolRuntime.tools") }}
          </h2>
        </div>
        <input
          v-model="filterText"
          class="fluent-control"
          type="search"
          spellcheck="false"
          :placeholder="t('toolRuntime.filterPlaceholder')"
        />
        <p v-if="loadingTools" class="fluent-page-meta">{{ t("common.loading") }}</p>
        <p v-else-if="!filteredTools.length" class="fluent-page-meta">
          {{ t("toolRuntime.noTools") }}
        </p>
        <div v-else class="list">
          <button
            v-for="tool in filteredTools"
            :key="tool.id"
            type="button"
            class="fluent-list-row row"
            :class="{ 'row--active': tool.id === selectedToolId }"
            @click="selectTool(tool)"
          >
            <strong class="name">{{ tool.name }}</strong>
            <span class="fluent-chip status-chip fluent-chip--stub">{{ tool.kind }}</span>
            <span class="detail">{{ tool.description || tool.id }}</span>
          </button>
        </div>
      </section>

      <section class="fluent-surface card pane">
        <div class="row-head">
          <h2 class="section-title">
            <span class="fluent-icon-wash"><FluentIcon name="flash" size="md" /></span>
            {{ t("toolRuntime.capabilities") }}
          </h2>
          <button
            class="fluent-control"
            type="button"
            :disabled="!selectedToolId || loadingCaps"
            @click="loadCapabilities(true)"
          >
            {{ t("toolRuntime.refreshCapabilities") }}
          </button>
        </div>

        <p v-if="!selectedTool" class="fluent-page-meta">{{ t("toolRuntime.selectTool") }}</p>
        <p v-else-if="loadingCaps" class="fluent-page-meta">{{ t("common.loading") }}</p>
        <p v-else-if="!capabilities.length" class="fluent-page-meta">
          {{ t("toolRuntime.noCapabilities") }}
        </p>
        <div v-else class="list">
          <button
            v-for="capability in capabilities"
            :key="capability.capabilityId"
            type="button"
            class="fluent-list-row row"
            :class="{ 'row--active': capability.capabilityId === selectedCapabilityId }"
            @click="selectCapability(capability)"
          >
            <strong class="name">{{ capability.name }}</strong>
            <span class="fluent-chip status-chip" :class="riskChipClass(capability.risk)">
              {{ riskLabel(capability.risk) }}
            </span>
            <span class="detail">{{ capability.description || capability.capabilityId }}</span>
          </button>
        </div>

        <template v-if="selectedCapability">
          <p class="fluent-page-meta">
            {{ selectedCapability.description || selectedCapability.capabilityId }}
          </p>
          <CapabilityForm v-model="formValues" :schema="selectedCapability.inputSchema" />
          <div class="actions">
            <button
              class="fluent-control fluent-control--accent"
              type="button"
              :disabled="calling"
              @click="runPrimary"
            >
              {{
                isReadCapability(selectedCapability)
                  ? t("toolRuntime.execute")
                  : t("toolRuntime.preview")
              }}
            </button>
            <button
              v-if="awaitingExecute"
              class="fluent-control fluent-control--accent"
              type="button"
              :disabled="calling"
              @click="confirmExecute"
            >
              {{ t("toolRuntime.confirmExecute") }}
            </button>
          </div>
        </template>
        <p v-else-if="selectedTool" class="fluent-page-meta">
          {{ t("toolRuntime.selectCapability") }}
        </p>

        <div v-if="previewPayload != null" class="result">
          <h3 class="result-title">{{ t("toolRuntime.previewHeading") }}</h3>
          <pre class="fluent-surface result-body">{{ formatPayload(previewPayload) }}</pre>
        </div>
        <div v-if="resultPayload != null" class="result">
          <h3 class="result-title">{{ t("toolRuntime.resultHeading") }}</h3>
          <pre class="fluent-surface result-body">{{ formatPayload(resultPayload) }}</pre>
        </div>
      </section>
    </div>
  </section>
</template>

<style scoped>
.runtime {
  min-height: 0;
}

.card {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  padding: var(--fluent-space-lg);
}

.section-title {
  margin: 0;
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  font-size: var(--fluent-font-size-subtitle);
}

.row-head {
  display: flex;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  flex-wrap: wrap;
  align-items: center;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
  align-items: center;
}

.status-chip {
  min-height: var(--fluent-control-height-standard);
  pointer-events: none;
  cursor: default;
  font-size: var(--fluent-font-size-caption);
}

.gateway-url-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
  align-items: center;
}

.gateway-url-label {
  flex: 0 0 auto;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.gateway-url-input {
  flex: 1 1 16rem;
  min-width: 12rem;
}

.layout {
  display: grid;
  grid-template-columns: minmax(14rem, 22rem) minmax(0, 1fr);
  gap: var(--fluent-space-md);
  min-height: 0;
  align-items: start;
}

.pane {
  min-width: 0;
}

.list {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
  max-height: 22rem;
  overflow: auto;
}

.row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  grid-template-rows: auto auto;
  column-gap: var(--fluent-space-sm);
  row-gap: var(--fluent-space-xs);
  width: 100%;
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.row--active {
  border-color: var(--fluent-color-fill-accent);
  background: var(--fluent-color-fill-selected);
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.detail {
  grid-column: 1 / -1;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-title {
  margin: 0;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.result-body {
  margin: 0;
  padding: var(--fluent-space-md);
  overflow: auto;
  max-height: 16rem;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: var(--fluent-font-size-caption);
  white-space: pre-wrap;
  word-break: break-word;
}

@media (max-width: 800px) {
  .layout {
    grid-template-columns: 1fr;
  }
}
</style>
