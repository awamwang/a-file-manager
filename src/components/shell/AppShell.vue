<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import CommandPanel from "../command/CommandPanel.vue";
import { buildAppCommands } from "../../commands/registry";
import {
  isToggleCommandPanelShortcut,
  toggleCommandPanel,
} from "../../state/commandPanel";
import {
  currentPath,
  goUp,
  listError,
  listLoading,
  listing,
  navigateTo,
  navigatorTabs,
} from "../../state/navigatorBrowse";

const { t, locale } = useI18n();

type HealthPayload = {
  ok: boolean;
  name: string;
  version: string;
  identifier: string;
};

type NavItem = {
  id: string;
  labelKey: string;
  to?: string;
  later?: boolean;
};

const route = useRoute();
const router = useRouter();

const addressDraft = ref("");
const activeTab = ref("navigator");
const health = ref<HealthPayload | null>(null);
const healthError = ref<string | null>(null);
const isMaximized = ref(false);
const maxHover = ref(false);
const unlisteners: UnlistenFn[] = [];

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

function onGlobalKeydown(event: KeyboardEvent) {
  if (!isToggleCommandPanelShortcut(event)) return;
  event.preventDefault();
  toggleCommandPanel();
}

const navItems: NavItem[] = [
  { id: "navigator", labelKey: "nav.navigator", to: "/navigator" },
  { id: "discovery", labelKey: "nav.discovery", to: "/discovery" },
  { id: "providers", labelKey: "nav.providers", to: "/providers" },
  { id: "work-groups", labelKey: "nav.workGroups", to: "/work-groups" },
  { id: "projects", labelKey: "nav.projects", to: "/projects" },
  { id: "extensions", labelKey: "nav.extensions", to: "/extensions" },
  { id: "settings", labelKey: "nav.settings", to: "/settings" },
  { id: "health", labelKey: "nav.health", to: "/health" },
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
  if (route.path !== "/navigator") {
    await router.push("/navigator");
  }
  await navigateTo(addressDraft.value);
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
  return Boolean(item.to && route.path === item.to);
}

function openTab(id: string, path: string) {
  activeTab.value = id;
  void router.push(path);
}

function tabLabel(path: string) {
  const parts = path.replace(/\\/g, "/").replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || path;
}

async function openNavigatorTab(path: string) {
  activeTab.value = "navigator";
  if (route.path !== "/navigator") {
    await router.push("/navigator");
  }
  await navigateTo(path);
}

onMounted(() => {
  void loadHealth();
  void refreshMaximized();
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
          ─
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
          {{ isMaximized ? "❐" : "□" }}
        </button>
        <button
          id="frame-tb-close"
          type="button"
          class="win-btn win-btn--close"
          :aria-label="t('chrome.close')"
          @click="closeWindow"
        >
          ✕
        </button>
      </div>
    </header>

    <div class="toolbar" role="toolbar" :aria-label="t('chrome.toolbar')">
      <button
        type="button"
        class="fluent-control toolbar-btn"
        disabled
        :title="t('chrome.historyLater')"
      >
        {{ t("chrome.back") }}
      </button>
      <button
        type="button"
        class="fluent-control toolbar-btn"
        disabled
        :title="t('chrome.historyLater')"
      >
        {{ t("chrome.forward") }}
      </button>
      <button
        type="button"
        class="fluent-control toolbar-btn"
        :title="t('chrome.upTitle')"
        @click="onGoUp"
      >
        {{ t("chrome.up") }}
      </button>
      <span class="toolbar-spacer" />
      <button
        type="button"
        class="fluent-control toolbar-btn"
        disabled
        :title="t('chrome.later')"
      >
        {{ t("chrome.view") }}
      </button>
      <button
        type="button"
        class="fluent-control toolbar-btn"
        :title="t('chrome.commandsTitle')"
        aria-keyshortcuts="Control+Shift+P"
        @click="toggleCommandPanel"
      >
        {{ t("chrome.commands") }}
        <span class="toolbar-shortcut">Ctrl+Shift+P</span>
      </button>
    </div>

    <form class="address" :aria-label="t('chrome.address')" @submit.prevent="submitAddress">
      <label class="address-label" for="address-input">{{ t("chrome.path") }}</label>
      <input
        id="address-input"
        v-model="addressDraft"
        class="address-input"
        type="text"
        spellcheck="false"
        autocomplete="off"
        :placeholder="t('chrome.pathPlaceholder')"
      />
      <button type="submit" class="fluent-control address-go">{{ t("chrome.go") }}</button>
    </form>

    <div class="tabstrip" role="tablist" :aria-label="t('chrome.tabs')">
      <template v-if="navigatorTabs.length">
        <button
          v-for="(tabPath, index) in navigatorTabs"
          :key="`${tabPath}-${index}`"
          type="button"
          role="tab"
          class="tab"
          :class="{ 'tab--active': route.path === '/navigator' && currentPath === tabPath }"
          :aria-selected="route.path === '/navigator' && currentPath === tabPath"
          :title="tabPath"
          @click="openNavigatorTab(tabPath)"
        >
          {{ tabLabel(tabPath) }}
        </button>
      </template>
      <button
        v-else
        type="button"
        role="tab"
        class="tab"
        :class="{ 'tab--active': activeTab === 'navigator' || route.path === '/navigator' }"
        :aria-selected="route.path === '/navigator'"
        @click="openTab('navigator', '/navigator')"
      >
        {{ t("nav.navigator") }}
      </button>
      <button
        type="button"
        role="tab"
        class="tab"
        :class="{ 'tab--active': route.path === '/health' }"
        :aria-selected="route.path === '/health'"
        @click="openTab('health', '/health')"
      >
        {{ t("nav.health") }}
      </button>
      <button
        type="button"
        class="tab tab--add"
        disabled
        :title="t('chrome.newTabLater')"
        :aria-label="t('chrome.newTab')"
      >
        +
      </button>
    </div>

    <div class="body">
      <nav class="sidenav" :aria-label="t('chrome.sideNav')">
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
          <span>{{ t(item.labelKey) }}</span>
          <span v-if="item.later" class="later-badge">{{ t("chrome.later") }}</span>
        </button>
      </nav>

      <main class="content">
        <slot />
      </main>
    </div>

    <footer class="statusbar" :aria-label="t('chrome.status')">
      <span>{{ statusText }}</span>
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
  min-width: var(--fluent-control-height-comfortable);
  height: var(--fluent-control-height-standard);
  min-height: var(--fluent-control-height-standard);
  padding-inline: var(--fluent-space-md);
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
  min-height: var(--fluent-control-height-standard);
  padding: 0 var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid transparent;
  border-bottom: 0;
  border-radius: var(--fluent-radius-sm) var(--fluent-radius-sm) 0 0;
  background: transparent;
  color: var(--fluent-color-text-secondary);
  font: inherit;
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

.tab--add {
  min-width: var(--fluent-control-height-standard);
  opacity: 0.45;
  cursor: not-allowed;
}

.body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.sidenav {
  width: var(--fluent-sidenav-width);
  flex-shrink: 0;
  padding: var(--fluent-space-sm);
  border-right: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
}

.sidenav-item {
  width: 100%;
  justify-content: space-between;
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
  background: var(--fluent-color-fill-control);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
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
