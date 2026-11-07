<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import FluentIcon from "../icons/FluentIcon.vue";

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
  <aside v-if="enabled" class="preview fluent-surface" :aria-label="t('preview.title')">
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
  display: flex;
  flex-direction: column;
  width: min(22rem, 38%);
  min-width: 14rem;
  max-width: 28rem;
  min-height: 0;
  border-left: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: 0;
  overflow: hidden;
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
