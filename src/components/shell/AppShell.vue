<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  currentPath,
  goUp,
  listError,
  listLoading,
  listing,
  navigateTo,
} from "../../state/navigatorBrowse";

type HealthPayload = {
  ok: boolean;
  name: string;
  version: string;
  identifier: string;
};

type NavItem = {
  id: string;
  label: string;
  to?: string;
  later?: boolean;
};

const route = useRoute();
const router = useRouter();

const addressDraft = ref("");
const activeTab = ref("navigator");
const health = ref<HealthPayload | null>(null);
const healthError = ref<string | null>(null);

const navItems: NavItem[] = [
  { id: "navigator", label: "Navigator", to: "/navigator" },
  { id: "discovery", label: "Discovery", to: "/discovery" },
  { id: "providers", label: "Providers", to: "/providers" },
  { id: "projects", label: "Projects", later: true },
  { id: "work-groups", label: "Work Groups", later: true },
  { id: "settings", label: "Settings", to: "/settings" },
  { id: "health", label: "Health", to: "/health" },
];

watch(
  currentPath,
  (path) => {
    addressDraft.value = path;
  },
  { immediate: true },
);

const statusText = computed(() => {
  const healthPart = healthError.value
    ? `health: error`
    : health.value?.ok
      ? `health: ok · v${health.value.version}`
      : "health: …";

  if (route.path === "/navigator") {
    if (listLoading.value) return `${healthPart} · navigator: loading`;
    if (listError.value) return `${healthPart} · navigator: error`;
    if (listing.value) {
      return `${healthPart} · ${listing.value.dirCount} folders · ${listing.value.fileCount} files`;
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

async function minimizeWindow() {
  await getCurrentWindow().minimize();
}

async function toggleMaximizeWindow() {
  await getCurrentWindow().toggleMaximize();
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

onMounted(() => {
  void loadHealth();
});
</script>

<template>
  <div class="shell fluent-chrome">
    <header class="titlebar" aria-label="Title bar">
      <div class="titlebar-drag" data-tauri-drag-region>
        <span class="brand" data-tauri-drag-region>A File Manager</span>
        <span class="caption-meta" data-tauri-drag-region>Standard · S5</span>
      </div>
      <div class="window-controls" aria-label="Window controls">
        <button type="button" class="win-btn" aria-label="Minimize" @click="minimizeWindow">
          ─
        </button>
        <!-- Snap Layouts deferred to S9 — maximize hit target only -->
        <button
          type="button"
          class="win-btn"
          aria-label="Maximize"
          @click="toggleMaximizeWindow"
        >
          □
        </button>
        <button type="button" class="win-btn win-btn--close" aria-label="Close" @click="closeWindow">
          ✕
        </button>
      </div>
    </header>

    <div class="toolbar" role="toolbar" aria-label="Main toolbar">
      <button type="button" class="fluent-control toolbar-btn" disabled title="History later">Back</button>
      <button type="button" class="fluent-control toolbar-btn" disabled title="History later">Forward</button>
      <button type="button" class="fluent-control toolbar-btn" title="Go up" @click="onGoUp">Up</button>
      <span class="toolbar-spacer" />
      <button type="button" class="fluent-control toolbar-btn" disabled title="Later">View</button>
    </div>

    <form class="address" aria-label="Address bar" @submit.prevent="submitAddress">
      <label class="address-label" for="address-input">Path</label>
      <input
        id="address-input"
        v-model="addressDraft"
        class="address-input"
        type="text"
        spellcheck="false"
        autocomplete="off"
        placeholder="Native OS path (Kernel) — Enter to go"
      />
      <button type="submit" class="fluent-control address-go">Go</button>
    </form>

    <div class="tabstrip" role="tablist" aria-label="Tabs">
      <button
        type="button"
        role="tab"
        class="tab"
        :class="{ 'tab--active': activeTab === 'navigator' || route.path === '/navigator' }"
        :aria-selected="route.path === '/navigator'"
        @click="openTab('navigator', '/navigator')"
      >
        Navigator
      </button>
      <button
        type="button"
        role="tab"
        class="tab"
        :class="{ 'tab--active': route.path === '/health' }"
        :aria-selected="route.path === '/health'"
        @click="openTab('health', '/health')"
      >
        Health
      </button>
      <button type="button" class="tab tab--add" disabled title="Multi-tab later" aria-label="New tab">
        +
      </button>
    </div>

    <div class="body">
      <nav class="sidenav" aria-label="Side navigation">
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
          :title="item.later ? 'Later phase' : undefined"
          @click="item.to && router.push(item.to)"
        >
          <span>{{ item.label }}</span>
          <span v-if="item.later" class="later-badge">later</span>
        </button>
      </nav>

      <main class="content">
        <slot />
      </main>
    </div>

    <footer class="statusbar" aria-label="Status">
      <span>{{ statusText }}</span>
      <button type="button" class="status-refresh" @click="loadHealth">Refresh health</button>
    </footer>
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
  width: 46px;
  height: var(--fluent-titlebar-height);
  border: 0;
  background: transparent;
  color: var(--fluent-color-text-primary);
  font-size: 12px;
  line-height: 1;
}

.win-btn:hover {
  background: var(--fluent-color-fill-subtle);
}

.win-btn--close:hover {
  background: #c42b1c;
  color: #fff;
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

.toolbar-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.toolbar-spacer {
  flex: 1;
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

.address-input:focus {
  outline: 2px solid var(--fluent-color-fill-accent);
  outline-offset: 1px;
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

.tab--active {
  background: var(--fluent-color-fill-control);
  border-color: var(--fluent-color-stroke-divider);
  color: var(--fluent-color-text-primary);
  font-weight: var(--fluent-font-weight-semibold);
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
  width: 180px;
  flex-shrink: 0;
  padding: var(--fluent-space-sm);
  border-right: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
  overflow: auto;
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
  background: var(--fluent-color-fill-subtle);
}

.sidenav-item--active {
  background: var(--fluent-color-fill-subtle);
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
  min-height: 28px;
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
