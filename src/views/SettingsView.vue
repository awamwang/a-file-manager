<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import {
  computed,
  defineAsyncComponent,
  onMounted,
  onUnmounted,
  ref,
  watch,
} from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import {
  CUSTOM_TOOLS_SETTING_KEY,
  TOOL_ORDER_SETTING_KEY,
  listOpenTools,
  parseCustomTools,
  type CustomOpenTool,
  type OpenToolInfo,
  type ProjectOpenMode,
} from "../state/openTools";
import { createListPointerReorder } from "../state/listPointerReorder";
import { normalizeOpenToolExtensions } from "../state/openToolExtensions";
import { applyUiChromeFromValues } from "../state/uiChrome";
import {
  getCachedSettingsDocument,
  getCachedSettingsSchema,
  hydrateSettingsSchema,
  rememberSettingsDocument,
  type SettingsDocument,
  type SettingsSchemaManifest,
} from "../state/settingsHydrate";
import { scheduleAfterPaint } from "../state/scheduleIdle";
import { applyAppLocaleFromValues } from "../i18n/index";
import {
  getCachedPlatformCapabilities,
  hydratePlatform,
  type PlatformCapabilities,
} from "../config/platform.ts";
import {
  settingEnumOptionPath,
  settingFieldMessagePath,
} from "../i18n/settingsFields";
import { useI18n } from "vue-i18n";
import FluentIcon from "../components/icons/FluentIcon.vue";
import { settingsIconForKey, type FluentGlyphId } from "../components/icons/glyphs";
import {
  chordFromEvent,
  chordKey,
  effectiveChordsForId,
  effectiveShortcutDefinitions,
  findShortcutConflicts,
  listShortcutActionsForDisplay,
  listShortcutsForDisplay,
  resetActionChords,
  resetAllShortcutOverrides,
  setActionChords,
  shortcutOverridesVersion,
  validateNewChord,
  type ShortcutChord,
  type ShortcutValidationResult,
} from "../shortcuts/index.ts";
import { applyLaunchAtStartupPreference } from "../utils/autostartSync";
import {
  favoritesOsHook,
  loadFavoritesOsHookStatus,
  setFavoritesOsHookEnabled,
} from "../state/favorites";
import {
  compareSettingsUiKeys,
  isPlatformUnsupportedSetting,
  isSettingsUiHiddenKey,
  settingsUiGroupForKey,
  settingsUiGroupMessagePath,
  settingsUiGroupOrder,
  type SettingsUiGroupId,
} from "../state/settingsUi";
import {
  displaySettingsStringValue,
  isSettingsEmptyMeansDefaultKey,
  loadSettingsEffectiveDefaults,
  normalizeSettingsStringForSave,
  type SettingsEffectiveDefaults,
} from "../state/settingsEffectiveDefaults";

/** AIA-276: Health / Search stay off the default Settings chunk until that section opens. */
const HealthView = defineAsyncComponent(() => import("./HealthView.vue"));
const AboutView = defineAsyncComponent(() => import("./AboutView.vue"));
const SearchSettingsPanel = defineAsyncComponent(
  () => import("../components/settings/SearchSettingsPanel.vue"),
);

const { t, te, locale } = useI18n();
const route = useRoute();
const router = useRouter();

const effectiveDefaults = ref<SettingsEffectiveDefaults>({});

/** macOS port: capability-gated settings UI, resolved once per session. */
const platformCaps = ref<PlatformCapabilities>(getCachedPlatformCapabilities());
const showOsFavoriteVerb = computed(() => {
  const caps = platformCaps.value;
  // Unknown host (vite-only / IPC pending) keeps the Windows-first UI intact.
  return !caps.os || caps.os === "windows" || caps.osShellVerb;
});

function fieldTitle(def: SettingDefinition): string {
  const path = settingFieldMessagePath(def.key, "title");
  return te(path) ? String(t(path)) : def.title;
}

function fieldDescription(def: SettingDefinition): string {
  const path = settingFieldMessagePath(def.key, "description");
  const fallbackDefault =
    isSettingsEmptyMeansDefaultKey(def.key)
      ? effectiveDefaults.value[def.key]
      : undefined;
  if (te(path)) {
    return fallbackDefault != null && fallbackDefault !== ""
      ? String(t(path, { default: fallbackDefault }))
      : String(t(path));
  }
  return def.description;
}

function enumOptionLabel(settingKey: string, option: string): string {
  const path = settingEnumOptionPath(settingKey, option);
  return te(path) ? String(t(path)) : option;
}

type SettingsTier = "core" | "power" | "experimental";
type SettingsSection =
  | SettingsTier
  | "health"
  | "search"
  | "shortcuts"
  | "openTools"
  | "about";

type IdeAppInfo = OpenToolInfo;

const OPEN_TOOL_IDS = [
  "vscode",
  "cursor",
  "trae",
  "qoder",
  "qwenwork",
  "sublime",
  "notepadpp",
  "windsurf",
  "idea",
  "pycharm",
  "webstorm",
  "sevenzip",
  "chrome",
  "edge",
  "vlc",
] as const;

/**
 * Fallback rows when `ide_list_apps` fails. Names and scopes mirror the
 * backend presets (`preset_meta` / platform IDE_DEFS) so the Settings page
 * still renders meaningful rows without the registry.
 */
const OPEN_TOOL_FALLBACK: Record<
  string,
  { name: string; supportsProject: boolean; fileExtensions: string[] }
> = {
  vscode: { name: "Visual Studio Code", supportsProject: true, fileExtensions: [] },
  cursor: { name: "Cursor", supportsProject: true, fileExtensions: [] },
  trae: { name: "Trae", supportsProject: true, fileExtensions: [] },
  qoder: { name: "Qoder CN", supportsProject: true, fileExtensions: [] },
  qwenwork: { name: "QwenWork", supportsProject: true, fileExtensions: [] },
  sublime: { name: "Sublime Text", supportsProject: true, fileExtensions: [] },
  // Any file, never a folder: Notepad++ has no folder view.
  notepadpp: { name: "Notepad++", supportsProject: false, fileExtensions: ["*"] },
  windsurf: { name: "Windsurf", supportsProject: true, fileExtensions: [] },
  idea: { name: "IntelliJ IDEA", supportsProject: true, fileExtensions: [] },
  pycharm: { name: "PyCharm", supportsProject: true, fileExtensions: [] },
  webstorm: { name: "WebStorm", supportsProject: true, fileExtensions: [] },
  sevenzip: {
    name: "7-Zip File Manager",
    supportsProject: false,
    fileExtensions: ["zip", "7z", "rar", "tar", "gz", "tgz", "bz2", "xz", "zst", "iso", "cab"],
  },
  chrome: {
    name: "Google Chrome",
    supportsProject: false,
    fileExtensions: ["html", "htm", "xhtml", "svg", "pdf"],
  },
  edge: {
    name: "Microsoft Edge",
    supportsProject: false,
    fileExtensions: ["html", "htm", "xhtml", "svg", "pdf"],
  },
  vlc: {
    name: "VLC media player",
    supportsProject: false,
    fileExtensions: [
      "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v",
      "mp3", "flac", "wav", "aac", "ogg", "opus", "m4a",
    ],
  },
};

type SettingDefinition = SettingsSchemaManifest["definitions"][number];

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
    id: "search" as const,
    label: t("settings.sectionSearch"),
    icon: "search" as FluentGlyphId,
  },
  {
    id: "openTools" as const,
    label: t("settings.sectionOpenTools"),
    icon: "puzzle" as FluentGlyphId,
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
  {
    id: "about" as const,
    label: t("settings.sectionAbout"),
    icon: "globe" as FluentGlyphId,
  },
]);

const ideApps = ref<IdeAppInfo[]>([]);
const ideAppsLoading = ref(false);
/** User-defined open tools parsed from the `core.openTool.custom` setting. */
const customTools = ref<CustomOpenTool[]>([]);

/** Settings-page search over built-in + custom tool names. */
const openToolSearch = ref("");
const openToolSearchActive = computed(() => openToolSearch.value.trim() !== "");
/** More than this many unavailable built-in rows collapse behind an expander. */
const UNAVAILABLE_COLLAPSE_THRESHOLD = 4;
const showUnavailableTools = ref(false);

/**
 * The single Settings list: built-in presets + custom tools merged in the
 * persisted menu order (`ide_list_apps` applies `core.openTool.order`), i.e.
 * the same order the "Open with" menus show. This list is the first-level
 * entry: drag to reorder, tick to enable, Configure opens the details.
 */
const filteredOpenToolRows = computed(() => {
  const query = openToolSearch.value.trim().toLowerCase();
  if (!query) return ideApps.value;
  return ideApps.value.filter((app) => app.name.toLowerCase().includes(query));
});
const unavailableBuiltinCount = computed(
  () => filteredOpenToolRows.value.filter((app) => app.builtin && !app.available).length,
);
/** Unavailable built-in rows hidden behind the expander (0 = nothing collapsed). */
const hiddenUnavailableCount = computed(() => {
  if (openToolSearchActive.value || showUnavailableTools.value) return 0;
  return Math.max(0, unavailableBuiltinCount.value - UNAVAILABLE_COLLAPSE_THRESHOLD);
});
/** Rows actually rendered: persisted order, surplus unavailable built-ins
 * hidden (custom tools always stay visible). */
const visibleOpenToolRows = computed(() => {
  if (!hiddenUnavailableCount.value) return filteredOpenToolRows.value;
  let shownUnavailable = 0;
  return filteredOpenToolRows.value.filter((app) => {
    if (!app.builtin || app.available) return true;
    shownUnavailable += 1;
    return shownUnavailable <= UNAVAILABLE_COLLAPSE_THRESHOLD;
  });
});
/** The “hide” affordance only makes sense with a collapsible surplus. */
const canCollapseUnavailable = computed(
  () => showUnavailableTools.value && unavailableBuiltinCount.value > UNAVAILABLE_COLLAPSE_THRESHOLD,
);

/** One row per function, with all FINAL chords (builtins + user overrides). */
const shortcutActionRows = computed(() => {
  void locale.value;
  void shortcutOverridesVersion.value;
  return listShortcutActionsForDisplay();
});

const shortcutConflicts = computed(() => {
  void shortcutOverridesVersion.value;
  return findShortcutConflicts(effectiveShortcutDefinitions());
});

/** Legacy flat view still supplies the count of shadowed bindings. */
const shortcutBlockedCount = computed(() => {
  void shortcutOverridesVersion.value;
  return listShortcutsForDisplay(effectiveShortcutDefinitions()).filter(
    (row) => Boolean(row.blockedBy),
  ).length;
});

const recordingActionId = ref<string | null>(null);
/** null = adding a new chord; a number = replacing that existing binding. */
const recordingBindingIndex = ref<number | null>(null);
const shortcutNotice = ref<string | null>(null);
let recordingListenerAttached = false;

function currentChords(actionId: string): ShortcutChord[] {
  return effectiveChordsForId(actionId);
}

function startRecording(actionId: string, index: number | null = null) {
  stopRecording();
  shortcutNotice.value = null;
  recordingActionId.value = actionId;
  recordingBindingIndex.value = index;
  // Capture phase: the combo must never reach the global shortcut dispatcher.
  window.addEventListener("keydown", onRecordKeydown, true);
  recordingListenerAttached = true;
}

function stopRecording() {
  if (recordingListenerAttached) {
    window.removeEventListener("keydown", onRecordKeydown, true);
    recordingListenerAttached = false;
  }
  recordingActionId.value = null;
  recordingBindingIndex.value = null;
}

function noticeForValidation(result: ShortcutValidationResult): string {
  switch (result.reason) {
    case "reserved":
      return String(t("shortcuts.errorReserved"));
    case "duplicate":
      return String(t("shortcuts.errorDuplicate"));
    case "conflict":
      return String(t("shortcuts.errorConflict", { id: result.ownerId ?? "" }));
    default:
      return String(t("shortcuts.errorUnknown"));
  }
}

function onRecordKeydown(event: KeyboardEvent) {
  const actionId = recordingActionId.value;
  if (!actionId) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.key === "Escape") {
    stopRecording();
    return;
  }
  const chord = chordFromEvent(event);
  // Still holding a modifier (or IME composing): keep listening.
  if (!chord) return;
  const result = validateNewChord(actionId, chord);
  if (!result.ok) {
    // Re-recording the very same chord is a no-op, not a duplicate error.
    const replaceIndex = recordingBindingIndex.value;
    const oldChord = replaceIndex === null ? null : currentChords(actionId)[replaceIndex];
    if (
      result.reason === "duplicate" &&
      oldChord &&
      chordKey(oldChord) === chordKey(chord)
    ) {
      stopRecording();
      return;
    }
    shortcutNotice.value = noticeForValidation(result);
    return;
  }
  const replaceIndex = recordingBindingIndex.value;
  const chords = currentChords(actionId);
  if (replaceIndex !== null && replaceIndex >= 0 && replaceIndex < chords.length) {
    const next = chords.slice();
    next[replaceIndex] = chord;
    setActionChords(actionId, next);
  } else {
    setActionChords(actionId, [...chords, chord]);
  }
  shortcutNotice.value = null;
  stopRecording();
}

function removeChord(actionId: string, chord: ShortcutChord) {
  stopRecording();
  const key = chordKey(chord);
  setActionChords(
    actionId,
    currentChords(actionId).filter((candidate) => chordKey(candidate) !== key),
  );
  shortcutNotice.value = null;
}

function restoreAction(actionId: string) {
  resetActionChords(actionId);
  shortcutNotice.value = null;
  stopRecording();
}

function restoreAll() {
  resetAllShortcutOverrides();
  shortcutNotice.value = null;
  stopRecording();
}

onUnmounted(() => {
  stopRecording();
  disposeOpenToolReorder();
});

const visibleDefs = computed(() =>
  (schema.value?.definitions ?? [])
    .filter(
      (def) =>
        activeSection.value !== "health" &&
        activeSection.value !== "search" &&
        activeSection.value !== "shortcuts" &&
        activeSection.value !== "openTools" &&
        activeSection.value !== "about" &&
        def.tier === activeSection.value &&
        // Open tools has its own section UI.
        !def.key.startsWith("core.openTool.") &&
        // FIL-59：过时/双入口项默认隐藏（键仍在 schema）。
        !isSettingsUiHiddenKey(def.key) &&
        // Host policy is authoritative: settings_schema_get already narrowed
        // enum options/defaults and marked OS-only rows (preview Peek on macOS,
        // Everything, HKCU autostart...). The capability fallback below only
        // matters for a cached/vite-only schema that predates that field.
        !def.hidden &&
        !isPlatformUnsupportedSetting(def.key, platformCaps.value),
    )
    .slice()
    .sort((a, b) => compareSettingsUiKeys(a.key, b.key)),
);

type SettingsFieldGroup = {
  id: SettingsUiGroupId;
  label: string;
  defs: SettingDefinition[];
};

const visibleFieldGroups = computed((): SettingsFieldGroup[] => {
  const defs = visibleDefs.value;
  if (!defs.length) return [];
  const byGroup = new Map<SettingsUiGroupId, SettingDefinition[]>();
  for (const def of defs) {
    const id = settingsUiGroupForKey(def.key);
    const list = byGroup.get(id) ?? [];
    list.push(def);
    byGroup.set(id, list);
  }
  return settingsUiGroupOrder()
    .map((id) => {
      const groupDefs = byGroup.get(id);
      if (!groupDefs?.length) return null;
      return {
        id,
        label: t(settingsUiGroupMessagePath(id)),
        defs: groupDefs,
      };
    })
    .filter((group): group is SettingsFieldGroup => group != null);
});

function isSpecialSettingsPath(path: string) {
  return (
    path.endsWith("/health") ||
    path.endsWith("/discovery") ||
    path.endsWith("/shortcuts") ||
    path.endsWith("/favorites") ||
    path.endsWith("/open-tools") ||
    path.endsWith("/about")
  );
}

function syncSectionFromRoute() {
  if (route.path.endsWith("/health")) {
    activeSection.value = "health";
  } else if (route.path.endsWith("/discovery")) {
    activeSection.value = "search";
  } else if (route.path.endsWith("/shortcuts")) {
    activeSection.value = "shortcuts";
  } else if (route.path.endsWith("/favorites")) {
    // FIL-65：收藏配置并入高级；保留深链兼容。
    activeSection.value = "power";
  } else if (route.path.endsWith("/open-tools")) {
    activeSection.value = "openTools";
  } else if (route.path.endsWith("/about")) {
    activeSection.value = "about";
  } else if (
    activeSection.value === "health" ||
    activeSection.value === "search" ||
    activeSection.value === "shortcuts" ||
    activeSection.value === "openTools" ||
    activeSection.value === "about"
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
  customTools.value = parseCustomTools(valueOf(CUSTOM_TOOLS_SETTING_KEY));
  try {
    ideApps.value = await listOpenTools();
  } catch {
    ideApps.value = OPEN_TOOL_IDS.map((id) => {
      const meta = OPEN_TOOL_FALLBACK[id] ?? { name: id, supportsProject: false, fileExtensions: [] };
      return {
        id,
        name: meta.name,
        available: false,
        enabled: Boolean(valueOf(openToolEnabledKey(id)) ?? true),
        configuredPath: String(valueOf(openToolPathKey(id)) ?? ""),
        resolution: "none",
        resolvedPath: null,
        builtin: true,
        supportsProject: meta.supportsProject,
        projectOpenMode: "workspace" as ProjectOpenMode,
        foldersOnly: false,
        projectArgs: [],
        fileExtensions: meta.fileExtensions,
      };
    });
  } finally {
    ideAppsLoading.value = false;
  }
}

function ideStatusLabel(app: IdeAppInfo): string {
  if (!app.available) return t("settings.openTools.notFound");
  if (app.resolution === "configured") return t("settings.openTools.viaConfigured");
  return t("settings.openTools.viaAuto");
}

async function onOpenToolPathChange(id: string, event: Event) {
  const value = (event.target as HTMLInputElement).value;
  await patchValue(openToolPathKey(id), value);
  await refreshIdeApps();
}

/**
 * Row checkbox = the `enabled` switch. Built-in presets persist to
 * `core.openTool.<id>.enabled`; custom tools edit their JSON entry.
 * `ide_list_apps` reports `enabled` for both, so `app.enabled` is the
 * single source for the checkbox.
 */
async function onRowEnabledChange(app: IdeAppInfo, event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  if (app.builtin) {
    await patchValue(openToolEnabledKey(app.id), checked);
    await refreshIdeApps();
  } else {
    patchCustomTool(app.id, { enabled: checked });
  }
}

/** Config dialog: null = closed; otherwise the open-tool id being edited. */
const configuringToolId = ref<string | null>(null);
const configuringApp = computed(
  () => ideApps.value.find((app) => app.id === configuringToolId.value) ?? null,
);
/** Custom tools edit their persisted JSON entry (not the discovery copy). */
const configuringCustomTool = computed(
  () => customTools.value.find((tool) => tool.id === configuringToolId.value) ?? null,
);

function openToolConfig(id: string) {
  configuringToolId.value = id;
}

function closeToolConfig() {
  configuringToolId.value = null;
}

/**
 * Drag-reorder the open tools that feed the "Open with" menus. The persisted
 * order (`core.openTool.order`) is applied backend-side inside
 * `ide_list_apps`, so this single list drives the order of the Settings rows,
 * the Navigator "Open with" submenu and the Project "open elsewhere" menu —
 * one source of truth. `ideApps` already comes back in the effective order.
 */
const {
  onPointerDown: openToolOnPointerDown,
  dragFromId: openToolDragFrom,
  dragOverId: openToolDragOver,
  dropEdge: openToolDropEdge,
  dragMoved: openToolDragMoved,
  dispose: disposeOpenToolReorder,
} = createListPointerReorder({
  itemSelector: "[data-open-tool-order] [data-list-item-id]",
  onReorder: async (fromId, toId) => {
    const ids = ideApps.value.map((app) => app.id);
    const from = ids.indexOf(fromId);
    const to = ids.indexOf(toId);
    if (from < 0 || to < 0 || from === to) return;
    ids.splice(to, 0, ids.splice(from, 1)[0]);
    await patchValue(TOOL_ORDER_SETTING_KEY, JSON.stringify(ids));
    await refreshIdeApps();
  },
});

function nextCustomToolId(existing: CustomOpenTool[]): string {
  let index = 1;
  const taken = new Set(existing.map((tool) => tool.id));
  while (taken.has(`custom:tool-${index}`)) index += 1;
  return `custom:tool-${index}`;
}

async function saveCustomTools(list: CustomOpenTool[]) {
  customTools.value = list;
  await patchValue(CUSTOM_TOOLS_SETTING_KEY, JSON.stringify(list));
  await refreshIdeApps();
}

function addCustomTool() {
  void saveCustomTools([
    ...customTools.value,
    {
      id: nextCustomToolId(customTools.value),
      name: t("settings.openTools.newCustomName"),
      path: "",
      enabled: true,
      supportsProject: false,
      projectOpenMode: "workspace",
      foldersOnly: false,
      projectArgs: [],
      fileExtensions: [],
    },
  ]);
}

function removeCustomTool(id: string) {
  void saveCustomTools(customTools.value.filter((tool) => tool.id !== id));
}

function patchCustomTool(id: string, patch: Partial<CustomOpenTool>) {
  void saveCustomTools(
    customTools.value.map((tool) => (tool.id === id ? { ...tool, ...patch } : tool)),
  );
}

function onCustomProjectArgsChange(id: string, raw: string) {
  patchCustomTool(id, { projectArgs: raw.split(/\s+/).filter(Boolean) });
}

function onCustomFileExtensionsChange(id: string, raw: string) {
  patchCustomTool(id, { fileExtensions: normalizeOpenToolExtensions(raw) });
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
  if (section === "search") {
    if (route.path !== "/settings/discovery") {
      void router.push("/settings/discovery");
    }
    return;
  }
  if (section === "shortcuts") {
    if (route.path !== "/settings/shortcuts") {
      void router.push("/settings/shortcuts");
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
  if (section === "about") {
    if (route.path !== "/settings/about") {
      void router.push("/settings/about");
    }
    return;
  }
  if (section === "power") {
    void loadFavoritesOsHookStatus().catch(() => undefined);
  }
  if (isSpecialSettingsPath(route.path)) {
    void router.push("/settings");
  }
}

const everythingPreferredVisible = computed(
  () => !isPlatformUnsupportedSetting("core.everythingPreferred", platformCaps.value),
);

async function onEverythingPreferredChange(value: boolean) {
  await patchValue("core.everythingPreferred", value);
}

watch(
  () => route.path,
  () => {
    syncSectionFromRoute();
  },
  { immediate: true },
);

/** AIA-276: paint from hydrate cache immediately; reconcile IPC off the first frame. */
function paintSettingsFromCache(): boolean {
  const cachedSchema = getCachedSettingsSchema();
  const cachedDoc = getCachedSettingsDocument();
  if (cachedSchema) schema.value = cachedSchema;
  if (cachedDoc) {
    document.value = cachedDoc;
    loading.value = false;
    return true;
  }
  return false;
}

async function refreshEffectiveDefaults() {
  effectiveDefaults.value = await loadSettingsEffectiveDefaults({
    newFolderLocaleDefault: String(t("contextMenu.newFolderDefault")),
    newFileLocaleDefault: String(t("contextMenu.newFileDefault")),
  });
}

async function loadAll() {
  error.value = null;
  const hadCache = paintSettingsFromCache();
  if (!hadCache) loading.value = true;
  try {
    const [manifest, doc] = await Promise.all([
      hydrateSettingsSchema(),
      invoke<SettingsDocument>("settings_get").then((fresh) => {
        rememberSettingsDocument(fresh);
        return fresh;
      }),
    ]);
    schema.value = manifest;
    document.value = doc;
    applyUiChromeFromValues(doc.values);
    applyAppLocaleFromValues(doc.values);
    await refreshEffectiveDefaults();
    // Autostart registry write is non-critical for Settings paint — defer past first frame.
    scheduleAfterPaint(() => {
      void applyLaunchAtStartupPreference(
        Boolean(doc.values["core.launchAtStartup"]),
      ).catch((syncErr) => {
        console.warn("autostart sync on load failed", syncErr);
      });
    });
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
  const raw = (event.target as HTMLInputElement).value;
  const value = normalizeSettingsStringForSave(
    key,
    raw,
    effectiveDefaults.value,
  );
  void patchValue(key, value);
}

function stringFieldDisplayValue(key: string): string {
  return displaySettingsStringValue(
    valueOf(key),
    key,
    effectiveDefaults.value,
  );
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

watch(locale, () => {
  void refreshEffectiveDefaults();
});

onMounted(() => {
  // Re-read after the single-flight IPC so rows appear/disappear when the
  // capability answer arrives (cold start may beat the bootstrap warm-up).
  void hydratePlatform().then((caps) => {
    platformCaps.value = caps;
  });
  void loadAll().then(() => {
    if (activeSection.value === "openTools") {
      void refreshIdeApps();
    }
  });
  // FIL-65：收藏 OS 动词在高级分区 — 仅进入时加载（AIA-276）。
  if (activeSection.value === "power") {
    scheduleAfterPaint(() => {
      void loadFavoritesOsHookStatus().catch(() => undefined);
    });
  }
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
      <nav class="tier-nav fluent-surface" :aria-label="t('settings.title')">
        <button
          v-for="section in sections"
          :key="section.id"
          type="button"
          class="tier-item"
          :class="{
            'tier-item--active': activeSection === section.id,
            'fluent-accent-edge': activeSection === section.id,
            'tier-item--warning':
              section.id === 'experimental' && activeSection === 'experimental',
          }"
          :aria-current="activeSection === section.id ? 'true' : undefined"
          @click="selectSection(section.id)"
        >
          <span class="fluent-icon-wash">
            <FluentIcon :name="section.icon" size="md" />
          </span>
          <span class="tier-label">{{ section.label }}</span>
        </button>
      </nav>

      <div v-if="activeSection === 'health'" class="health-slot">
        <HealthView />
      </div>

      <div v-else-if="activeSection === 'about'" class="about-slot">
        <AboutView />
      </div>

      <div v-else-if="activeSection === 'search'" class="search-slot">
        <SearchSettingsPanel
          :everything-preferred-visible="everythingPreferredVisible"
          :everything-preferred="Boolean(valueOf('core.everythingPreferred') ?? true)"
          :everything-preferred-saving="savingKey === 'core.everythingPreferred'"
          @update:everything-preferred="onEverythingPreferredChange"
        />
      </div>

      <div v-else-if="activeSection === 'openTools'" class="fluent-surface open-tools-panel">
        <p class="open-tools-meta">{{ t("settings.openTools.meta") }}</p>
        <p class="open-tools-meta">{{ t("settings.openTools.customMeta") }}</p>
        <div class="open-tools-toolbar">
          <input
            v-model="openToolSearch"
            class="fluent-control text open-tools-search"
            type="text"
            :placeholder="t('settings.openTools.searchPlaceholder')"
          />
          <button
            type="button"
            class="fluent-control open-tools-refresh"
            :disabled="ideAppsLoading"
            @click="refreshIdeApps"
          >
            {{ t("settings.openTools.rediscover") }}
          </button>
          <button type="button" class="fluent-control open-tools-add" @click="addCustomTool">
            {{ t("settings.openTools.addCustom") }}
          </button>
        </div>
        <p v-if="ideAppsLoading" class="fluent-hint">{{ t("settings.loading") }}</p>

        <div
          class="open-tool-order"
          data-open-tool-order
          :class="{ 'open-tool-order--reordering': openToolDragMoved }"
        >
          <div
            v-for="app in visibleOpenToolRows"
            :key="app.id"
            class="open-tool-order-row"
            :data-list-item-id="app.id"
            :class="{
              'open-tool-order-row--dragging': openToolDragFrom === app.id,
              'open-tool-order-row--before':
                openToolDragOver === app.id && openToolDropEdge === 'before',
              'open-tool-order-row--after':
                openToolDragOver === app.id && openToolDropEdge === 'after',
            }"
            @pointerdown="openToolOnPointerDown($event, app.id)"
          >
            <FluentIcon name="list" size="sm" class="open-tool-order-handle" />
            <div class="open-tool-order-text">
              <span class="open-tool-order-name">
                {{ app.name }}
                <span class="open-tool-order-kind">
                  {{
                    app.builtin
                      ? t("settings.openTools.builtinTitle")
                      : t("settings.openTools.customTitle")
                  }}
                </span>
                <span class="open-tool-order-status">{{ ideStatusLabel(app) }}</span>
              </span>
              <code v-if="app.resolvedPath" class="key" :title="app.resolvedPath">{{
                app.resolvedPath
              }}</code>
            </div>
            <div class="open-tool-controls">
              <label class="open-tool-toggle">
                <input
                  type="checkbox"
                  class="fluent-field-check"
                  :checked="app.enabled"
                  :disabled="savingKey === openToolEnabledKey(app.id)"
                  @change="onRowEnabledChange(app, $event)"
                />
                <span>{{ t("settings.openTools.enabled") }}</span>
              </label>
              <button
                type="button"
                class="fluent-control open-tool-configure"
                @click="openToolConfig(app.id)"
              >
                {{ t("settings.openTools.configure") }}
              </button>
            </div>
          </div>
        </div>
        <button
          v-if="hiddenUnavailableCount > 0"
          type="button"
          class="fluent-control open-tools-unavailable-toggle"
          @click="showUnavailableTools = true"
        >
          {{ t("settings.openTools.showUnavailable", { count: hiddenUnavailableCount }) }}
        </button>
        <button
          v-else-if="canCollapseUnavailable"
          type="button"
          class="fluent-control open-tools-unavailable-toggle"
          @click="showUnavailableTools = false"
        >
          {{ t("settings.openTools.hideUnavailable") }}
        </button>

        <Teleport to="body">
          <div
            v-if="configuringApp"
            class="open-tool-config-overlay"
            role="presentation"
            @click.self="closeToolConfig"
          >
            <div
              class="open-tool-config-dialog fluent-surface"
              role="dialog"
              aria-modal="true"
              :aria-label="t('settings.openTools.configure')"
            >
              <h3 class="open-tool-config-title">
                {{ t("settings.openTools.configTitle", { name: configuringApp.name }) }}
              </h3>

              <template v-if="configuringApp.builtin">
                <p class="open-tool-config-status">{{ ideStatusLabel(configuringApp) }}</p>
                <code v-if="configuringApp.resolvedPath" class="open-tool-config-path">{{
                  configuringApp.resolvedPath
                }}</code>
                <input
                  class="fluent-control text open-tool-path"
                  type="text"
                  :placeholder="t('settings.openTools.pathPlaceholder')"
                  :disabled="savingKey === openToolPathKey(configuringApp.id)"
                  :value="String(valueOf(openToolPathKey(configuringApp.id)) ?? '')"
                  @change="onOpenToolPathChange(configuringApp.id, $event)"
                />
              </template>

              <template v-else-if="configuringCustomTool">
                <p class="open-tool-config-status">
                  {{
                    configuringApp.available
                      ? t("settings.openTools.viaCustom")
                      : t("settings.openTools.notFound")
                  }}
                </p>
                <input
                  class="fluent-control text"
                  type="text"
                  :value="configuringCustomTool.name"
                  :placeholder="t('settings.openTools.customNamePlaceholder')"
                  @change="
                    patchCustomTool(configuringCustomTool.id, {
                      name: ($event.target as HTMLInputElement).value,
                    })
                  "
                />
                <input
                  class="fluent-control text open-tool-path"
                  type="text"
                  :value="configuringCustomTool.path"
                  :placeholder="t('settings.openTools.pathPlaceholder')"
                  @change="
                    patchCustomTool(configuringCustomTool.id, {
                      path: ($event.target as HTMLInputElement).value,
                    })
                  "
                />
                <input
                  class="fluent-control text"
                  type="text"
                  :value="configuringCustomTool.projectArgs.join(' ')"
                  :placeholder="t('settings.openTools.projectArgsPlaceholder')"
                  @change="
                    onCustomProjectArgsChange(
                      configuringCustomTool.id,
                      ($event.target as HTMLInputElement).value,
                    )
                  "
                />
                <input
                  class="fluent-control text open-tool-extensions"
                  type="text"
                  :value="configuringCustomTool.fileExtensions.join(', ')"
                  :placeholder="t('settings.openTools.fileExtensionsPlaceholder')"
                  @change="
                    onCustomFileExtensionsChange(
                      configuringCustomTool.id,
                      ($event.target as HTMLInputElement).value,
                    )
                  "
                />
                <div class="open-tool-config-fields">
                  <label class="open-tool-toggle">
                    <input
                      type="checkbox"
                      class="fluent-field-check"
                      :checked="configuringCustomTool.supportsProject"
                      @change="
                        patchCustomTool(configuringCustomTool.id, {
                          supportsProject: ($event.target as HTMLInputElement).checked,
                        })
                      "
                    />
                    <span>{{ t("settings.openTools.supportsProject") }}</span>
                  </label>
                  <label class="open-tool-toggle">
                    <input
                      type="checkbox"
                      class="fluent-field-check"
                      :checked="configuringCustomTool.foldersOnly"
                      @change="
                        patchCustomTool(configuringCustomTool.id, {
                          foldersOnly: ($event.target as HTMLInputElement).checked,
                        })
                      "
                    />
                    <span>{{ t("settings.openTools.foldersOnly") }}</span>
                  </label>
                  <select
                    class="fluent-control"
                    :value="configuringCustomTool.projectOpenMode"
                    @change="
                      patchCustomTool(configuringCustomTool.id, {
                        projectOpenMode: ($event.target as HTMLSelectElement)
                          .value as ProjectOpenMode,
                      })
                    "
                  >
                    <option value="workspace">
                      {{ t("settings.openTools.projectModeWorkspace") }}
                    </option>
                    <option value="contextFile">
                      {{ t("settings.openTools.projectModeContextFile") }}
                    </option>
                  </select>
                  <button
                    type="button"
                    class="fluent-control open-tool-remove"
                    @click="
                      removeCustomTool(configuringCustomTool.id);
                      closeToolConfig();
                    "
                  >
                    {{ t("settings.openTools.remove") }}
                  </button>
                </div>
              </template>

              <button
                type="button"
                class="fluent-control open-tool-config-close"
                @click="closeToolConfig"
              >
                {{ t("settings.openTools.configClose") }}
              </button>
            </div>
          </div>
        </Teleport>
      </div>

      <div v-else-if="activeSection === 'shortcuts'" class="fluent-surface shortcuts-panel">
        <p class="shortcuts-meta">{{ t("shortcuts.meta") }}</p>

        <div class="shortcut-toolbar">
          <button type="button" class="fluent-control" @click="restoreAll">
            {{ t("shortcuts.resetAll") }}
          </button>
          <span v-if="shortcutNotice" class="shortcut-notice">{{
            shortcutNotice
          }}</span>
        </div>

        <p
          v-if="shortcutConflicts.length"
          class="fluent-banner fluent-banner--error shortcuts-banner"
        >
          {{
            t("shortcuts.conflict", {
              chords: shortcutConflicts.map((c) => c.chordLabel).join(", "),
            })
          }}
          <template v-if="shortcutBlockedCount">
            {{ t("shortcuts.blockedCount", { count: shortcutBlockedCount }) }}
          </template>
        </p>

        <div v-for="row in shortcutActionRows" :key="row.id" class="shortcut-row">
          <div class="shortcut-text">
            <span class="title">
              {{ t(row.labelKey) }}
              <span v-if="row.isOverridden" class="shortcut-badge">{{
                t("shortcuts.overridden")
              }}</span>
            </span>
            <code class="key">{{ row.id }}</code>
            <p v-if="!row.configurable" class="desc">
              {{ t("shortcuts.fixedGroup") }}
            </p>
          </div>

          <div class="shortcut-bindings">
            <span
              v-for="(binding, index) in row.bindings"
              :key="binding.label"
              class="shortcut-binding"
            >
              <kbd
                class="chord"
                :class="{
                  'chord--blocked': Boolean(binding.blockedBy),
                  'chord--editable': row.configurable,
                  'chord--recording':
                    recordingActionId === row.id && recordingBindingIndex === index,
                }"
                :role="row.configurable ? 'button' : undefined"
                :tabindex="row.configurable ? 0 : undefined"
                :title="
                  binding.blockedBy
                    ? t('shortcuts.blockedBy', { id: binding.blockedBy })
                    : row.configurable
                      ? t('shortcuts.change')
                      : undefined
                "
                :aria-label="
                  row.configurable
                    ? `${binding.label} — ${t('shortcuts.change')}`
                    : undefined
                "
                @click="row.configurable && startRecording(row.id, index)"
                @keydown.enter.prevent="row.configurable && startRecording(row.id, index)"
                @keydown.space.prevent="row.configurable && startRecording(row.id, index)"
                >{{ binding.label }}</kbd
              >
              <button
                v-if="row.configurable"
                type="button"
                class="binding-remove"
                :aria-label="t('shortcuts.remove')"
                :title="t('shortcuts.remove')"
                @click="removeChord(row.id, binding.chord)"
              >
                ×
              </button>
            </span>
            <span v-if="!row.bindings.length" class="shortcut-unbound">{{
              t("shortcuts.unbound")
            }}</span>

            <template v-if="row.configurable">
              <button
                v-if="recordingActionId !== row.id"
                type="button"
                class="fluent-control shortcut-action"
                @click="startRecording(row.id)"
              >
                {{ t("shortcuts.add") }}
              </button>
              <template v-else>
                <span class="shortcut-recording">{{
                  t("shortcuts.recording")
                }}</span>
                <button
                  type="button"
                  class="fluent-control shortcut-action"
                  @click="stopRecording"
                >
                  {{ t("shortcuts.cancel") }}
                </button>
              </template>
              <button
                v-if="row.isOverridden"
                type="button"
                class="fluent-control shortcut-action"
                @click="restoreAction(row.id)"
              >
                {{ t("shortcuts.reset") }}
              </button>
            </template>
          </div>
        </div>
      </div>

      <template v-else>
        <p v-if="loading" class="fluent-hint">{{ t("settings.loading") }}</p>
        <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>

        <div v-else-if="schema && document" class="fluent-surface fields">
          <p
            v-if="platformCaps.storeBuild"
            class="fluent-banner fluent-banner--info store-build-autostart-notice"
          >
            {{ t("settings.storeBuildAutostartNotice") }}
          </p>
          <section
            v-for="group in visibleFieldGroups"
            :key="group.id"
            class="field-group"
          >
            <h2 class="field-group-title">{{ group.label }}</h2>
            <div
              v-for="def in group.defs"
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
                <code v-if="def.tier === 'experimental'" class="key">{{ def.key }}</code>
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
                  <option
                    v-for="option in def.enumValues ?? []"
                    :key="option"
                    :value="option"
                  >
                    {{ enumOptionLabel(def.key, option) }}
                  </option>
                </select>

                <input
                  v-else-if="def.valueKind === 'string'"
                  :id="def.key"
                  class="fluent-control text"
                  type="text"
                  :disabled="savingKey === def.key"
                  :value="stringFieldDisplayValue(def.key)"
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
          </section>

          <section
            v-if="activeSection === 'power' && showOsFavoriteVerb"
            class="field-group favorites-in-advanced"
          >
            <h2 class="field-group-title">{{ t("settings.favoritesMeta") }}</h2>
            <div class="field">
              <span class="fluent-icon-wash field-icon">
                <FluentIcon name="star" size="md" />
              </span>
              <div class="field-text">
                <label class="title" for="favorites-os-hook">{{
                  t("favorites.osVerb")
                }}</label>
                <p v-if="favoritesOsHook?.note" class="desc">{{ favoritesOsHook.note }}</p>
              </div>
              <div class="control">
                <input
                  id="favorites-os-hook"
                  type="checkbox"
                  class="fluent-field-check"
                  :checked="Boolean(favoritesOsHook?.registered)"
                  @change="onToggleOsFavoriteVerb"
                />
              </div>
            </div>
          </section>
        </div>
      </template>
    </div>
  </section>
</template>

<style scoped>
.panel {
  max-width: min(100%, 88rem);
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
  border-radius: var(--fluent-radius-sm);
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
  /* Left accent edge comes from the shared .fluent-accent-edge primitive
     (same declaration, applied via class on the active item). */
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
  gap: var(--fluent-space-lg);
  padding-block: var(--fluent-space-md);
  /* 表单型分区保留可读行长，避免壳层 88rem 后说明文字被拉成超长行。 */
  max-width: 52rem;
}

.fields-note {
  margin: 0 var(--fluent-space-lg);
}

.field-group {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.field-group-title {
  margin: 0 var(--fluent-space-lg) var(--fluent-space-sm);
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-secondary);
}

.health-slot,
.search-slot,
.about-slot {
  min-width: 0;
}

.shortcuts-panel,
.favorites-panel,
.open-tools-panel {
  display: flex;
  flex-direction: column;
  padding: var(--fluent-space-md) 0;
}

.open-tools-toolbar {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  margin: 0 var(--fluent-space-lg) var(--fluent-space-sm);
}

.open-tools-refresh,
.open-tools-add {
  white-space: nowrap;
}

.open-tool-order {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
  margin: 0 var(--fluent-space-lg);
  padding: var(--fluent-space-xs);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
}

.open-tool-order--reordering {
  cursor: grabbing;
  user-select: none;
}

.open-tool-order-row {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-xs) var(--fluent-space-sm);
  border-radius: var(--fluent-radius-sm);
  cursor: grab;
  background: transparent;
}

.open-tool-order-row:hover {
  background: var(--fluent-color-fill-subtle-secondary);
}

.open-tool-order-row--dragging {
  opacity: 0.4;
}

.open-tool-order-row--before {
  box-shadow: inset 0 2px 0 var(--fluent-color-fill-accent);
}

.open-tool-order-row--after {
  box-shadow: inset 0 -2px 0 var(--fluent-color-fill-accent);
}

.open-tool-order-handle {
  color: var(--fluent-color-text-secondary);
}

.open-tool-order-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.open-tool-order-name {
  min-width: 0;
  font-weight: var(--fluent-font-weight-semibold);
}

.open-tool-order-kind {
  margin-left: var(--fluent-space-xs);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.open-tool-order-status {
  margin-left: var(--fluent-space-xs);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.open-tool-order-text .key {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.open-tool-remove {
  align-self: flex-start;
}

.shortcuts-meta,
.favorites-meta,
.open-tools-meta {
  margin: 0 var(--fluent-space-lg) var(--fluent-space-md);
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.open-tool-controls {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  flex-shrink: 0;
}

.open-tool-toggle {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  font: inherit;
  cursor: pointer;
}

.open-tool-configure {
  white-space: nowrap;
}

.open-tool-path {
  width: 100%;
}

.open-tool-extensions {
  width: 100%;
}

.open-tools-search {
  flex: 1;
  min-width: 0;
  max-width: 320px;
}

.open-tools-unavailable-toggle {
  align-self: flex-start;
  margin: var(--fluent-space-xs) var(--fluent-space-lg) 0;
}

.open-tool-config-overlay {
  position: fixed;
  inset: 0;
  /* 与 PromptDialogHost 同级，盖在 toast(90) 与命令面板(1000) 之上。 */
  z-index: 1100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--fluent-space-lg);
  background: color-mix(in srgb, var(--fluent-color-text-primary) 28%, transparent);
}

.open-tool-config-dialog {
  width: min(30rem, 100%);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  max-height: 80vh;
  overflow: auto;
  padding: var(--fluent-space-lg);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  box-shadow: var(--fluent-shadow-flyout);
}

.open-tool-config-title {
  margin: 0;
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.open-tool-config-status {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.open-tool-config-path {
  overflow-wrap: anywhere;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.open-tool-config-fields {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.open-tool-config-close {
  align-self: flex-start;
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
  align-items: flex-start;
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
  font-family: var(--fluent-font-family-mono);
  font-size: var(--fluent-font-size-caption);
}

.shortcut-toolbar {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-md);
  margin: 0 var(--fluent-space-lg) var(--fluent-space-md);
}

.shortcut-notice {
  color: var(--fluent-color-status-danger);
  font-size: var(--fluent-font-size-caption);
}

.shortcuts-banner {
  margin-inline: var(--fluent-space-lg);
}

.shortcut-badge {
  margin-left: var(--fluent-space-sm);
  padding: 2px var(--fluent-space-sm);
  border-radius: var(--fluent-radius-sm);
  background: var(--fluent-color-fill-selected);
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-regular);
  white-space: nowrap;
}

.shortcut-bindings {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: var(--fluent-space-xs);
  max-width: 26rem;
}

/* Chord + its remove button share one frame, so × reads as part of the chip. */
.shortcut-binding {
  display: inline-flex;
  align-items: stretch;
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-sm);
  background: var(--fluent-color-fill-subtle);
  overflow: hidden;
}

.binding-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.25rem;
  align-self: stretch;
  padding: 0;
  border: 0;
  border-left: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: 0;
  background: transparent;
  color: var(--fluent-color-text-secondary);
  font: inherit;
  cursor: pointer;
}

.binding-remove:hover {
  background: var(--fluent-color-fill-control-hover);
  color: var(--fluent-color-status-danger);
}

.chord--editable {
  cursor: pointer;
}

.chord--editable:hover {
  background: var(--fluent-color-fill-control-hover);
}

.chord--recording {
  background: var(--fluent-color-fill-selected);
  outline: 2px solid var(--fluent-color-stroke-accent-soft);
  outline-offset: -2px;
}

.chord--blocked {
  opacity: 0.55;
  text-decoration: line-through;
}

.shortcut-unbound,
.shortcut-recording {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.shortcut-recording {
  color: var(--fluent-color-status-info);
}

.shortcut-action {
  padding: 2px var(--fluent-space-sm);
  font-size: var(--fluent-font-size-caption);
}

.health-slot :deep(.panel),
.about-slot :deep(.panel) {
  max-width: none;
  padding: 0;
}

.health-slot :deep(.fluent-page-header),
.about-slot :deep(.fluent-page-header) {
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
