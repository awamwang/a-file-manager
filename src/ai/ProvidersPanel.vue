<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/** AIA-220 / A7 — AI Providers MVP: mainstream presets + Compatible. No billing. */
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { toastSuccess } from "../state/toast";
import type { AiConfigDocument, AiProvider } from "./types";

const { t } = useI18n();

const document = ref<AiConfigDocument | null>(null);
const error = ref<string | null>(null);
const loading = ref(false);
const selectedId = ref<string | null>(null);

const draftName = ref("");
const draftId = ref("");
const draftBaseUrl = ref("");
const draftApiKey = ref("");
const draftModel = ref("");

const ERROR_KEYS: Record<string, string> = {
  AI_PROVIDER_ID_EMPTY: "ai.providers.errorIdEmpty",
  AI_PROVIDER_ID_INVALID: "ai.providers.errorIdInvalid",
  AI_PROVIDER_NAME_EMPTY: "ai.providers.errorNameEmpty",
  AI_PROVIDER_UNKNOWN: "ai.providers.errorUnknown",
  AI_PROVIDER_BUILTIN_REMOVE: "ai.providers.errorBuiltinRemove",
  AI_PROVIDER_ID_CONFLICT: "ai.providers.errorIdConflict",
};

function mapError(err: unknown): string {
  const raw = err instanceof Error ? err.message : String(err);
  for (const [code, key] of Object.entries(ERROR_KEYS)) {
    if (raw.includes(code)) return t(key);
  }
  return raw;
}

const providers = computed(() => document.value?.aiProviders ?? []);
const selected = computed(
  () => providers.value.find((p) => p.id === selectedId.value) ?? null,
);
const summary = computed(() => {
  const all = providers.value;
  return t("ai.providers.summary", {
    total: all.length,
    enabled: all.filter((p) => p.enabled).length,
    compatible: all.filter((p) => p.kind === "compatible").length,
  });
});

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    document.value = await invoke<AiConfigDocument>("ai_providers_list");
    if (selectedId.value && !providers.value.some((p) => p.id === selectedId.value)) {
      selectedId.value = null;
    }
  } catch (err) {
    error.value = mapError(err);
  } finally {
    loading.value = false;
  }
}

function select(row: AiProvider) {
  selectedId.value = row.id;
  draftName.value = row.displayName;
  draftBaseUrl.value = row.baseUrl;
  draftApiKey.value = row.apiKey;
  draftModel.value = row.defaultModel;
}

async function toggleEnabled(row: AiProvider) {
  error.value = null;
  try {
    document.value = await invoke<AiConfigDocument>("ai_providers_set_enabled", {
      id: row.id,
      enabled: !row.enabled,
    });
    toastSuccess(
      row.enabled ? t("ai.providers.noticeDisabled") : t("ai.providers.noticeEnabled"),
    );
  } catch (err) {
    error.value = mapError(err);
  }
}

async function saveSelected() {
  if (!selected.value) return;
  error.value = null;
  try {
    document.value = await invoke<AiConfigDocument>("ai_providers_update", {
      id: selected.value.id,
      patch: {
        displayName: selected.value.kind === "compatible" ? draftName.value : null,
        baseUrl: draftBaseUrl.value,
        apiKey: draftApiKey.value,
        defaultModel: draftModel.value,
      },
    });
    toastSuccess(t("ai.providers.noticeSaved"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function addCompatible() {
  error.value = null;
  try {
    document.value = await invoke<AiConfigDocument>("ai_providers_upsert_compatible", {
      id: draftId.value.trim() || null,
      displayName: draftName.value,
      baseUrl: draftBaseUrl.value,
      apiKey: draftApiKey.value,
      defaultModel: draftModel.value,
    });
    toastSuccess(t("ai.providers.noticeCompatibleAdded"));
    const added = providers.value.find((p) => p.displayName === draftName.value.trim());
    if (added) select(added);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function removeSelected() {
  if (!selected.value || selected.value.kind !== "compatible") return;
  error.value = null;
  try {
    document.value = await invoke<AiConfigDocument>("ai_providers_remove", {
      id: selected.value.id,
    });
    selectedId.value = null;
    toastSuccess(t("ai.providers.noticeRemoved"));
  } catch (err) {
    error.value = mapError(err);
  }
}

function startCompatibleDraft() {
  selectedId.value = null;
  draftName.value = "";
  draftId.value = "";
  draftBaseUrl.value = "https://";
  draftApiKey.value = "";
  draftModel.value = "";
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="providers-panel" data-ai-panel="providers">
    <p class="hint">{{ t("ai.providers.hint") }}</p>
    <p class="summary">{{ summary }}</p>
    <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>

    <div class="toolbar">
      <button type="button" class="fluent-btn" @click="startCompatibleDraft">
        {{ t("ai.providers.addCompatible") }}
      </button>
      <button type="button" class="fluent-btn" :disabled="loading" @click="refresh">
        {{ t("ai.providers.refresh") }}
      </button>
    </div>

    <ul class="list" role="list">
      <li
        v-for="row in providers"
        :key="row.id"
        class="row"
        :class="{ active: selectedId === row.id }"
        role="listitem"
        @click="select(row)"
      >
        <div class="row-main">
          <strong>{{ row.displayName }}</strong>
          <span class="meta">{{ row.id }} · {{ row.kind }}</span>
        </div>
        <button type="button" class="fluent-btn fluent-btn--quiet" @click.stop="toggleEnabled(row)">
          {{ row.enabled ? t("ai.providers.disable") : t("ai.providers.enable") }}
        </button>
      </li>
    </ul>

    <div class="editor">
      <h3>{{ selected ? t("ai.providers.editTitle") : t("ai.providers.compatibleTitle") }}</h3>
      <label class="field">
        <span>{{ t("ai.providers.fieldName") }}</span>
        <input
          v-model="draftName"
          class="fluent-input"
          type="text"
          :disabled="!!selected && selected.kind === 'preset'"
        />
      </label>
      <label v-if="!selected" class="field">
        <span>{{ t("ai.providers.fieldId") }}</span>
        <input v-model="draftId" class="fluent-input" type="text" />
      </label>
      <label class="field">
        <span>{{ t("ai.providers.fieldBaseUrl") }}</span>
        <input v-model="draftBaseUrl" class="fluent-input" type="text" />
      </label>
      <label class="field">
        <span>{{ t("ai.providers.fieldApiKey") }}</span>
        <input v-model="draftApiKey" class="fluent-input" type="password" autocomplete="off" />
      </label>
      <label class="field">
        <span>{{ t("ai.providers.fieldModel") }}</span>
        <input v-model="draftModel" class="fluent-input" type="text" />
      </label>
      <div class="actions">
        <button
          v-if="selected"
          type="button"
          class="fluent-btn fluent-btn--primary"
          @click="saveSelected"
        >
          {{ t("ai.providers.save") }}
        </button>
        <button
          v-else
          type="button"
          class="fluent-btn fluent-btn--primary"
          :disabled="!draftName.trim()"
          @click="addCompatible"
        >
          {{ t("ai.providers.saveCompatible") }}
        </button>
        <button
          v-if="selected?.kind === 'compatible'"
          type="button"
          class="fluent-btn"
          @click="removeSelected"
        >
          {{ t("ai.providers.remove") }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.hint,
.summary {
  margin: 0 0 var(--fluent-space-sm);
  opacity: 0.85;
  font-size: 0.9rem;
}
.toolbar,
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-xs);
  margin-bottom: var(--fluent-space-sm);
}
.list {
  list-style: none;
  margin: 0 0 var(--fluent-space-md);
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-sm);
  padding: 0.45rem 0.6rem;
  border-radius: 6px;
  cursor: pointer;
}
.row.active {
  background: var(--fluent-subtle-fill, rgba(0, 0, 0, 0.06));
}
.row-main {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
}
.meta {
  opacity: 0.7;
  font-size: 0.8rem;
}
.editor h3 {
  margin: 0 0 var(--fluent-space-sm);
  font-size: 1rem;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin-bottom: var(--fluent-space-sm);
}
</style>
