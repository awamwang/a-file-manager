<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import CommandPanel from "../command/CommandPanel.vue";
import FluentIcon from "../icons/FluentIcon.vue";
import { NAV_ICON_BY_ID } from "../icons/glyphs";
import { buildAppCommands } from "../../commands/registry";
import { toggleCommandPanel } from "../../state/commandPanel";
import { requestGlobalSearchFocus } from "../../state/globalSearch";
import {
  dispatchShortcutEvent,
  registerShortcutHandler,
  unregisterShortcutHandler,
} from "../../shortcuts/index.ts";
import {
  bootstrapNavigatorOpenState,
  closeNavigatorTab,
  currentPath,
  goUp,
  listError,
  listLoading,
  listing,
  liveOpenState,
  navigateTo,
  newNavigatorTab,
  stripTabs,
  switchNavigatorTab,
} from "../../state/navigatorBrowse";
import {
  beginHistoryNavigation,
  buildBreadcrumbs,
  canGoBack,
  canGoForward,
  endHistoryNavigation,
  popBack,
  popForward,
} from "../../state/navHistory";
import {
  favoritesDoc,
  favoritesOsHook,
  loadFavorites,
  loadFavoritesOsHookStatus,
  setFavoritesOsHookEnabled,
} from "../../state/favorites";
import { dualPaneEnabled, toggleDualPane } from "../../state/dualPane";

const { t, locale } = useI18n();

type HealthPayload = {
  ok: boolean;
  name: string;
  version: string;
  identifier: string;
};

type NavItem = {
  id: keyof typeof NAV_ICON_BY_ID | string;
  labelKey: string;
  to?: string;
  later?: boolean;
};

function navIcon(id: string) {
  return NAV_ICON_BY_ID[id] ?? "folder";
}

const route = useRoute();
const router = useRouter();

const addressDraft = ref("");
const addressEditing = ref(false);
const activeTab = ref("navigator");
const health = ref<HealthPayload | null>(null);
const healthError = ref<string | null>(null);
const isMaximized = ref(false);
const maxHover = ref(false);
const unlisteners: UnlistenFn[] = [];

const breadcrumbs = computed(() => buildBreadcrumbs(currentPath.value));

async function ensureNavigatorThenGoUp() {
  if (route.path !== "/navigator") {
    await router.push("/navigator");
  }
  await goUp();
}

const panelCommands = computed(() => {
  void locale.value;
  return buildAppCommands({
    router,
    ensureNavigatorThenGoUp,
    refreshHealth: loadHealth,
    t: (key) => t(key),
  });
});

function focusAddressBar() {
  addressEditing.value = true;
  addressDraft.value = currentPath.value;
  requestAnimationFrame(() => {
    const input = document.getElementById("address-input");
    if (input instanceof HTMLInputElement) {
      input.focus();
      input.select();
    }
  });
}

async function onGoBack() {
  const target = popBack(currentPath.value);
  if (!target) return;
  await ensureNavigatorRoute();
  beginHistoryNavigation();
  try {
    await navigateTo(target);
  } finally {
    endHistoryNavigation();
  }
}

async function onGoForward() {
  const target = popForward(currentPath.value);
  if (!target) return;
  await ensureNavigatorRoute();
  beginHistoryNavigation();
  try {
    await navigateTo(target);
  } finally {
    endHistoryNavigation();
  }
}

async function onBreadcrumb(path: string) {
  addressEditing.value = false;
  await ensureNavigatorRoute();
  await navigateTo(path);
}

async function openFavoritePin(path: string) {
  await ensureNavigatorRoute();
  await navigateTo(path);
}

function favoriteLabel(path: string): string {
  const parts = path.replace(/\\/g, "/").replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || path;
}

async function onToggleOsFavoriteVerb(event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  try {
    await setFavoritesOsHookEnabled(checked);
  } catch {
    await loadFavoritesOsHookStatus().catch(() => undefined);
  }
}

function onGlobalKeydown(event: KeyboardEvent) {
  if (dispatchShortcutEvent(event)) {
    event.preventDefault();
  }
}

function registerShellShortcuts() {
  registerShortcutHandler("commandPanel.toggle", () => {
    toggleCommandPanel();
  });
  registerShortcutHandler("discovery.globalSearch", () => {
    requestGlobalSearchFocus();
    if (route.path !== "/discovery") {
      void router.push("/discovery");
    }
  });
  registerShortcutHandler("navigator.goUp", () => {
    void ensureNavigatorThenGoUp();
  });
  registerShortcutHandler("navigator.newTab", () => {
    void onNewNavigatorTab();
  });
  registerShortcutHandler("navigator.back", () => {
    void onGoBack();
  });
  registerShortcutHandler("navigator.forward", () => {
    void onGoForward();
  });
  registerShortcutHandler("chrome.focusAddress", () => {
    focusAddressBar();
  });
  registerShortcutHandler("view.settings", () => {
    void router.push("/settings");
  });
  registerShortcutHandler("view.shortcuts", () => {
    void router.push("/settings/shortcuts");
  });
}

function unregisterShellShortcuts() {
  for (const id of [
    "commandPanel.toggle",
    "discovery.globalSearch",
    "navigator.goUp",
    "navigator.newTab",
    "navigator.back",
    "navigator.forward",
    "chrome.focusAddress",
    "view.settings",
    "view.shortcuts",
  ]) {
    unregisterShortcutHandler(id);
  }
}

/** Primary pillars first; Discovery + Providers sit above Settings; Health is Settings-only. */
const navItems: NavItem[] = [
  { id: "navigator", labelKey: "nav.navigator", to: "/navigator" },
  { id: "work-groups", labelKey: "nav.workGroups", to: "/work-groups" },
  { id: "projects", labelKey: "nav.projects", to: "/projects" },
  { id: "extensions", labelKey: "nav.extensions", to: "/extensions" },
  { id: "discovery", labelKey: "nav.discovery", to: "/discovery" },
  { id: "providers", labelKey: "nav.providers", to: "/providers" },
  { id: "settings", labelKey: "nav.settings", to: "/settings" },
];

watch(
  currentPath,
  (path) => {
    addressDraft.value = path;
  },
  { immediate: true },
);

const statusText = computed(() => {
  void locale.value;
  const healthPart = healthError.value
    ? t("chrome.healthError")
    : health.value?.ok
      ? t("chrome.healthOk", { version: health.value.version })
      : t("chrome.healthPending");

  if (route.path === "/navigator") {
    if (listLoading.value) return `${healthPart} · ${t("chrome.navigatorLoading")}`;
    if (listError.value) return `${healthPart} · ${t("chrome.navigatorError")}`;
    if (listing.value) {
      return `${healthPart} · ${t("chrome.navigatorCounts", {
        dirs: listing.value.dirCount,
        files: listing.value.fileCount,
      })}`;
    }
  }
  return healthPart;
});

async function loadHealth() {
  healthError.value = null;
  try {
    health.value = await invoke<HealthPayload>("health_get");
  } catch (err) {
    health.value = null;
    healthError.value = err instanceof Error ? err.message : String(err);
  }
}

async function submitAddress() {
  addressEditing.value = false;
  if (route.path !== "/navigator") {
    await router.push("/navigator");
  }
  await navigateTo(addressDraft.value);
}

function onAddressBlur() {
  // Defer so click on Go / breadcrumb can run first.
  window.setTimeout(() => {
    if (document.activeElement?.id === "address-input") return;
    addressEditing.value = false;
    addressDraft.value = currentPath.value;
  }, 120);
}

async function onGoUp() {
  if (route.path !== "/navigator") {
    await router.push("/navigator");
  }
  await goUp();
}

async function refreshMaximized() {
  try {
    isMaximized.value = await getCurrentWindow().isMaximized();
  } catch {
    isMaximized.value = false;
  }
}

async function minimizeWindow() {
  await getCurrentWindow().minimize();
}

async function toggleMaximizeWindow() {
  await getCurrentWindow().toggleMaximize();
  await refreshMaximized();
}

async function closeWindow() {
  await getCurrentWindow().close();
}

function isNavActive(item: NavItem) {
  if (!item.to) return false;
  if (item.id === "settings") {
    return route.path === "/settings" || route.path.startsWith("/settings/");
  }
  return route.path === item.to;
}

function openTab(id: string, path: string) {
  activeTab.value = id;
  void router.push(path);
}

function tabLabel(path: string) {
  const parts = path.replace(/\\/g, "/").replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || path;
}

async function ensureNavigatorRoute() {
  activeTab.value = "navigator";
  if (route.path !== "/navigator") {
    await router.push("/navigator");
  }
}

async function onSwitchNavigatorTab(groupIndex: number) {
  await ensureNavigatorRoute();
  await switchNavigatorTab(groupIndex);
}

async function onCloseNavigatorTab(event: Event, groupIndex: number) {
  event.stopPropagation();
  event.preventDefault();
  await ensureNavigatorRoute();
  await closeNavigatorTab(groupIndex);
}

async function onNewNavigatorTab() {
  await ensureNavigatorRoute();
  await newNavigatorTab();
}

onMounted(() => {
  void loadHealth();
  void refreshMaximized();
  void bootstrapNavigatorOpenState();
  void loadFavorites().catch(() => undefined);
  void loadFavoritesOsHookStatus().catch(() => undefined);
  registerShellShortcuts();
  window.addEventListener("keydown", onGlobalKeydown);

  const win = getCurrentWindow();
  void win.onResized(() => {
    void refreshMaximized();
  }).then((unlisten) => {
    unlisteners.push(unlisten);
  });

  // Snap overlay owns the maximize hit-test (HTMAXBUTTON). Wire hover/click back to Fluent UI.
  void listen("tauri-frame://snap/click", () => {
    void toggleMaximizeWindow();
  }).then((unlisten) => unlisteners.push(unlisten));
  void listen("tauri-frame://snap/mouseenter", () => {
    maxHover.value = true;
  }).then((unlisten) => unlisteners.push(unlisten));
  void listen("tauri-frame://snap/mouseleave", () => {
    maxHover.value = false;
  }).then((unlisten) => unlisteners.push(unlisten));
});

onUnmounted(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
  unregisterShellShortcuts();
  for (const unlisten of unlisteners) {
    unlisten();
  }
});
</script>

<template>
  <div class="shell fluent-chrome">
    <header class="titlebar" data-tauri-frame-tb :aria-label="t('chrome.titleBar')">
      <div class="titlebar-drag" data-tauri-drag-region>
        <span class="brand" data-tauri-drag-region>{{ t("app.name") }}</span>
        <span class="caption-meta" data-tauri-drag-region>{{ t("app.captionMeta") }}</span>
      </div>
      <div class="window-controls" :aria-label="t('chrome.windowControls')">
        <button
          id="frame-tb-minimize"
          type="button"
          class="win-btn"
          :aria-label="t('chrome.minimize')"
          @click="minimizeWindow"
        >
          <FluentIcon name="minimize" size="sm" />
        </button>
        <!--
          Snap: native HTMAXBUTTON overlay (tauri-plugin-frame) covers this button.
          :hover / click come from tauri-frame://snap/* events — see script.
        -->
        <button
          id="frame-tb-maximize"
          type="button"
          class="win-btn"
          :class="{ 'win-btn--hover': maxHover }"
          :aria-label="isMaximized ? t('chrome.restore') : t('chrome.maximize')"
          @click="toggleMaximizeWindow"
        >
          <FluentIcon :name="isMaximized ? 'restore' : 'maximize'" size="sm" />
        </button>
        <button
          id="frame-tb-close"
          type="button"
          class="win-btn win-btn--close"
          :aria-label="t('chrome.close')"
          @click="closeWindow"
        >
          <FluentIcon name="close" size="sm" />
        </button>
      </div>
    </header>

    <div class="toolbar" role="toolbar" :aria-label="t('chrome.toolbar')">
      <button
        type="button"
        class="fluent-control toolbar-btn"
        :disabled="!canGoBack"
        :title="t('chrome.back')"
        :aria-label="t('chrome.back')"
        @click="onGoBack"
      >
        <FluentIcon name="back" size="sm" />
        <span>{{ t("chrome.back") }}</span>
      </button>
      <button
        type="button"
        class="fluent-control toolbar-btn"
        :disabled="!canGoForward"
        :title="t('chrome.forward')"
        :aria-label="t('chrome.forward')"
        @click="onGoForward"
      >
        <FluentIcon name="forward" size="sm" />
        <span>{{ t("chrome.forward") }}</span>
      </button>
      <button
        type="button"
        class="fluent-control toolbar-btn"
        :title="t('chrome.upTitle')"
        @click="onGoUp"
      >
        <FluentIcon name="up" size="sm" />
        <span>{{ t("chrome.up") }}</span>
      </button>
      <span class="toolbar-spacer" />
      <button
        type="button"
        class="fluent-control toolbar-btn"
        :class="{ 'toolbar-btn--active': dualPaneEnabled }"
        :title="t('chrome.dualPane')"
        :aria-pressed="dualPaneEnabled"
        @click="toggleDualPane"
      >
        <FluentIcon name="view" size="sm" />
        <span>{{ t("chrome.dualPane") }}</span>
      </button>
      <button
        type="button"
        class="fluent-control toolbar-btn"
        :title="t('chrome.commandsTitle')"
        aria-keyshortcuts="Control+Shift+P"
        @click="toggleCommandPanel"
      >
        <FluentIcon name="commands" size="sm" />
        <span>{{ t("chrome.commands") }}</span>
        <span class="toolbar-shortcut">{{ t("chrome.commandsShortcut") }}</span>
      </button>
    </div>

    <form class="address" :aria-label="t('chrome.address')" @submit.prevent="submitAddress">
      <label class="address-label" for="address-input">{{ t("chrome.path") }}</label>
      <div
        v-if="!addressEditing"
        class="breadcrumbs"
        role="navigation"
        :aria-label="t('chrome.breadcrumbs')"
        @click="focusAddressBar"
      >
        <template v-if="breadcrumbs.length">
          <button
            v-for="(crumb, index) in breadcrumbs"
            :key="crumb.path + String(index)"
            type="button"
            class="crumb"
            :title="crumb.path"
            @click.stop="onBreadcrumb(crumb.path)"
          >
            <span v-if="index > 0" class="crumb-sep" aria-hidden="true">›</span>
            <span class="crumb-label">{{ crumb.label }}</span>
          </button>
        </template>
        <span v-else class="crumb-placeholder">{{ t("chrome.pathPlaceholder") }}</span>
      </div>
      <input
        v-else
        id="address-input"
        v-model="addressDraft"
        class="address-input"
        type="text"
        spellcheck="false"
        autocomplete="off"
        :placeholder="t('chrome.pathPlaceholder')"
        @blur="onAddressBlur"
        @keydown.escape.prevent="addressEditing = false"
      />
      <button type="submit" class="fluent-control address-go">{{ t("chrome.go") }}</button>
    </form>

    <div class="tabstrip" role="tablist" :aria-label="t('chrome.tabs')">
      <div
        v-for="tab in stripTabs"
        :key="tab.id"
        class="tab"
        :class="{
          'tab--active':
            route.path === '/navigator' && liveOpenState.currentTabGroupIndex === tab.groupIndex,
        }"
        role="tab"
        :aria-selected="
          route.path === '/navigator' && liveOpenState.currentTabGroupIndex === tab.groupIndex
        "
        :title="tab.path"
        @click="onSwitchNavigatorTab(tab.groupIndex)"
      >
        <FluentIcon name="folder" size="sm" class="tab-icon" />
        <span class="tab-label">{{ tab.name || tabLabel(tab.path) }}</span>
        <button
          type="button"
          class="tab-close"
          :aria-label="t('chrome.closeTab')"
          :title="t('chrome.closeTab')"
          @click="onCloseNavigatorTab($event, tab.groupIndex)"
        >
          <FluentIcon name="close" size="sm" />
        </button>
      </div>
      <button
        v-if="!stripTabs.length"
        type="button"
        role="tab"
        class="tab"
        :class="{ 'tab--active': activeTab === 'navigator' || route.path === '/navigator' }"
        :aria-selected="route.path === '/navigator'"
        @click="openTab('navigator', '/navigator')"
      >
        <FluentIcon name="folderOpen" size="sm" />
        <span>{{ t("nav.navigator") }}</span>
      </button>
      <button
        type="button"
        class="tab tab--add"
        :title="t('chrome.newTab')"
        :aria-label="t('chrome.newTab')"
        @click="onNewNavigatorTab"
      >
        <FluentIcon name="add" size="sm" />
      </button>
    </div>

    <div class="body">
      <nav class="sidenav fluent-layer-alt" :aria-label="t('chrome.sideNav')">
        <div class="fav-pins" role="group" :aria-label="t('favorites.label')">
          <div class="fav-pins-title">
            <FluentIcon name="heart" size="sm" />
            <span>{{ t("favorites.label") }}</span>
          </div>
          <label class="fav-os-hook" :title="favoritesOsHook?.note ?? ''">
            <input
              type="checkbox"
              class="fluent-field-check"
              :checked="Boolean(favoritesOsHook?.registered)"
              @change="onToggleOsFavoriteVerb"
            />
            <span>{{ t("favorites.osVerb") }}</span>
          </label>
          <button
            v-for="fav in favoritesDoc.favorites"
            :key="fav.path"
            type="button"
            class="fluent-list-row sidenav-item fav-pin"
            :title="fav.path"
            :class="{
              'sidenav-item--active':
                route.path === '/navigator' &&
                currentPath.replace(/\\/g, '/').toLowerCase() ===
                  fav.path.replace(/\\/g, '/').toLowerCase(),
            }"
            @click="openFavoritePin(fav.path)"
          >
            <FluentIcon :name="fav.isDir ? 'folder' : 'document'" size="md" class="sidenav-icon" />
            <span class="sidenav-label">{{ favoriteLabel(fav.path) }}</span>
          </button>
          <p v-if="!favoritesDoc.favorites.length" class="fav-empty">
            {{ t("favorites.empty") }}
          </p>
        </div>
        <button
          v-for="item in navItems"
          :key="item.id"
          type="button"
          class="fluent-list-row sidenav-item"
          :class="{
            'sidenav-item--active': isNavActive(item),
            'sidenav-item--later': item.later,
          }"
          :disabled="item.later"
          :title="item.later ? t('chrome.later') : undefined"
          @click="item.to && router.push(item.to)"
        >
          <FluentIcon :name="navIcon(item.id)" size="md" class="sidenav-icon" />
          <span class="sidenav-label">{{ t(item.labelKey) }}</span>
          <span v-if="item.later" class="later-badge">{{ t("chrome.later") }}</span>
        </button>
      </nav>

      <main class="content">
        <slot />
      </main>
    </div>

    <footer class="statusbar" :aria-label="t('chrome.status')">
      <span
        class="status-text"
        :class="{
          'fluent-status-text--down': Boolean(healthError),
          'fluent-status-text--ok': health?.ok && !healthError,
        }"
      >{{ statusText }}</span>
      <button type="button" class="status-refresh" @click="loadHealth">
        {{ t("chrome.refreshHealth") }}
      </button>
    </footer>

    <CommandPanel :commands="panelCommands" />
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 100%;
  background: var(--fluent-color-fill-subtle);
}

.titlebar {
  display: flex;
  align-items: stretch;
  height: var(--fluent-titlebar-height);
  flex-shrink: 0;
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
  user-select: none;
}

.titlebar-drag {
  display: flex;
  flex: 1;
  align-items: center;
  gap: var(--fluent-space-md);
  padding-inline: var(--fluent-space-lg);
  min-width: 0;
}

.brand {
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.caption-meta {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.window-controls {
  display: flex;
  flex-shrink: 0;
}

.win-btn {
  width: var(--fluent-titlebar-button-width);
  height: var(--fluent-titlebar-height);
  border: 0;
  background: transparent;
  color: var(--fluent-color-text-primary);
  font-size: 12px;
  line-height: 1;
  transition: background-color 80ms ease;
}

.win-btn:hover,
.win-btn--hover {
  background: var(--fluent-color-fill-control-hover);
}

.win-btn:active {
  background: var(--fluent-color-fill-control-pressed);
}

.win-btn--close:hover {
  background: var(--fluent-color-status-danger);
  color: var(--fluent-color-text-on-accent);
}

.toolbar {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  height: var(--fluent-toolbar-height);
  flex-shrink: 0;
  padding-inline: var(--fluent-space-md);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
}

.toolbar-btn {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  min-width: var(--fluent-control-height-comfortable);
  height: var(--fluent-control-height-standard);
  min-height: var(--fluent-control-height-standard);
  padding-inline: var(--fluent-space-md);
}

.toolbar-btn--active {
  background: var(--fluent-color-fill-selected);
}

.toolbar-spacer {
  flex: 1;
}

.toolbar-shortcut {
  margin-inline-start: var(--fluent-space-sm);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  font-weight: var(--fluent-font-weight-regular);
}

.address {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  height: var(--fluent-addressbar-height);
  flex-shrink: 0;
  padding-inline: var(--fluent-space-md);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
}

.address-label {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.breadcrumbs {
  flex: 1;
  display: flex;
  align-items: center;
  flex-wrap: nowrap;
  overflow: hidden;
  min-height: var(--fluent-control-height-standard);
  padding-inline: var(--fluent-space-sm);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
  cursor: text;
}

.breadcrumbs:hover {
  background: var(--fluent-color-fill-subtle-secondary);
}

.crumb {
  display: inline-flex;
  align-items: center;
  max-width: 10rem;
  border: 0;
  padding: 0.1rem 0.2rem;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: var(--fluent-color-text-primary);
  font: inherit;
  font-size: var(--fluent-font-size-body);
  cursor: pointer;
}

.crumb:hover,
.crumb:focus-visible {
  background: var(--fluent-color-fill-selected);
  outline: none;
}

.crumb-sep {
  margin-inline-end: 0.25rem;
  color: var(--fluent-color-text-secondary);
}

.crumb-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.crumb-placeholder {
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  padding-inline: var(--fluent-space-xs);
}

.address-input {
  flex: 1;
  height: var(--fluent-control-height-standard);
  min-height: var(--fluent-control-height-standard);
  padding-inline: var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
  color: var(--fluent-color-text-primary);
  font: inherit;
}

.address-input:hover {
  background: var(--fluent-color-fill-subtle-secondary);
}

.address-input:focus {
  outline: 2px solid var(--fluent-color-stroke-focus);
  outline-offset: 1px;
  background: var(--fluent-color-fill-control);
}

.address-go {
  height: var(--fluent-control-height-standard);
  min-height: var(--fluent-control-height-standard);
}

.tabstrip {
  display: flex;
  align-items: flex-end;
  gap: var(--fluent-space-xs);
  min-height: var(--fluent-control-height-standard);
  flex-shrink: 0;
  padding: var(--fluent-space-xs) var(--fluent-space-md) 0;
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-subtle);
}

.tab {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  max-width: 14rem;
  min-height: var(--fluent-control-height-standard);
  padding: 0 var(--fluent-space-sm) 0 var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid transparent;
  border-bottom: 0;
  border-radius: var(--fluent-radius-sm) var(--fluent-radius-sm) 0 0;
  background: transparent;
  color: var(--fluent-color-text-secondary);
  font: inherit;
  cursor: pointer;
}

.tab:hover:not(:disabled) {
  background: var(--fluent-color-fill-control-hover);
  color: var(--fluent-color-text-primary);
}

.tab--active {
  background: var(--fluent-color-fill-control);
  border-color: var(--fluent-color-stroke-divider);
  color: var(--fluent-color-text-primary);
  font-weight: var(--fluent-font-weight-semibold);
  box-shadow: inset 0 -2px 0 var(--fluent-color-fill-accent);
}

.tab-icon {
  flex-shrink: 0;
  opacity: 0.85;
}

.tab-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tab-close {
  flex-shrink: 0;
  width: 1.25rem;
  height: 1.25rem;
  border: 0;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: inherit;
  font: inherit;
  line-height: 1;
  cursor: pointer;
  opacity: 0.7;
}

.tab-close:hover {
  background: var(--fluent-color-fill-control-pressed);
  opacity: 1;
}

.tab--add {
  min-width: var(--fluent-control-height-standard);
  justify-content: center;
  padding-inline: 0;
}

.body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.fav-pins {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding-bottom: var(--fluent-space-sm);
  margin-bottom: var(--fluent-space-sm);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.fav-pins-title {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  padding: var(--fluent-space-xs) var(--fluent-space-md);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.fav-os-hook {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  padding: 0 var(--fluent-space-md) var(--fluent-space-xs);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  cursor: pointer;
}

.fav-empty {
  margin: 0;
  padding: 0 var(--fluent-space-md) var(--fluent-space-xs);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.fav-pin .sidenav-label {
  font-size: var(--fluent-font-size-caption);
}

.sidenav {
  width: var(--fluent-sidenav-width);
  flex-shrink: 0;
  padding: var(--fluent-space-sm);
  border-right: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
}

.sidenav-item {
  position: relative;
  width: 100%;
  justify-content: flex-start;
  gap: var(--fluent-space-sm);
  border: 0;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.sidenav-item:hover:not(:disabled) {
  background: var(--fluent-color-fill-control-hover);
}

.sidenav-item--active {
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-primary);
}

.sidenav-item--active::before {
  content: "";
  position: absolute;
  left: 0;
  top: 6px;
  bottom: 6px;
  width: 3px;
  border-radius: 0 2px 2px 0;
  background: var(--fluent-color-stroke-accent-soft);
}

.sidenav-icon {
  color: var(--fluent-color-fill-accent);
}

.sidenav-item--active .sidenav-icon {
  color: var(--fluent-color-stroke-accent-soft);
}

.sidenav-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sidenav-item--later {
  opacity: 0.55;
  cursor: not-allowed;
}

.later-badge {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.content {
  flex: 1;
  min-width: 0;
  padding: var(--fluent-space-lg);
  overflow: auto;
}

.statusbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  min-height: var(--fluent-statusbar-height);
  flex-shrink: 0;
  padding-inline: var(--fluent-space-md);
  border-top: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-layer-alt);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.status-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-refresh {
  border: 0;
  background: transparent;
  color: var(--fluent-color-fill-accent);
  font: inherit;
  cursor: pointer;
  min-height: var(--fluent-control-height-standard);
  padding-inline: var(--fluent-space-sm);
}
</style>
