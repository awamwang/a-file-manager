<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import FluentIcon from "../icons/FluentIcon.vue";

export type EntryStat = {
  name: string;
  path: string;
  kind: string;
  size: number;
  modifiedMs: number | null;
  createdMs: number | null;
  readonly: boolean;
  totalSize?: number | null;
  fileCount?: number | null;
  dirCount?: number | null;
  totalSizeTruncated?: boolean;
};

const props = defineProps<{
  path: string | null;
  open: boolean;
}>();

const emit = defineEmits<{ close: [] }>();
const { t, locale } = useI18n();

const loading = ref(false);
const error = ref<string | null>(null);
const stat = ref<EntryStat | null>(null);

const kindLabel = computed(() => {
  if (!stat.value) return "";
  return stat.value.kind === "dir" ? t("properties.kindFolder") : t("properties.kindFile");
});

const rows = computed(() => {
  const s = stat.value;
  if (!s) return [] as Array<{ key: string; label: string; value: string; mono?: boolean }>;
  const list: Array<{ key: string; label: string; value: string; mono?: boolean }> = [
    { key: "name", label: t("properties.name"), value: s.name },
    { key: "kind", label: t("properties.kind"), value: kindLabel.value },
    { key: "path", label: t("properties.path"), value: s.path, mono: true },
  ];
  if (s.kind === "dir") {
    if (s.totalSize != null) {
      const sizeText = formatBytes(s.totalSize);
      list.push({
        key: "totalSize",
        label: t("properties.totalSize"),
        value: s.totalSizeTruncated
          ? `${sizeText} ${t("properties.truncatedHint")}`
          : sizeText,
      });
    }
    if (s.fileCount != null) {
      list.push({
        key: "fileCount",
        label: t("properties.fileCount"),
        value: String(s.fileCount),
      });
    }
    if (s.dirCount != null) {
      list.push({
        key: "dirCount",
        label: t("properties.dirCount"),
        value: String(s.dirCount),
      });
    }
  } else {
    list.push({ key: "size", label: t("properties.size"), value: formatBytes(s.size) });
  }
  list.push(
    { key: "modified", label: t("properties.modified"), value: formatTime(s.modifiedMs) },
    { key: "created", label: t("properties.created"), value: formatTime(s.createdMs) },
    {
      key: "readonly",
      label: t("properties.readonly"),
      value: s.readonly ? t("common.yes") : t("common.no"),
    },
  );
  return list;
});

function formatBytes(size: number): string {
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  if (size < 1024 * 1024 * 1024) return `${(size / (1024 * 1024)).toFixed(1)} MB`;
  return `${(size / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatTime(ms: number | null): string {
  if (ms == null) return t("properties.unknown");
  void locale.value;
  return new Date(ms).toLocaleString();
}

async function load() {
  if (!props.path) {
    stat.value = null;
    return;
  }
  loading.value = true;
  error.value = null;
  try {
    stat.value = await invoke<EntryStat>("navigator_stat", { path: props.path });
  } catch (err) {
    stat.value = null;
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    loading.value = false;
  }
}

watch(
  () => [props.open, props.path] as const,
  ([open]) => {
    if (open) void load();
  },
  { immediate: true },
);
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="props-backdrop"
      role="presentation"
      @click.self="emit('close')"
    >
      <aside class="props-panel fluent-surface" role="dialog" :aria-label="t('properties.title')">
        <header class="props-head">
          <h2>{{ t("properties.title") }}</h2>
          <button type="button" class="fluent-control" :aria-label="t('common.dismiss')" @click="emit('close')">
            <FluentIcon name="close" size="sm" />
          </button>
        </header>
        <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>
        <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>
        <ul v-else-if="stat" class="props-list">
          <li v-for="row in rows" :key="row.key" class="props-row">
            <span class="props-label">{{ row.label }}</span>
            <span class="props-value" :class="{ mono: row.mono }" :title="row.value">{{ row.value }}</span>
          </li>
        </ul>
      </aside>
    </div>
  </Teleport>
</template>

<style scoped>
.props-backdrop {
  position: fixed;
  inset: 0;
  z-index: 90;
  display: flex;
  justify-content: flex-end;
  background: rgb(0 0 0 / 28%);
}

.props-panel {
  width: min(24rem, 100%);
  height: 100%;
  padding: 0;
  border-radius: 0;
  border-left: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  box-shadow: var(--fluent-shadow-flyout, -8px 0 24px rgb(0 0 0 / 18%));
  overflow: auto;
  display: flex;
  flex-direction: column;
}

.props-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.props-head h2 {
  margin: 0;
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.props-list {
  list-style: none;
  margin: 0;
  padding: var(--fluent-space-sm) 0;
}

.props-row {
  display: grid;
  grid-template-columns: 7rem 1fr;
  gap: var(--fluent-space-md);
  align-items: start;
  padding: var(--fluent-space-sm) var(--fluent-space-lg);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.props-row:last-child {
  border-bottom: 0;
}

.props-label {
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  padding-top: 0.1rem;
}

.props-value {
  margin: 0;
  overflow-wrap: anywhere;
  font-size: var(--fluent-font-size-body);
}

.mono {
  font-family: var(--fluent-font-family-mono);
  font-size: var(--fluent-font-size-caption);
}
</style>
