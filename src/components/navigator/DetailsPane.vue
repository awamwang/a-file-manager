<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import FluentIcon from "../icons/FluentIcon.vue";
import { formatBytes } from "../../state/formatBytes";
import type { NavigatorEntry } from "../../state/navigatorBrowse";
import {
  SIDE_PANE_MAX_WIDTH,
  SIDE_PANE_MIN_WIDTH,
  applyDetailsPaneWidth,
  detailsPaneWidth,
  loadDetailsPaneWidth,
  setDetailsPaneWidth,
} from "../../state/sidePaneWidth";

type EntryStat = {
  name: string;
  path: string;
  kind: string;
  size: number;
  modifiedMs: number | null;
  createdMs: number | null;
};

const props = defineProps<{
  enabled: boolean;
  /** Selected entry paths (may be empty). */
  paths: string[];
  /** Optional listing rows for quick multi-select size without IPC. */
  entries?: NavigatorEntry[];
}>();

const emit = defineEmits<{ close: [] }>();
const { t, locale } = useI18n();

const loading = ref(false);
const error = ref<string | null>(null);
const stat = ref<EntryStat | null>(null);
let seq = 0;

const resizing = ref(false);
let resizeStartX = 0;
let resizeStartWidth = detailsPaneWidth.value;

onMounted(() => {
  loadDetailsPaneWidth();
});

function onResizeStart(event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  resizing.value = true;
  resizeStartX = event.clientX;
  resizeStartWidth = detailsPaneWidth.value;
  const target = event.currentTarget;
  if (target instanceof HTMLElement) {
    target.setPointerCapture(event.pointerId);
  }
  window.addEventListener("pointermove", onResizeMove);
  window.addEventListener("pointerup", onResizeEnd);
  window.addEventListener("pointercancel", onResizeEnd);
}

function onResizeMove(event: PointerEvent) {
  if (!resizing.value) return;
  const delta = resizeStartX - event.clientX;
  applyDetailsPaneWidth(resizeStartWidth + delta, window.innerWidth);
}

function onResizeEnd() {
  if (!resizing.value) return;
  resizing.value = false;
  window.removeEventListener("pointermove", onResizeMove);
  window.removeEventListener("pointerup", onResizeEnd);
  window.removeEventListener("pointercancel", onResizeEnd);
  setDetailsPaneWidth(detailsPaneWidth.value, window.innerWidth);
}

function onSplitterKey(event: KeyboardEvent) {
  const step = event.shiftKey ? 32 : 8;
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    setDetailsPaneWidth(detailsPaneWidth.value + step, window.innerWidth);
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    setDetailsPaneWidth(detailsPaneWidth.value - step, window.innerWidth);
  } else if (event.key === "Home") {
    event.preventDefault();
    setDetailsPaneWidth(SIDE_PANE_MIN_WIDTH, window.innerWidth);
  } else if (event.key === "End") {
    event.preventDefault();
    setDetailsPaneWidth(SIDE_PANE_MAX_WIDTH, window.innerWidth);
  }
}

const singlePath = computed(() => (props.paths.length === 1 ? props.paths[0]! : null));

const multiSummary = computed(() => {
  if (props.paths.length <= 1) return null;
  const known = (props.entries ?? []).filter((entry) => props.paths.includes(entry.path));
  const fileBytes = known
    .filter((entry) => entry.kind !== "dir")
    .reduce((sum, entry) => sum + (entry.size || 0), 0);
  return {
    count: props.paths.length,
    known: known.length,
    fileBytes,
  };
});

const rows = computed(() => {
  const s = stat.value;
  if (!s) return [] as Array<{ key: string; label: string; value: string; mono?: boolean }>;
  return [
    { key: "name", label: t("detailsPane.name"), value: s.name },
    {
      key: "kind",
      label: t("detailsPane.kind"),
      value: s.kind === "dir" ? t("detailsPane.kindFolder") : t("detailsPane.kindFile"),
    },
    { key: "size", label: t("detailsPane.size"), value: formatBytes(s.size) },
    {
      key: "modified",
      label: t("detailsPane.modified"),
      value: formatTime(s.modifiedMs),
    },
    { key: "path", label: t("detailsPane.path"), value: s.path, mono: true },
  ];
});

function formatTime(ms: number | null): string {
  if (ms == null) return t("detailsPane.unknown");
  try {
    return new Date(ms).toLocaleString(locale.value);
  } catch {
    return String(ms);
  }
}

async function load(path: string | null) {
  const my = ++seq;
  if (!props.enabled || !path) {
    stat.value = null;
    error.value = null;
    loading.value = false;
    return;
  }
  loading.value = true;
  error.value = null;
  try {
    const next = await invoke<EntryStat>("navigator_stat", { path });
    if (my !== seq) return;
    stat.value = next;
  } catch (err) {
    if (my !== seq) return;
    stat.value = null;
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    if (my === seq) loading.value = false;
  }
}

watch(
  () => [props.enabled, singlePath.value] as const,
  ([enabled, path]) => {
    if (!enabled) {
      stat.value = null;
      return;
    }
    void load(path);
  },
  { immediate: true },
);
</script>

<template>
  <aside
    v-if="enabled"
    class="details-pane fluent-surface"
    role="complementary"
    :style="{ width: `${detailsPaneWidth}px` }"
    :aria-label="t('detailsPane.title')"
  >
    <div
      class="side-pane-splitter"
      :class="{ 'side-pane-splitter--active': resizing }"
      role="separator"
      aria-orientation="vertical"
      tabindex="0"
      :aria-label="t('detailsPane.resize')"
      :aria-valuenow="detailsPaneWidth"
      :aria-valuemin="SIDE_PANE_MIN_WIDTH"
      :aria-valuemax="SIDE_PANE_MAX_WIDTH"
      @pointerdown="onResizeStart"
      @keydown="onSplitterKey"
    />
    <header class="details-head">
      <h2>{{ t("detailsPane.title") }}</h2>
      <button
        type="button"
        class="fluent-control details-close"
        :aria-label="t('common.dismiss')"
        @click="emit('close')"
      >
        <FluentIcon name="close" size="sm" />
      </button>
    </header>

    <p v-if="!paths.length" class="fluent-banner">{{ t("detailsPane.empty") }}</p>
    <template v-else-if="multiSummary">
      <ul class="details-list">
        <li class="details-row">
          <span class="details-label">{{ t("detailsPane.selected") }}</span>
          <span class="details-value">{{
            t("detailsPane.selectedCount", { count: multiSummary.count })
          }}</span>
        </li>
        <li v-if="multiSummary.known" class="details-row">
          <span class="details-label">{{ t("detailsPane.size") }}</span>
          <span class="details-value">{{ formatBytes(multiSummary.fileBytes) }}</span>
        </li>
      </ul>
      <p class="details-hint">{{ t("detailsPane.multiHint") }}</p>
    </template>
    <template v-else>
      <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>
      <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>
      <ul v-else-if="stat" class="details-list">
        <li v-for="row in rows" :key="row.key" class="details-row">
          <span class="details-label">{{ row.label }}</span>
          <span class="details-value" :class="{ mono: row.mono }" :title="row.value">{{
            row.value
          }}</span>
        </li>
      </ul>
    </template>
  </aside>
</template>

<style scoped>
.details-pane {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 0 0 auto;
  /* AIA-256 / AIA-252: hard floor (= SIDE_PANE_MIN_WIDTH / 15rem). */
  min-width: 15rem;
  min-height: 0;
  border-left: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: 0;
  overflow: auto;
}

.side-pane-splitter {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 0;
  z-index: 1;
  width: 5px;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  cursor: col-resize;
  touch-action: none;
  outline: none;
}

.side-pane-splitter::after {
  content: "";
  position: absolute;
  top: 0;
  bottom: 0;
  left: -2px;
  right: -2px;
}

.side-pane-splitter:hover,
.side-pane-splitter:focus-visible,
.side-pane-splitter--active {
  background: color-mix(in srgb, var(--fluent-color-fill-accent) 18%, transparent);
}

.details-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.details-head h2 {
  margin: 0;
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.details-close {
  padding: var(--fluent-space-xs);
}

.details-list {
  list-style: none;
  margin: 0;
  padding: var(--fluent-space-sm) 0;
}

.details-row {
  display: grid;
  grid-template-columns: 5.5rem 1fr;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-xs) var(--fluent-space-md);
  font-size: var(--fluent-font-size-caption);
}

.details-label {
  color: var(--fluent-color-text-secondary);
}

.details-value {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--fluent-color-text-primary);
}

.details-value.mono {
  font-family: var(--fluent-font-family-mono);
  white-space: normal;
  word-break: break-all;
}

.details-hint {
  margin: 0;
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
</style>
