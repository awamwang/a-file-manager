<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";

const { t } = useI18n();

type BackendInfo = {
  id: string;
  name: string;
  description: string;
  healthy: boolean;
  capabilities: string[];
  detail?: string | null;
  interfaceOnly?: boolean;
};

type ProviderEntry = {
  name: string;
  uri: string;
  isDir: boolean;
  isFile: boolean;
  size: number;
  localPath?: string | null;
};

type DirListing = {
  uri: string;
  backend: string;
  entries: ProviderEntry[];
  error?: string | null;
};

const backends = ref<BackendInfo[]>([]);
const uri = ref("awam://root");
const listing = ref<DirListing | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

async function loadBackends() {
  backends.value = await invoke<BackendInfo[]>("provider_backends");
}

async function listUri(target?: string) {
  loading.value = true;
  error.value = null;
  listing.value = null;
  try {
    if (target) uri.value = target;
    listing.value = await invoke<DirListing>("provider_list", { uri: uri.value });
    if (listing.value.error) {
      error.value = listing.value.error;
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    loading.value = false;
  }
}

function openEntry(entry: ProviderEntry) {
  if (entry.isDir) {
    void listUri(entry.uri);
  }
}

onMounted(() => {
  void loadBackends()
    .then(() => listUri("awam://root"))
    .catch((err) => {
      error.value = err instanceof Error ? err.message : String(err);
    });
});
</script>

<template>
  <section class="panel">
    <header class="header">
      <h1>{{ t("providers.title") }}</h1>
      <p class="meta">{{ t("providers.meta") }}</p>
    </header>

    <div class="backends">
      <button
        v-for="backend in backends"
        :key="backend.id"
        type="button"
        class="backend"
        :class="{
          'backend--ok': backend.healthy,
          'backend--stub': backend.interfaceOnly,
        }"
        @click="listUri(`awam://${backend.id}`)"
      >
        <strong>{{ backend.name }}</strong>
        <span
          class="fluent-chip status-chip"
          :class="{
            'fluent-chip--stub': backend.interfaceOnly,
            'fluent-chip--ok': !backend.interfaceOnly && backend.healthy,
            'fluent-chip--down': !backend.interfaceOnly && !backend.healthy,
          }"
        >{{
          backend.interfaceOnly
            ? t("providers.interfaceOnly")
            : backend.healthy
              ? t("providers.statusOk")
              : t("providers.statusDown")
        }}</span>
        <span class="detail">{{ backend.detail || backend.description }}</span>
      </button>
    </div>

    <form class="uri-row" @submit.prevent="listUri()">
      <input
        v-model="uri"
        class="fluent-control"
        type="text"
        spellcheck="false"
        :placeholder="t('providers.uriPlaceholder')"
      />
      <button class="fluent-control fluent-control--accent" type="submit" :disabled="loading">
        {{ loading ? t("providers.listing") : t("providers.list") }}
      </button>
    </form>

    <p v-if="error" class="banner banner--error">{{ error }}</p>
    <p v-else-if="listing && listing.entries.length === 0" class="banner">
      {{ t("providers.emptyListing") }}
    </p>

    <div v-if="listing && listing.entries.length" class="list">
      <button
        v-for="entry in listing.entries"
        :key="entry.uri"
        type="button"
        class="fluent-list-row row"
        @dblclick="openEntry(entry)"
      >
        <span class="kind">{{
          entry.isDir ? t("providers.folder") : t("providers.file")
        }}</span>
        <span class="name">{{ entry.name }}</span>
        <span class="path">{{ entry.uri }}</span>
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

.backends {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
  gap: var(--fluent-space-sm);
}

.backend {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--fluent-space-xs);
  min-height: var(--fluent-control-height-comfortable);
  padding: var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.backend--ok {
  border-color: var(--fluent-color-fill-accent);
}

.backend--stub {
  opacity: 0.7;
}

.detail {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.status-chip {
  justify-self: start;
  min-height: var(--fluent-control-height-standard);
  pointer-events: none;
  cursor: default;
  font-size: var(--fluent-font-size-caption);
}

.uri-row {
  display: flex;
  gap: var(--fluent-space-sm);
}

.uri-row .fluent-control:first-child {
  flex: 1;
}

.banner {
  margin: 0;
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  color: var(--fluent-color-text-secondary);
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

.row {
  width: 100%;
  border: 0;
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: 0;
  background: transparent;
  color: inherit;
  text-align: left;
  display: grid;
  grid-template-columns: 4.5rem 10rem 1fr;
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
