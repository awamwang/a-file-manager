<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { applyUiChromeFromValues } from "../state/uiChrome";
import { rememberSettingsDocument } from "../state/settingsHydrate";
import { applyAppLocaleFromValues } from "../i18n/index";
import {
  settingEnumOptionPath,
  settingFieldMessagePath,
} from "../i18n/settingsFields";
import { useI18n } from "vue-i18n";
import FluentIcon from "../components/icons/FluentIcon.vue";
import HealthView from "./HealthView.vue";
import { settingsIconForKey, type FluentGlyphId } from "../components/icons/glyphs";
import {
  findShortcutConflicts,
  listShortcutsForDisplay,
} from "../shortcuts/index.ts";
import { applyLaunchAtStartupPreference } from "../utils/autostartSync";
import {
  favoritesOsHook,
  loadFavoritesOsHookStatus,
  setFavoritesOsHookEnabled,
} from "../state/favorites";

const { t, te, locale } = useI18n();
const route = useRoute();
const router = useRouter();

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
type SettingsSection = SettingsTier | "health" | "shortcuts" | "favorites" | "openTools";

type IdeAppInfo = {
  id: string;
  name: string;
  available: boolean;
  enabled?: boolean;
  resolvedPath?: string | null;
  resolution?: string;
  configuredPath?: string;
};

const OPEN_TOOL_IDS = ["vscode", "cursor", "trae", "qoder"] as const;

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
const activeSection = ref<SettingsSection>("core");

/** Common-use first; experimental always last. */
const sections = computed(() => [
  { id: "core" as const, label: t("settings.tierCore"), icon: TIER_ICONS.core },
  {
    id: "openTools" as const,
    label: t("settings.sectionOpenTools"),
    icon: "puzzle" as FluentGlyphId,
  },
  {
    id: "favorites" as const,
    label: t("settings.sectionFavorites"),
    icon: "star" as FluentGlyphId,
  },
  {
    id: "shortcuts" as const,
    label: t("settings.sectionShortcuts"),
    icon: "commands" as FluentGlyphId,
  },
  { id: "power" as const, label: t("settings.tierPower"), icon: TIER_ICONS.power },
  {
    id: "health" as const,
    label: t("settings.sectionHealth"),
    icon: "heartPulse" as FluentGlyphId,
  },
  {
    id: "experimental" as const,
    label: t("settings.tierExperimental"),
    icon: TIER_ICONS.experimental,
  },
]);

const ideApps = ref<IdeAppInfo[]>([]);
const ideAppsLoading = ref(false);

const shortcutRows = computed(() => {
  void locale.value;
  return listShortcutsForDisplay();
});

const shortcutConflicts = computed(() => findShortcutConflicts());

const visibleDefs = computed(() =>
  (schema.value?.definitions ?? []).filter(
    (def) =>
      activeSection.value !== "health" &&
      activeSection.value !== "shortcuts" &&
      activeSection.value !== "favorites" &&
      activeSection.value !== "openTools" &&
      def.tier === activeSection.value &&
      // AIA-87/89: Discovery owns blacklist editing (Settings is not the authority UI).
      def.key !== "core.discoveryBlacklist" &&
      // Open tools has its own section UI.
      !def.key.startsWith("core.openTool."),
  ),
);

function isSpecialSettingsPath(path: string) {
  return (
    path.endsWith("/health") ||
    path.endsWith("/shortcuts") ||
    path.endsWith("/favorites") ||
    path.endsWith("/open-tools")
  );
}

function syncSectionFromRoute() {
  if (route.path.endsWith("/health")) {
    activeSection.value = "health";
  } else if (route.path.endsWith("/shortcuts")) {
    activeSection.value = "shortcuts";
  } else if (route.path.endsWith("/favorites")) {
    activeSection.value = "favorites";
  } else if (route.path.endsWith("/open-tools")) {
    activeSection.value = "openTools";
  } else if (
    activeSection.value === "health" ||
    activeSection.value === "shortcuts" ||
    activeSection.value === "favorites" ||
    activeSection.value === "openTools"
  ) {
    activeSection.value = "core";
  }
}

function openToolEnabledKey(id: string) {
  return `core.openTool.${id}.enabled`;
}

function openToolPathKey(id: string) {
  return `core.openTool.${id}.path`;
}

async function refreshIdeApps() {
  ideAppsLoading.value = true;
  try {
    ideApps.value = await invoke<IdeAppInfo[]>("ide_list_apps");
  } catch {
    ideApps.value = OPEN_TOOL_IDS.map((id) => ({
      id,
      name: id,
      available: false,
      enabled: Boolean(valueOf(openToolEnabledKey(id)) ?? true),
      configuredPath: String(valueOf(openToolPathKey(id)) ?? ""),
      resolution: "none",
    }));
  } finally {
    ideAppsLoading.value = false;
  }
}

function ideStatusLabel(app: IdeAppInfo): string {
  if (!app.available) return t("settings.openTools.notFound");
  if (app.resolution === "configured") return t("settings.openTools.viaConfigured");
  return t("settings.openTools.viaAuto");
}

async function onOpenToolEnabledChange(id: string, event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  await patchValue(openToolEnabledKey(id), checked);
  await refreshIdeApps();
}

async function onOpenToolPathChange(id: string, event: Event) {
  const value = (event.target as HTMLInputElement).value;
  await patchValue(openToolPathKey(id), value);
  await refreshIdeApps();
}

async function onToggleOsFavoriteVerb(event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  try {
    await setFavoritesOsHookEnabled(checked);
  } catch {
    await loadFavoritesOsHookStatus().catch(() => undefined);
  }
}

function selectSection(section: SettingsSection) {
  activeSection.value = section;
  if (section === "health") {
    if (route.path !== "/settings/health") {
      void router.push("/settings/health");
    }
    return;
  }
  if (section === "shortcuts") {
    if (route.path !== "/settings/shortcuts") {
      void router.push("/settings/shortcuts");
    }
    return;
  }
  if (section === "favorites") {
    if (route.path !== "/settings/favorites") {
      void router.push("/settings/favorites");
    }
    return;
  }
  if (section === "openTools") {
    if (route.path !== "/settings/open-tools") {
      void router.push("/settings/open-tools");
    }
    void refreshIdeApps();
    return;
  }
  if (isSpecialSettingsPath(route.path)) {
    void router.push("/settings");
  }
}

watch(
  () => route.path,
  () => {
    syncSectionFromRoute();
  },
  { immediate: true },
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
    rememberSettingsDocument(doc);
    applyUiChromeFromValues(doc.values);
    applyAppLocaleFromValues(doc.values);
    // Keep OS login item aligned with persisted preference (best-effort).
    try {
      await applyLaunchAtStartupPreference(Boolean(doc.values["core.launchAtStartup"]));
    } catch (syncErr) {
      console.warn("autostart sync on load failed", syncErr);
    }
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
    if (key === "core.launchAtStartup" && typeof value === "boolean") {
      await applyLaunchAtStartupPreference(value);
    }
    const patch: Record<string, unknown> = { [key]: value };
    // Hidden-at-login only applies when launch-at-startup is on.
    if (key === "core.launchAtStartup" && value === false) {
      patch["core.launchAtStartupHidden"] = false;
    }
    document.value = await invoke<SettingsDocument>("settings_set", {
      patch,
    });
    rememberSettingsDocument(document.value);
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

function isBooleanDisabled(key: string): boolean {
  if (key === "core.launchAtStartupHidden") {
    return !Boolean(valueOf("core.launchAtStartup"));
  }
  return false;
}

function onSelectChange(key: string, event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  void patchValue(key, value);
}

function onTextChange(key: string, event: Event) {
  const value = (event.target as HTMLInputElement).value;
  void patchValue(key, value);
}

function stringArrayText(key: string): string {
  const value = valueOf(key);
  if (Array.isArray(value)) {
    return value.map((item) => String(item)).join("\n");
  }
  return "";
}

function onStringArrayChange(key: string, event: Event) {
  const raw = (event.target as HTMLTextAreaElement).value;
  const paths = raw
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  void patchValue(key, paths);
}

onMounted(() => {
  void loadAll().then(() => {
    if (activeSection.value === "openTools") {
      void refreshIdeApps();
    }
  });
  void loadFavoritesOsHookStatus().catch(() => undefined);
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

    <div class="settings-layout">
      <aside class="tier-nav fluent-surface" role="tablist" :aria-label="t('settings.title')">
        <button
          v-for="section in sections"
          :key="section.id"
          type="button"
          class="tier-item"
          :class="{
            'tier-item--active': activeSection === section.id,
            'tier-item--warning':
              section.id === 'experimental' && activeSection === 'experimental',
          }"
          role="tab"
          :aria-selected="activeSection === section.id"
          @click="selectSection(section.id)"
        >
          <span class="fluent-icon-wash">
            <FluentIcon :name="section.icon" size="md" />
          </span>
          <span class="tier-label">{{ section.label }}</span>
        </button>
      </aside>

      <div v-if="activeSection === 'health'" class="health-slot">
        <HealthView />
      </div>

      <div v-else-if="activeSection === 'favorites'" class="fluent-surface favorites-panel">
        <p class="favorites-meta">{{ t("settings.favoritesMeta") }}</p>
        <label class="favorites-os-hook" :title="favoritesOsHook?.note ?? ''">
          <input
            type="checkbox"
            class="fluent-field-check"
            :checked="Boolean(favoritesOsHook?.registered)"
            @change="onToggleOsFavoriteVerb"
          />
          <span>{{ t("favorites.osVerb") }}</span>
        </label>
        <p v-if="favoritesOsHook?.note" class="favorites-note">{{ favoritesOsHook.note }}</p>
      </div>

      <div v-else-if="activeSection === 'openTools'" class="fluent-surface open-tools-panel">
        <p class="open-tools-meta">{{ t("settings.openTools.meta") }}</p>
        <p v-if="ideAppsLoading" class="fluent-banner">{{ t("settings.loading") }}</p>
        <div v-for="app in ideApps" :key="app.id" class="open-tool-row">
          <div class="open-tool-text">
            <span class="title">{{ app.name }}</span>
            <p class="desc">{{ ideStatusLabel(app) }}</p>
            <code v-if="app.resolvedPath" class="key" :title="app.resolvedPath">{{
              app.resolvedPath
            }}</code>
          </div>
          <div class="open-tool-controls">
            <label class="open-tool-toggle">
              <input
                type="checkbox"
                class="fluent-field-check"
                :checked="Boolean(valueOf(openToolEnabledKey(app.id)) ?? true)"
                :disabled="savingKey === openToolEnabledKey(app.id)"
                @change="onOpenToolEnabledChange(app.id, $event)"
              />
              <span>{{ t("settings.openTools.enabled") }}</span>
            </label>
            <input
              class="fluent-control text open-tool-path"
              type="text"
              :placeholder="t('settings.openTools.pathPlaceholder')"
              :disabled="savingKey === openToolPathKey(app.id)"
              :value="String(valueOf(openToolPathKey(app.id)) ?? '')"
              @change="onOpenToolPathChange(app.id, $event)"
            />
          </div>
        </div>
        <button
          type="button"
          class="fluent-control open-tools-refresh"
          :disabled="ideAppsLoading"
          @click="refreshIdeApps"
        >
          {{ t("settings.openTools.rediscover") }}
        </button>
      </div>

      <div v-else-if="activeSection === 'shortcuts'" class="fluent-surface shortcuts-panel">
        <p class="shortcuts-meta">{{ t("shortcuts.meta") }}</p>
        <p
          v-if="shortcutConflicts.length"
          class="fluent-banner fluent-banner--error"
        >
          {{
            t("shortcuts.conflict", {
              chords: shortcutConflicts.map((c) => c.chordLabel).join(", "),
            })
          }}
        </p>
        <div
          v-for="row in shortcutRows"
          :key="row.id"
          class="shortcut-row"
          :class="{ 'shortcut-row--blocked': Boolean(row.blockedBy) }"
        >
          <div class="shortcut-text">
            <span class="title">{{ t(row.labelKey) }}</span>
            <code class="key">{{ row.id }}</code>
            <p v-if="row.blockedBy" class="desc">
              {{ t("shortcuts.blockedBy", { id: row.blockedBy }) }}
            </p>
          </div>
          <kbd class="chord">{{ row.chordLabel }}</kbd>
        </div>
      </div>

      <template v-else>
        <p v-if="loading" class="fluent-banner">{{ t("settings.loading") }}</p>
        <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>

        <div v-else-if="schema && document" class="fluent-surface fields">
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
                :disabled="savingKey === def.key || isBooleanDisabled(def.key)"
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

              <textarea
                v-else-if="def.valueKind === 'stringArray'"
                :id="def.key"
                class="fluent-control text array-text"
                rows="4"
                :disabled="savingKey === def.key"
                :value="stringArrayText(def.key)"
                @change="onStringArrayChange(def.key, $event)"
              />

              <span v-else class="readonly">{{ valueOf(def.key) }}</span>
            </div>
          </div>
        </div>
      </template>
    </div>
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

.health-slot {
  min-width: 0;
}

.shortcuts-panel,
.favorites-panel,
.open-tools-panel {
  display: flex;
  flex-direction: column;
  padding: var(--fluent-space-md) 0;
}

.shortcuts-meta,
.favorites-meta,
.open-tools-meta {
  margin: 0 var(--fluent-space-lg) var(--fluent-space-md);
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.open-tool-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(14rem, 18rem);
  gap: var(--fluent-space-md);
  align-items: start;
  padding: var(--fluent-space-sm) var(--fluent-space-lg);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.open-tool-text {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.open-tool-text .title {
  font-weight: var(--fluent-font-weight-semibold);
}

.open-tool-text .desc {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.open-tool-text .key {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.open-tool-controls {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
}

.open-tool-toggle {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  font: inherit;
  cursor: pointer;
}

.open-tool-path {
  width: 100%;
}

.open-tools-refresh {
  align-self: flex-start;
  margin: var(--fluent-space-md) var(--fluent-space-lg) 0;
}

.favorites-os-hook {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  margin: 0 var(--fluent-space-lg);
  min-height: var(--fluent-control-height-comfortable);
  font: inherit;
  color: var(--fluent-color-text-primary);
  cursor: pointer;
}

.favorites-note {
  margin: var(--fluent-space-sm) var(--fluent-space-lg) 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.shortcut-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.shortcut-row:last-child {
  border-bottom: 0;
}

.shortcut-row--blocked {
  opacity: 0.55;
}

.shortcut-text {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
  min-width: 0;
}

.chord {
  flex-shrink: 0;
  padding: var(--fluent-space-xs) var(--fluent-space-sm);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-sm);
  background: var(--fluent-color-fill-subtle);
  font-family: var(--fluent-font-family-mono);
  font-size: var(--fluent-font-size-caption);
}

.health-slot :deep(.panel) {
  max-width: none;
  padding: 0;
}

.health-slot :deep(.fluent-page-header) {
  margin-bottom: var(--fluent-space-md);
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

.array-text {
  min-height: 5rem;
  resize: vertical;
  font-family: var(--fluent-font-family-monospace, Consolas, monospace);
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

  .open-tool-row {
    grid-template-columns: 1fr;
  }
}
</style>
