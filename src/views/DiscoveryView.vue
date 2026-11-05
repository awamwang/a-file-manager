<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { openOrFocusNavigatorTab, type NavigatorEntry } from "../state/navigatorBrowse";
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
import { toastError, toastSuccess } from "../state/toast";

type DiscoveryEngine = "everything" | "fallback" | "unavailable";
type DiscoveryIndexState = "ready" | "degraded" | "unavailable";
type EngineCardId = "everything" | "fallback";

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
const selectedCard = ref<EngineCardId>("everything");
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

const everythingStatusText = computed(() => {
  if (!status.value) return "";
  if (status.value.everythingAvailable) {
    if (status.value.estimatedItemCount != null) {
      return t("discovery.everythingReadyCount", {
        count: status.value.estimatedItemCount,
      });
    }
    return t("discovery.everythingReady");
  }
  return t("discovery.everythingOffline");
});

const fallbackStatusText = computed(() => {
  if (!status.value) return "";
  if (!status.value.fallbackEnabled) return t("discovery.fallbackOff");
  if (status.value.activeEngine === "fallback") return t("discovery.fallbackActive");
  return t("discovery.fallbackStandby");
});

const resultEngineLabel = computed(() => {
  const engine = result.value?.engine;
  if (!engine || engine === "unavailable") return t("discovery.engines.unavailable");
  return t(`discovery.engines.${engine}`);
});

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
  toastSuccess(message);
}

async function focusSearchInput() {
  await nextTick();
  searchInput.value?.focus();
  searchInput.value?.select();
}

async function refreshStatus() {
  status.value = await invoke<DiscoveryStatus>("discovery_status");
  blacklistDraft.value = (status.value.blacklist ?? []).join("\n");
  // Prefer showing the card that matches live engine when first loaded.
  if (status.value.activeEngine === "fallback") {
    selectedCard.value = "fallback";
  } else {
    selectedCard.value = "everything";
  }
}

async function selectEngineCard(card: EngineCardId) {
  selectedCard.value = card;
  if (!status.value) return;
  const wantFallback = card === "fallback";
  if (status.value.fallbackEnabled === wantFallback) return;
  status.value = await invoke<DiscoveryStatus>("discovery_set_fallback_enabled", {
    enabled: wantFallback,
  });
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
    const msg = err instanceof Error ? err.message : String(err);
    error.value = msg;
    toastError(msg);
  } finally {
    blacklistSaving.value = false;
  }
}

async function runSearch() {
  loading.value = true;
  error.value = null;
  result.value = null;
  try {
    status.value = await invoke<DiscoveryStatus>("discovery_status");
    result.value = await invoke<DiscoverySearchResult>("discovery_search", {
      query: query.value,
      options: { offset: 0, limit: 50 },
    });
    requestShellIcons((result.value?.hits ?? []).map((h) => normalizeHitPath(h.path)));
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    error.value = msg;
    toastError(msg);
  } finally {
    loading.value = false;
  }
}

async function openHit(hit: DiscoveryHit) {
  const path = hit.path.replace(/\//g, "\\");
  if (hit.isDir) {
    await openOrFocusNavigatorTab(path);
  } else {
    const parent = path.includes("\\") ? path.slice(0, path.lastIndexOf("\\")) : path;
    await openOrFocusNavigatorTab(parent || path);
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

    <div class="engine-cards" role="radiogroup" :aria-label="t('discovery.engineCards')">
      <button
        type="button"
        class="fluent-surface engine-card"
        :class="{ 'engine-card--active': selectedCard === 'everything' }"
        role="radio"
        :aria-checked="selectedCard === 'everything'"
        @click="selectEngineCard('everything')"
      >
        <span class="engine-card-icon">
          <FluentIcon name="search" size="md" />
        </span>
        <strong>{{ t("discovery.engines.everything") }}</strong>
        <span class="engine-card-status">{{ everythingStatusText }}</span>
      </button>
      <button
        type="button"
        class="fluent-surface engine-card"
        :class="{ 'engine-card--active': selectedCard === 'fallback' }"
        role="radio"
        :aria-checked="selectedCard === 'fallback'"
        @click="selectEngineCard('fallback')"
      >
        <span class="engine-card-icon">
          <FluentIcon name="folder" size="md" />
        </span>
        <strong>{{ t("discovery.engines.fallback") }}</strong>
        <span class="engine-card-status">{{ fallbackStatusText }}</span>
      </button>
    </div>

    <div
      v-if="status && selectedCard === 'everything'"
      class="fluent-surface config-panel"
      data-engine-config="everything"
    >
      <h2 class="config-title">{{ t("discovery.everythingConfigTitle") }}</h2>
      <p class="config-copy">{{ everythingStatusText }}</p>
      <p class="config-copy">{{ t("discovery.everythingConfigHint") }}</p>

      <h3 class="config-subtitle">{{ t("discovery.blacklistTitle") }}</h3>
      <p class="config-copy">{{ t("discovery.blacklistHint") }}</p>
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
        <span class="config-copy">
          {{ t("discovery.blacklistCount", { count: status.blacklist.length }) }}
        </span>
      </div>
    </div>

    <div
      v-else-if="status && selectedCard === 'fallback'"
      class="fluent-surface config-panel"
      data-engine-config="fallback"
    >
      <h2 class="config-title">{{ t("discovery.fallbackConfigTitle") }}</h2>
      <p class="config-copy">{{ fallbackStatusText }}</p>
      <p class="config-copy">{{ t("discovery.fallbackConfigHint") }}</p>
      <p class="config-copy">{{ t("discovery.fallbackBlacklistNote") }}</p>
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

    <p v-if="result && result.hits.length === 0" class="fluent-banner fluent-banner--info">
      {{ t("discovery.noHits", { engine: resultEngineLabel, total: result.totalCount }) }}
    </p>

    <div v-if="result && result.hits.length" class="fluent-surface list" role="list">
      <p class="fluent-page-meta list-meta">
        {{
          t("discovery.shown", {
            shown: result.hits.length,
            total: result.totalCount,
            engine: resultEngineLabel,
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
.engine-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
  gap: var(--fluent-space-md);
}

.engine-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--fluent-space-xs);
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  text-align: left;
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
  color: inherit;
  cursor: pointer;
}

.engine-card:hover {
  background: var(--fluent-color-fill-control-hover);
}

.engine-card--active {
  border-color: var(--fluent-color-fill-accent);
  box-shadow: inset 0 0 0 1px var(--fluent-color-fill-accent);
}

.engine-card-icon {
  color: var(--fluent-color-fill-accent);
}

.engine-card-status {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  line-height: 1.4;
}

.config-panel {
  margin-top: var(--fluent-space-md);
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.config-title {
  margin: 0;
  font-size: var(--fluent-font-size-body);
  font-weight: var(--fluent-font-weight-semibold);
}

.config-subtitle {
  margin: var(--fluent-space-sm) 0 0;
  font-size: var(--fluent-font-size-body);
  font-weight: var(--fluent-font-weight-semibold);
}

.config-copy {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  line-height: 1.45;
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
  margin-top: var(--fluent-space-md);
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
