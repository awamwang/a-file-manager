<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted } from "vue";
import { useI18n } from "vue-i18n";
import {
  bootstrapNavigatorOpenState,
  currentPath,
  listing,
  listError,
  listLoading,
  navigateTo,
  openEntry,
  type NavigatorEntry,
} from "../state/navigatorBrowse";
import FluentIcon from "../components/icons/FluentIcon.vue";
import { entryKindIcon } from "../components/icons/glyphs";

const { t } = useI18n();

onMounted(() => {
  void (async () => {
    await bootstrapNavigatorOpenState();
    if (currentPath.value && !listing.value && !listError.value && !listLoading.value) {
      await navigateTo(currentPath.value, { syncOpenState: false });
    }
  })();
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
  <section class="fluent-page">
    <header class="fluent-page-header">
      <h1 class="fluent-page-title">{{ t("navigator.title") }}</h1>
      <p class="fluent-page-meta">
        <template v-if="listing">
          {{
            t("navigator.metaConnected", {
              dirs: listing.dirCount,
              files: listing.fileCount,
            })
          }}
        </template>
        <template v-else>{{ t("navigator.metaDisconnected") }}</template>
      </p>
    </header>

    <p v-if="listLoading" class="fluent-banner">{{ t("navigator.loading") }}</p>
    <p v-else-if="listError" class="fluent-banner fluent-banner--error">{{ listError }}</p>
    <p v-else-if="listing && listing.entries.length === 0" class="fluent-banner">
      {{ t("navigator.empty") }}
    </p>

    <div v-if="listing && listing.entries.length" class="fluent-surface list" role="list">
      <button
        v-for="entry in listing.entries"
        :key="entry.path"
        type="button"
        class="fluent-list-row fluent-list-row--density row"
        role="listitem"
        @dblclick="onActivate(entry)"
        @keydown.enter.prevent="onActivate(entry)"
      >
        <span
          class="kind"
          :data-kind="entry.kind"
          :title="entry.kind === 'dir' ? t('navigator.folder') : t('navigator.file')"
        >
          <FluentIcon :name="entryKindIcon(entry.kind === 'dir')" size="md" />
        </span>
        <span class="name">{{ entry.name }}</span>
        <span class="size">{{ formatSize(entry) }}</span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.list {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
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
  grid-template-columns: 2rem 1fr 5.5rem;
  gap: var(--fluent-space-sm);
}

.row:last-child {
  border-bottom: 0;
}

.row:hover,
.row:focus-visible {
  background: var(--fluent-color-fill-selected);
  outline: none;
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
}

.size {
  text-align: right;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  font-variant-numeric: tabular-nums;
}
</style>
