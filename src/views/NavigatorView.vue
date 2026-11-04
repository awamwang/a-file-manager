<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted } from "vue";
import {
  currentPath,
  listing,
  listError,
  listLoading,
  loadDefaultPath,
  navigateTo,
  openEntry,
  type NavigatorEntry,
} from "../state/navigatorBrowse";

onMounted(() => {
  if (!currentPath.value) {
    void loadDefaultPath();
  } else if (!listing.value && !listError.value) {
    void navigateTo(currentPath.value);
  }
});

function formatSize(entry: NavigatorEntry): string {
  if (entry.kind === "dir") return "—";
  if (entry.size < 1024) return `${entry.size} B`;
  if (entry.size < 1024 * 1024) return `${(entry.size / 1024).toFixed(1)} KB`;
  return `${(entry.size / (1024 * 1024)).toFixed(1)} MB`;
}

function onActivate(entry: NavigatorEntry) {
  void openEntry(entry);
}
</script>

<template>
  <section class="panel">
    <header class="header">
      <h1>Navigator</h1>
      <p class="meta">
        Kernel OS browse ·
        <template v-if="listing">
          {{ listing.dirCount }} folders · {{ listing.fileCount }} files
        </template>
        <template v-else>not connected</template>
      </p>
    </header>

    <p v-if="listLoading" class="banner">Loading…</p>
    <p v-else-if="listError" class="banner banner--error">{{ listError }}</p>
    <p v-else-if="listing && listing.entries.length === 0" class="banner">
      This folder is empty.
    </p>

    <div v-if="listing && listing.entries.length" class="list" role="list">
      <button
        v-for="entry in listing.entries"
        :key="entry.path"
        type="button"
        class="fluent-list-row row"
        role="listitem"
        @dblclick="onActivate(entry)"
        @keydown.enter.prevent="onActivate(entry)"
      >
        <span class="kind" :data-kind="entry.kind">{{ entry.kind === "dir" ? "Folder" : "File" }}</span>
        <span class="name">{{ entry.name }}</span>
        <span class="size">{{ formatSize(entry) }}</span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.panel {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  min-height: 100%;
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

.banner {
  margin: 0;
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  color: var(--fluent-color-text-secondary);
}

.banner--error {
  color: #c42b1c;
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
  cursor: default;
  display: grid;
  grid-template-columns: 4.5rem 1fr 5.5rem;
  gap: var(--fluent-space-sm);
}

.row:last-child {
  border-bottom: 0;
}

.row:hover,
.row:focus-visible {
  background: var(--fluent-color-fill-subtle);
  outline: none;
}

.kind {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.kind[data-kind="dir"] {
  color: var(--fluent-color-fill-accent);
  font-weight: var(--fluent-font-weight-semibold);
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.size {
  text-align: right;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  font-variant-numeric: tabular-nums;
}
</style>
