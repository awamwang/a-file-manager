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
        <dl v-else-if="stat" class="props-grid">
          <dt>{{ t("properties.name") }}</dt>
          <dd>{{ stat.name }}</dd>
          <dt>{{ t("properties.kind") }}</dt>
          <dd>{{ kindLabel }}</dd>
          <dt>{{ t("properties.path") }}</dt>
          <dd class="mono" :title="stat.path">{{ stat.path }}</dd>
          <dt v-if="stat.kind !== 'dir'">{{ t("properties.size") }}</dt>
          <dd v-if="stat.kind !== 'dir'">{{ formatBytes(stat.size) }}</dd>
          <dt>{{ t("properties.modified") }}</dt>
          <dd>{{ formatTime(stat.modifiedMs) }}</dd>
          <dt>{{ t("properties.created") }}</dt>
          <dd>{{ formatTime(stat.createdMs) }}</dd>
          <dt>{{ t("properties.readonly") }}</dt>
          <dd>{{ stat.readonly ? t("common.yes") : t("common.no") }}</dd>
        </dl>
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
  width: min(22rem, 100%);
  height: 100%;
  padding: var(--fluent-space-lg);
  border-radius: 0;
  border-left: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  box-shadow: var(--fluent-shadow-flyout, -8px 0 24px rgb(0 0 0 / 18%));
  overflow: auto;
}

.props-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-sm);
  margin-bottom: var(--fluent-space-md);
}

.props-head h2 {
  margin: 0;
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.props-grid {
  display: grid;
  grid-template-columns: 6.5rem 1fr;
  gap: var(--fluent-space-sm) var(--fluent-space-md);
  margin: 0;
}

.props-grid dt {
  color: var(--fluent-color-text-secondary);
}

.props-grid dd {
  margin: 0;
  overflow-wrap: anywhere;
}

.mono {
  font-family: var(--fluent-font-family-mono);
  font-size: var(--fluent-font-size-caption);
}
</style>
