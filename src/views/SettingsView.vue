<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { applyUiChromeFromValues } from "../state/uiChrome";

type SettingsTier = "core" | "power" | "experimental";

type SettingDefinition = {
  key: string;
  tier: SettingsTier;
  title: string;
  description: string;
  valueKind: "boolean" | "string" | "number" | "stringArray" | "enum";
  enumValues?: string[];
  default: unknown;
};

type SettingsSchemaManifest = {
  version: number;
  tiers: SettingsTier[];
  definitions: SettingDefinition[];
};

type SettingsDocument = {
  version: number;
  values: Record<string, unknown>;
};

const loading = ref(true);
const savingKey = ref<string | null>(null);
const error = ref<string | null>(null);
const schema = ref<SettingsSchemaManifest | null>(null);
const document = ref<SettingsDocument | null>(null);
const activeTier = ref<SettingsTier>("core");

const tiers: { id: SettingsTier; label: string }[] = [
  { id: "core", label: "Core" },
  { id: "power", label: "Power" },
  { id: "experimental", label: "Experimental" },
];

const visibleDefs = computed(() =>
  (schema.value?.definitions ?? []).filter((def) => def.tier === activeTier.value),
);

async function loadAll() {
  loading.value = true;
  error.value = null;
  try {
    const [manifest, doc] = await Promise.all([
      invoke<SettingsSchemaManifest>("settings_schema_get"),
      invoke<SettingsDocument>("settings_get"),
    ]);
    schema.value = manifest;
    document.value = doc;
    applyUiChromeFromValues(doc.values);
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    loading.value = false;
  }
}

function valueOf(key: string): unknown {
  return document.value?.values[key];
}

async function patchValue(key: string, value: unknown) {
  savingKey.value = key;
  error.value = null;
  try {
    document.value = await invoke<SettingsDocument>("settings_set", {
      patch: { [key]: value },
    });
    applyUiChromeFromValues(document.value.values);
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
    await loadAll();
  } finally {
    savingKey.value = null;
  }
}

function onBooleanChange(key: string, event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  void patchValue(key, checked);
}

function onSelectChange(key: string, event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  void patchValue(key, value);
}

function onTextChange(key: string, event: Event) {
  const value = (event.target as HTMLInputElement).value;
  void patchValue(key, value);
}

onMounted(() => {
  void loadAll();
});
</script>

<template>
  <section class="fluent-page panel">
    <header class="fluent-page-header">
      <h1 class="fluent-page-title">Settings</h1>
      <p class="fluent-page-meta">
        Rust schema authority
        <template v-if="schema">· v{{ schema.version }}</template>
        <template v-if="document">· {{ Object.keys(document.values).length }} keys</template>
      </p>
    </header>

    <p v-if="loading" class="fluent-banner">Loading schema…</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>

    <template v-else-if="schema && document">
      <div class="tier-tabs" role="tablist">
        <button
          v-for="tier in tiers"
          :key="tier.id"
          type="button"
          class="fluent-chip"
          :class="{ 'fluent-chip--active': activeTier === tier.id }"
          role="tab"
          :aria-selected="activeTier === tier.id"
          @click="activeTier = tier.id"
        >
          {{ tier.label }}
        </button>
      </div>

      <div class="fluent-surface fields">
        <div v-for="def in visibleDefs" :key="def.key" class="field">
          <div class="field-text">
            <label class="title" :for="def.key">{{ def.title }}</label>
            <p class="desc">{{ def.description }}</p>
            <code class="key">{{ def.key }}</code>
          </div>

          <div class="control">
            <input
              v-if="def.valueKind === 'boolean'"
              :id="def.key"
              type="checkbox"
              class="fluent-field-check"
              :checked="Boolean(valueOf(def.key))"
              :disabled="savingKey === def.key"
              @change="onBooleanChange(def.key, $event)"
            />

            <select
              v-else-if="def.valueKind === 'enum'"
              :id="def.key"
              class="fluent-control select"
              :disabled="savingKey === def.key"
              :value="String(valueOf(def.key) ?? '')"
              @change="onSelectChange(def.key, $event)"
            >
              <option v-for="option in def.enumValues ?? []" :key="option" :value="option">
                {{ option }}
              </option>
            </select>

            <input
              v-else-if="def.valueKind === 'string'"
              :id="def.key"
              class="fluent-control text"
              type="text"
              :disabled="savingKey === def.key"
              :value="String(valueOf(def.key) ?? '')"
              @change="onTextChange(def.key, $event)"
            />

            <span v-else class="readonly">{{ valueOf(def.key) }}</span>
          </div>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
.panel {
  max-width: 52rem;
}

.tier-tabs {
  display: flex;
  gap: var(--fluent-space-sm);
}

.fields {
  display: flex;
  flex-direction: column;
}

.field {
  display: grid;
  grid-template-columns: 1fr minmax(9rem, 14rem);
  gap: var(--fluent-space-md);
  align-items: center;
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.field:last-child {
  border-bottom: 0;
}

.field-text {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
  min-width: 0;
}

.title {
  font-weight: var(--fluent-font-weight-semibold);
}

.desc {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.key {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.control {
  display: flex;
  justify-content: flex-end;
}

.select,
.text {
  width: 100%;
}

.readonly {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
</style>
