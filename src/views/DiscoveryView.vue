<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { navigateTo } from "../state/navigatorBrowse";
import { useRouter } from "vue-router";

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

onMounted(() => {
  void refreshStatus().catch((err) => {
    error.value = err instanceof Error ? err.message : String(err);
  });
});
</script>

<template>
  <section class="panel">
    <header class="header">
      <h1>{{ t("discovery.title") }}</h1>
      <p class="meta">{{ t("discovery.meta") }}</p>
    </header>

    <div v-if="status" class="status-card">
      <p>
        {{ t("discovery.engine") }}:
        <strong>{{ status.activeEngine }}</strong>
        <span v-if="status.degraded"> · {{ t("discovery.degraded") }}</span>
      </p>
      <p class="detail">{{ status.detail }}</p>
      <label class="fallback">
        <input
          type="checkbox"
          :checked="status.fallbackEnabled"
          @change="toggleFallback"
        />
        {{ t("discovery.fallback") }}
      </label>
    </div>

    <form class="search" @submit.prevent="runSearch">
      <input
        v-model="query"
        class="fluent-control query"
        type="search"
        :placeholder="t('discovery.placeholder')"
        spellcheck="false"
      />
      <button class="fluent-control fluent-control--accent" type="submit" :disabled="loading">
        {{ loading ? t("discovery.searching") : t("discovery.search") }}
      </button>
    </form>

    <p v-if="error" class="banner banner--error">{{ error }}</p>
    <p v-else-if="result && result.hits.length === 0" class="banner">
      {{ t("discovery.noHits", { engine: result.engine, total: result.totalCount }) }}
    </p>

    <div v-if="result && result.hits.length" class="list">
      <p class="meta">
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
        class="fluent-list-row row"
        @dblclick="openHit(hit)"
      >
        <span class="kind">{{ hit.isDir ? t("discovery.folder") : t("discovery.file") }}</span>
        <span class="name">{{ hit.name }}</span>
        <span class="path">{{ hit.path }}</span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.panel {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}

.header {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
}

h1 {
  margin: 0;
  font-size: var(--fluent-font-size-title);
  font-weight: var(--fluent-font-weight-semibold);
}

.meta {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.status-card,
.banner {
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
}

.status-card p {
  margin: 0 0 var(--fluent-space-sm);
}

.detail {
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
  gap: var(--fluent-space-sm);
}

.query {
  flex: 1;
}

.banner--error {
  color: var(--fluent-color-status-danger);
}

.list {
  display: flex;
  flex-direction: column;
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  overflow: hidden;
}

.list > .meta {
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
  grid-template-columns: 4.5rem 12rem 1fr;
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
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
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
