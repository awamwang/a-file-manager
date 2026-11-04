<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { applyUiChromeFromValues } from "../state/uiChrome";
import { applyAppLocaleFromValues } from "../i18n/index";
import {
  settingEnumOptionPath,
  settingFieldMessagePath,
} from "../i18n/settingsFields";
import { useI18n } from "vue-i18n";
import FluentIcon from "../components/icons/FluentIcon.vue";
import { settingsIconForKey, type FluentGlyphId } from "../components/icons/glyphs";

const { t, te } = useI18n();

function fieldTitle(def: SettingDefinition): string {
  const path = settingFieldMessagePath(def.key, "title");
  return te(path) ? String(t(path)) : def.title;
}

function fieldDescription(def: SettingDefinition): string {
  const path = settingFieldMessagePath(def.key, "description");
  return te(path) ? String(t(path)) : def.description;
}

function enumOptionLabel(settingKey: string, option: string): string {
  const path = settingEnumOptionPath(settingKey, option);
  return te(path) ? String(t(path)) : option;
}

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

const TIER_ICONS: Record<SettingsTier, FluentGlyphId> = {
  core: "settings",
  power: "flash",
  experimental: "bug",
};

const loading = ref(true);
const savingKey = ref<string | null>(null);
const error = ref<string | null>(null);
const schema = ref<SettingsSchemaManifest | null>(null);
const document = ref<SettingsDocument | null>(null);
const activeTier = ref<SettingsTier>("core");

const tiers = computed(() => [
  { id: "core" as const, label: t("settings.tierCore"), icon: TIER_ICONS.core },
  { id: "power" as const, label: t("settings.tierPower"), icon: TIER_ICONS.power },
  {
    id: "experimental" as const,
    label: t("settings.tierExperimental"),
    icon: TIER_ICONS.experimental,
  },
]);

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
    applyAppLocaleFromValues(doc.values);
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
    applyAppLocaleFromValues(document.value.values);
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
      <h1 class="fluent-page-title">{{ t("settings.title") }}</h1>
      <p class="fluent-page-meta">
        {{ t("settings.meta") }}
        <template v-if="schema">{{ t("settings.metaVersion", { version: schema.version }) }}</template>
        <template v-if="document">{{
          t("settings.metaKeys", { count: Object.keys(document.values).length })
        }}</template>
      </p>
    </header>

    <p v-if="loading" class="fluent-banner">{{ t("settings.loading") }}</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>

    <template v-else-if="schema && document">
      <div class="settings-layout">
        <aside class="tier-nav fluent-surface" role="tablist" :aria-label="t('settings.title')">
          <button
            v-for="tier in tiers"
            :key="tier.id"
            type="button"
            class="tier-item"
            :class="{
              'tier-item--active': activeTier === tier.id,
              'tier-item--warning': tier.id === 'experimental' && activeTier === 'experimental',
            }"
            role="tab"
            :aria-selected="activeTier === tier.id"
            @click="activeTier = tier.id"
          >
            <span class="fluent-icon-wash">
              <FluentIcon :name="tier.icon" size="md" />
            </span>
            <span class="tier-label">{{ tier.label }}</span>
          </button>
        </aside>

        <div class="fluent-surface fields">
          <div
            v-for="def in visibleDefs"
            :key="def.key"
            class="field"
            :class="{ 'field--experimental': def.tier === 'experimental' }"
          >
            <span class="fluent-icon-wash field-icon">
              <FluentIcon :name="settingsIconForKey(def.key)" size="md" />
            </span>

            <div class="field-text">
              <label class="title" :for="def.key">{{ fieldTitle(def) }}</label>
              <p class="desc">{{ fieldDescription(def) }}</p>
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
                  {{ enumOptionLabel(def.key, option) }}
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
      </div>
    </template>
  </section>
</template>

<style scoped>
.panel {
  max-width: 64rem;
}

.settings-layout {
  display: grid;
  grid-template-columns: 13.5rem minmax(0, 1fr);
  gap: var(--fluent-space-lg);
  align-items: start;
}

.tier-nav {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
  padding: var(--fluent-space-sm);
  background: var(--fluent-color-fill-layer-alt);
}

.tier-item {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  width: 100%;
  min-height: var(--fluent-control-height-comfortable);
  padding: var(--fluent-space-sm);
  border: 0;
  border-radius: var(--fluent-radius-md);
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.tier-item:hover {
  background: var(--fluent-color-fill-control-hover);
}

.tier-item--active {
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
  box-shadow: inset 3px 0 0 var(--fluent-color-stroke-accent-soft);
}

.tier-item--warning {
  background: var(--fluent-color-status-warning-bg);
}

.tier-label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.fields {
  display: flex;
  flex-direction: column;
}

.field {
  display: grid;
  grid-template-columns: auto 1fr minmax(9rem, 14rem);
  gap: var(--fluent-space-md);
  align-items: center;
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.field:last-child {
  border-bottom: 0;
}

.field--experimental {
  background: color-mix(in srgb, var(--fluent-color-status-warning-bg) 35%, transparent);
}

.field-icon {
  align-self: start;
  margin-top: 2px;
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

@media (max-width: 720px) {
  .settings-layout {
    grid-template-columns: 1fr;
  }

  .field {
    grid-template-columns: auto 1fr;
  }

  .control {
    grid-column: 2;
    justify-content: flex-start;
  }
}
</style>
