<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { navigateTo } from "../state/navigatorBrowse";
import { useRouter } from "vue-router";
import {
  consumeGlobalSearchFocus,
  globalSearchFocusToken,
} from "../state/globalSearch";
import FluentIcon from "../components/icons/FluentIcon.vue";
import { entryKindIcon } from "../components/icons/glyphs";

type DiscoveryEngine = "everything" | "fallback" | "unavailable";

type DiscoveryStatus = {
  everythingAvailable: boolean;
  fallbackEnabled: boolean;
  activeEngine: DiscoveryEngine;
  degraded: boolean;
  detail: string;
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

async function focusSearchInput() {
  await nextTick();
  searchInput.value?.focus();
  searchInput.value?.select();
}

async function refreshStatus() {
  status.value = await invoke<DiscoveryStatus>("discovery_status");
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

    <p v-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>
    <p v-else-if="result && result.hits.length === 0" class="fluent-banner fluent-banner--info">
      {{ t("discovery.noHits", { engine: result.engine, total: result.totalCount }) }}
    </p>

    <div v-if="result && result.hits.length" class="fluent-surface list">
      <p class="fluent-page-meta list-meta">
        {{
          t("discovery.shown", {
            shown: result.hits.length,
            total: result.totalCount,
            engine: result.engine,
          })
        }}
      </p>
      <button
        v-for="hit in result.hits"
        :key="hit.path + hit.name"
        type="button"
        class="fluent-list-row fluent-list-row--density row"
        @dblclick="openHit(hit)"
      >
        <span
          class="kind"
          :data-kind="hit.isDir ? 'dir' : 'file'"
          :title="hit.isDir ? t('discovery.folder') : t('discovery.file')"
        >
          <FluentIcon :name="entryKindIcon(hit.isDir)" size="md" />
        </span>
        <span class="name">{{ hit.name }}</span>
        <span class="path">{{ hit.path }}</span>
      </button>
    </div>
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
