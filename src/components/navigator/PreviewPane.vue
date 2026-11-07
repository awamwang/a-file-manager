<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import FluentIcon from "../icons/FluentIcon.vue";
import {
  SIDE_PANE_MAX_WIDTH,
  SIDE_PANE_MIN_WIDTH,
  applyPreviewPaneWidth,
  loadPreviewPaneWidth,
  previewPaneWidth,
  setPreviewPaneWidth,
} from "../../state/sidePaneWidth";

export type PreviewPayload = {
  kind: "image" | "text" | "unsupported" | string;
  path: string;
  text: string | null;
  dataUrl: string | null;
  truncated: boolean;
  error: string | null;
};

const props = defineProps<{
  path: string | null;
  enabled: boolean;
}>();

const emit = defineEmits<{ close: [] }>();

const { t } = useI18n();
const loading = ref(false);
const payload = ref<PreviewPayload | null>(null);
let seq = 0;

const resizing = ref(false);
let resizeStartX = 0;
let resizeStartWidth = previewPaneWidth.value;

onMounted(() => {
  loadPreviewPaneWidth();
});

function onResizeStart(event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  resizing.value = true;
  resizeStartX = event.clientX;
  resizeStartWidth = previewPaneWidth.value;
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
  // Left-edge splitter on a right pane: drag left widens.
  const delta = resizeStartX - event.clientX;
  applyPreviewPaneWidth(resizeStartWidth + delta, window.innerWidth);
}

function onResizeEnd() {
  if (!resizing.value) return;
  resizing.value = false;
  window.removeEventListener("pointermove", onResizeMove);
  window.removeEventListener("pointerup", onResizeEnd);
  window.removeEventListener("pointercancel", onResizeEnd);
  setPreviewPaneWidth(previewPaneWidth.value, window.innerWidth);
}

function onSplitterKey(event: KeyboardEvent) {
  const step = event.shiftKey ? 32 : 8;
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    setPreviewPaneWidth(previewPaneWidth.value + step, window.innerWidth);
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    setPreviewPaneWidth(previewPaneWidth.value - step, window.innerWidth);
  } else if (event.key === "Home") {
    event.preventDefault();
    setPreviewPaneWidth(SIDE_PANE_MIN_WIDTH, window.innerWidth);
  } else if (event.key === "End") {
    event.preventDefault();
    setPreviewPaneWidth(SIDE_PANE_MAX_WIDTH, window.innerWidth);
  }
}

async function load(path: string | null) {
  const my = ++seq;
  if (!props.enabled || !path) {
    payload.value = null;
    loading.value = false;
    return;
  }
  loading.value = true;
  try {
    const next = await invoke<PreviewPayload>("preview_load", { path });
    if (my !== seq) return;
    payload.value = next;
  } catch (err) {
    if (my !== seq) return;
    payload.value = {
      kind: "unsupported",
      path,
      text: null,
      dataUrl: null,
      truncated: false,
      error: err instanceof Error ? err.message : String(err),
    };
  } finally {
    if (my === seq) loading.value = false;
  }
}

watch(
  () => [props.enabled, props.path] as const,
  ([enabled, path]) => {
    if (!enabled) {
      payload.value = null;
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
    class="preview fluent-surface"
    :style="{ width: `${previewPaneWidth}px` }"
    :aria-label="t('preview.title')"
  >
    <div
      class="side-pane-splitter"
      :class="{ 'side-pane-splitter--active': resizing }"
      role="separator"
      aria-orientation="vertical"
      tabindex="0"
      :aria-label="t('preview.resize')"
      :aria-valuenow="previewPaneWidth"
      :aria-valuemin="SIDE_PANE_MIN_WIDTH"
      :aria-valuemax="SIDE_PANE_MAX_WIDTH"
      @pointerdown="onResizeStart"
      @keydown="onSplitterKey"
    />
    <header class="preview-head">
      <div class="preview-head-text">
        <h2>{{ t("preview.title") }}</h2>
        <p v-if="path" class="preview-path" :title="path">{{ path }}</p>
      </div>
      <button
        type="button"
        class="fluent-control preview-close"
        :aria-label="t('common.dismiss')"
        @click="emit('close')"
      >
        <FluentIcon name="close" size="sm" />
      </button>
    </header>
    <p v-if="!path" class="fluent-banner">{{ t("preview.empty") }}</p>
    <p v-else-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>
    <template v-else-if="payload">
      <p v-if="payload.error" class="fluent-banner fluent-banner--error">{{ payload.error }}</p>
      <img
        v-else-if="payload.kind === 'image' && payload.dataUrl"
        class="preview-image"
        :src="payload.dataUrl"
        alt=""
      />
      <pre
        v-else-if="payload.kind === 'text' && payload.text != null"
        class="preview-text"
      >{{ payload.text }}</pre>
      <p v-else class="fluent-banner">{{ t("preview.unsupported") }}</p>
      <p v-if="payload.truncated" class="preview-note">{{ t("preview.truncated") }}</p>
    </template>
  </aside>
</template>

<style scoped>
.preview {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 0 0 auto;
  /* AIA-256 / AIA-252: hard floor (= SIDE_PANE_MIN_WIDTH / 15rem). */
  min-width: 15rem;
  min-height: 0;
  border-left: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: 0;
  overflow: hidden;
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

.preview-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.preview-head-text {
  min-width: 0;
  flex: 1;
}

.preview-head h2 {
  margin: 0;
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.preview-close {
  flex-shrink: 0;
  padding: var(--fluent-space-xs);
}

.preview-path {
  margin: var(--fluent-space-xs) 0 0;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-image {
  display: block;
  max-width: 100%;
  max-height: 100%;
  margin: auto;
  object-fit: contain;
  padding: var(--fluent-space-md);
}

.preview-text {
  flex: 1;
  margin: 0;
  padding: var(--fluent-space-md);
  overflow: auto;
  font-family: var(--fluent-font-family-mono);
  font-size: var(--fluent-font-size-caption);
  white-space: pre-wrap;
  word-break: break-word;
}

.preview-note {
  margin: 0;
  padding: var(--fluent-space-xs) var(--fluent-space-md) var(--fluent-space-sm);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
</style>
