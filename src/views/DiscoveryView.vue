<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { navigateTo, type NavigatorEntry } from "../state/navigatorBrowse";
import { useRouter } from "vue-router";
import {
  consumeGlobalSearchFocus,
  globalSearchFocusToken,
} from "../state/globalSearch";
import { iconUrlFor, requestShellIcons } from "../state/shellIcons";
import { loadFavorites } from "../state/favorites";
import FluentIcon from "../components/icons/FluentIcon.vue";
import NavigatorContextMenu from "../components/navigator/NavigatorContextMenu.vue";
import { entryKindIcon } from "../components/icons/glyphs";

type DiscoveryEngine = "everything" | "fallback" | "unavailable";
type DiscoveryIndexState = "ready" | "degraded" | "unavailable";

type DiscoveryStatus = {
  everythingAvailable: boolean;
  fallbackEnabled: boolean;
  activeEngine: DiscoveryEngine;
  degraded: boolean;
  detail: string;
  indexState: DiscoveryIndexState;
  estimatedItemCount: number | null;
  blacklist: string[];
};

type DiscoveryHit = {
  name: string;
  path: string;
  isDir: boolean;
};

type DiscoverySearchResult = {
  query: string;
  engine: DiscoveryEngine;
  totalCount: number;
  offset: number;
  hits: DiscoveryHit[];
};

const { t } = useI18n();
const router = useRouter();
const status = ref<DiscoveryStatus | null>(null);
const query = ref("");
const result = ref<DiscoverySearchResult | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
const searchInput = ref<HTMLInputElement | null>(null);
const blacklistDraft = ref("");
const blacklistSaving = ref(false);
const selectedPath = ref<string | null>(null);
const menuOpen = ref(false);
const menuEntry = ref<NavigatorEntry | null>(null);
const menuX = ref(0);
const menuY = ref(0);
const notice = ref<string | null>(null);
let noticeTimer: ReturnType<typeof setTimeout> | null = null;

function normalizeHitPath(path: string): string {
  return path.replace(/\//g, "\\");
}

function hitToEntry(hit: DiscoveryHit): NavigatorEntry {
  return {
    name: hit.name,
    path: normalizeHitPath(hit.path),
    kind: hit.isDir ? "dir" : "file",
    size: 0,
  };
}

function hitIcon(hit: DiscoveryHit): string | null {
  return iconUrlFor(normalizeHitPath(hit.path));
}

function onHitClick(hit: DiscoveryHit) {
  selectedPath.value = hit.path;
}

function onHitContext(event: MouseEvent, hit: DiscoveryHit) {
  event.preventDefault();
  selectedPath.value = hit.path;
  menuEntry.value = hitToEntry(hit);
  menuX.value = event.clientX;
  menuY.value = event.clientY;
  menuOpen.value = true;
}

function onNotice(message: string) {
  notice.value = message;
  if (noticeTimer) clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => {
    notice.value = null;
    noticeTimer = null;
  }, 2800);
}

async function focusSearchInput() {
  await nextTick();
  searchInput.value?.focus();
  searchInput.value?.select();
}

async function refreshStatus() {
  status.value = await invoke<DiscoveryStatus>("discovery_status");
  blacklistDraft.value = (status.value.blacklist ?? []).join("\n");
}

async function saveBlacklist() {
  blacklistSaving.value = true;
  error.value = null;
  try {
    const paths = blacklistDraft.value
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean);
    status.value = await invoke<DiscoveryStatus>("discovery_set_blacklist", {
      paths,
    });
    blacklistDraft.value = (status.value.blacklist ?? []).join("\n");
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    blacklistSaving.value = false;
  }
}

async function runSearch() {
  loading.value = true;
  error.value = null;
  result.value = null;
  try {
    await refreshStatus();
    result.value = await invoke<DiscoverySearchResult>("discovery_search", {
      query: query.value,
      options: { offset: 0, limit: 50 },
    });
    requestShellIcons((result.value?.hits ?? []).map((h) => normalizeHitPath(h.path)));
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    loading.value = false;
  }
}

async function toggleFallback() {
  if (!status.value) return;
  status.value = await invoke<DiscoveryStatus>("discovery_set_fallback_enabled", {
    enabled: !status.value.fallbackEnabled,
  });
}

async function openHit(hit: DiscoveryHit) {
  const path = hit.path.replace(/\//g, "\\");
  if (hit.isDir) {
    await navigateTo(path);
  } else {
    const parent = path.includes("\\") ? path.slice(0, path.lastIndexOf("\\")) : path;
    await navigateTo(parent || path);
  }
  await router.push("/navigator");
}

watch(globalSearchFocusToken, () => {
  if (consumeGlobalSearchFocus()) {
    void focusSearchInput();
  }
});

onMounted(() => {
  void refreshStatus().catch((err) => {
    error.value = err instanceof Error ? err.message : String(err);
  });
  void loadFavorites().catch(() => undefined);
  if (consumeGlobalSearchFocus()) {
    void focusSearchInput();
  }
});
</script>

<template>
  <section class="fluent-page">
    <header class="fluent-page-header">
      <h1 class="fluent-page-title">{{ t("discovery.title") }}</h1>
      <p class="fluent-page-meta">{{ t("discovery.meta") }}</p>
    </header>

    <div
      v-if="status"
      class="fluent-surface status-card"
      :class="{
        'status-card--ok': status.activeEngine === 'everything' && !status.degraded,
        'status-card--warn': status.degraded || status.activeEngine === 'fallback',
        'status-card--down': status.activeEngine === 'unavailable',
      }"
    >
      <div class="status-row">
        <span>{{ t("discovery.engine") }}</span>
        <span
          class="fluent-chip engine-chip"
          :class="{
            'fluent-chip--ok': status.activeEngine === 'everything',
            'fluent-chip--stub': status.activeEngine === 'fallback',
            'fluent-chip--down': status.activeEngine === 'unavailable',
          }"
        >{{ status.activeEngine }}</span>
        <span
          v-if="status.degraded"
          class="fluent-chip fluent-chip--stub degraded-chip"
        >{{ t("discovery.degraded") }}</span>
      </div>
      <div class="status-row">
        <span>{{ t("discovery.indexState") }}</span>
        <span class="fluent-chip">{{ t(`discovery.indexStates.${status.indexState}`) }}</span>
        <span v-if="status.estimatedItemCount != null" class="index-count">
          {{ t("discovery.indexCount", { count: status.estimatedItemCount }) }}
        </span>
        <span v-else class="index-count">{{ t("discovery.indexCountUnknown") }}</span>
      </div>
      <p class="detail">{{ status.detail }}</p>
      <label class="fallback">
        <input
          type="checkbox"
          class="fluent-field-check"
          :checked="status.fallbackEnabled"
          @change="toggleFallback"
        />
        {{ t("discovery.fallback") }}
      </label>
    </div>

    <div class="fluent-surface blacklist-card">
      <h2 class="blacklist-title">{{ t("discovery.blacklistTitle") }}</h2>
      <p class="detail">{{ t("discovery.blacklistHint") }}</p>
      <textarea
        v-model="blacklistDraft"
        class="fluent-control blacklist-input"
        rows="4"
        :placeholder="t('discovery.blacklistPlaceholder')"
        spellcheck="false"
      />
      <div class="blacklist-actions">
        <button
          type="button"
          class="fluent-control fluent-control--accent"
          :disabled="blacklistSaving"
          @click="saveBlacklist"
        >
          {{ blacklistSaving ? t("discovery.blacklistSaving") : t("discovery.blacklistSave") }}
        </button>
        <span class="detail">
          {{ t("discovery.blacklistCount", { count: status?.blacklist.length ?? 0 }) }}
        </span>
      </div>
    </div>

    <form class="search" @submit.prevent="runSearch">
      <span class="search-icon" aria-hidden="true">
        <FluentIcon name="search" size="md" />
      </span>
      <input
        id="discovery-global-search"
        ref="searchInput"
        v-model="query"
        class="fluent-control query"
        type="search"
        :placeholder="t('discovery.placeholder')"
        spellcheck="false"
        aria-keyshortcuts="Control+Shift+F"
      />
      <button class="fluent-control fluent-control--accent" type="submit" :disabled="loading">
        {{ loading ? t("discovery.searching") : t("discovery.search") }}
      </button>
    </form>

    <p v-if="notice" class="fluent-banner fluent-banner--success">{{ notice }}</p>
    <p v-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>
    <p v-else-if="result && result.hits.length === 0" class="fluent-banner fluent-banner--info">
      {{ t("discovery.noHits", { engine: result.engine, total: result.totalCount }) }}
    </p>

    <div v-if="result && result.hits.length" class="fluent-surface list" role="list">
      <p class="fluent-page-meta list-meta">
        {{
          t("discovery.shown", {
            shown: result.hits.length,
            total: result.totalCount,
            engine: result.engine,
          })
        }}
      </p>
      <div
        v-for="hit in result.hits"
        :key="hit.path + hit.name"
        class="fluent-list-row fluent-list-row--density row"
        role="listitem"
        tabindex="0"
        :aria-selected="selectedPath === hit.path"
        :class="{ 'row--selected': selectedPath === hit.path }"
        @click="onHitClick(hit)"
        @dblclick="openHit(hit)"
        @keydown.enter.prevent="openHit(hit)"
        @contextmenu="onHitContext($event, hit)"
      >
        <span
          class="kind"
          :data-kind="hit.isDir ? 'dir' : 'file'"
          :title="hit.isDir ? t('discovery.folder') : t('discovery.file')"
        >
          <img
            v-if="hitIcon(hit)"
            class="shell-icon"
            :src="hitIcon(hit)!"
            alt=""
            width="16"
            height="16"
          />
          <FluentIcon v-else :name="entryKindIcon(hit.isDir)" size="md" />
        </span>
        <span class="name">{{ hit.name }}</span>
        <span class="path">{{ hit.path }}</span>
      </div>
    </div>

    <NavigatorContextMenu
      :open="menuOpen"
      :entry="menuEntry"
      :x="menuX"
      :y="menuY"
      @close="menuOpen = false"
      @notice="onNotice"
      @favorites-changed="loadFavorites"
    />
  </section>
</template>

<style scoped>
.status-card {
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.status-card--ok {
  background: var(--fluent-color-status-success-bg);
  border-color: var(--fluent-color-status-success);
}

.status-card--warn {
  background: var(--fluent-color-status-warning-bg);
  border-color: var(--fluent-color-status-warning);
}

.status-card--down {
  background: var(--fluent-color-status-danger-bg);
  border-color: var(--fluent-color-status-danger);
}

.status-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fluent-space-sm);
}

.engine-chip,
.degraded-chip {
  min-height: var(--fluent-control-height-standard);
  pointer-events: none;
  cursor: default;
  font-size: var(--fluent-font-size-caption);
}

.detail {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.fallback {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  font-size: var(--fluent-font-size-caption);
}

.index-count {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.blacklist-card {
  margin-top: var(--fluent-space-md);
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.blacklist-title {
  margin: 0;
  font-size: var(--fluent-font-size-body);
  font-weight: var(--fluent-font-weight-semibold);
}

.blacklist-input {
  width: 100%;
  min-height: 5.5rem;
  resize: vertical;
  font-family: var(--fluent-font-family-monospace, Consolas, monospace);
}

.blacklist-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fluent-space-sm);
}

.search {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
}

.search-icon {
  display: inline-flex;
  color: var(--fluent-color-fill-accent);
}

.query {
  flex: 1;
}

.list {
  display: flex;
  flex-direction: column;
}

.list-meta {
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.row {
  width: 100%;
  border: 0;
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: 0;
  background: transparent;
  color: inherit;
  text-align: left;
  display: grid;
  grid-template-columns: 2rem 12rem 1fr;
  gap: var(--fluent-space-sm);
  cursor: pointer;
}

.row:last-child {
  border-bottom: 0;
}

.row--selected {
  background: var(--fluent-color-fill-selected);
}

.shell-icon {
  width: 16px;
  height: 16px;
  object-fit: contain;
}

.row:hover {
  background: var(--fluent-color-fill-subtle);
}

.kind {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--fluent-color-text-secondary);
}

.kind[data-kind="dir"] {
  color: var(--fluent-color-fill-accent);
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: var(--fluent-font-weight-semibold);
}

.path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
</style>
