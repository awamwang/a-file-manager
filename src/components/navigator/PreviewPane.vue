<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import FluentIcon from "../icons/FluentIcon.vue";
import CopyableFieldValue from "../common/CopyableFieldValue.vue";
import {
  SIDE_PANE_MAX_WIDTH,
  SIDE_PANE_MIN_WIDTH,
  applyPreviewPaneWidth,
  loadPreviewPaneWidth,
  previewPaneWidth,
  setPreviewPaneWidth,
} from "../../state/sidePaneWidth";
import { newNavigatorTab, type NavigatorEntry } from "../../state/navigatorBrowse.ts";
import {
  isVirtualEntryPath,
  virtualEntryInfo,
  type VirtualEntryInfo,
} from "../../state/virtualEntryInfo.ts";

export type PreviewLinkInfo = {
  /** "shortcut" (.lnk) or "symlink" (reparse point). */
  kind: "shortcut" | "symlink" | string;
  /** Resolved target path (null when the link carries no readable target). */
  target: string | null;
  /** Whether the target still exists on disk. */
  targetExists: boolean;
  /** "dir" | "file" of the target (null when missing/unknown). */
  targetKind: string | null;
  /** Directory containing the target (原目录 / start-in folder). */
  targetDir: string | null;
  /** Whether the 原目录 still exists (lets the user jump there). */
  targetDirExists: boolean;
};

export type PreviewDirInfo = {
  /** Direct child files (null when the folder is unreadable). */
  fileCount: number | null;
  /** Direct child subfolders (null when the folder is unreadable). */
  dirCount: number | null;
  modifiedMs: number | null;
  createdMs: number | null;
};

export type PreviewPayload = {
  kind: "image" | "text" | "unsupported" | "link" | "dir" | string;
  path: string;
  text: string | null;
  dataUrl: string | null;
  truncated: boolean;
  error: string | null;
  /** Link metadata; non-null only for kind "link" payloads. */
  link: PreviewLinkInfo | null;
  /** Directory basics; non-null only for kind "dir" payloads. */
  dir: PreviewDirInfo | null;
};

const props = defineProps<{
  path: string | null;
  enabled: boolean;
  /** Listing row for the selected path; supplies name/kind/size for virtual items. */
  entry?: NavigatorEntry | null;
}>();

const emit = defineEmits<{ close: [] }>();

const { t, locale } = useI18n();
const loading = ref(false);
const payload = ref<PreviewPayload | null>(null);
const virtualInfo = ref<VirtualEntryInfo | null>(null);
const copiedTarget = ref(false);
let seq = 0;
let copyResetTimer: number | null = null;

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
  copiedTarget.value = false;
  if (copyResetTimer !== null) {
    window.clearTimeout(copyResetTimer);
    copyResetTimer = null;
  }
  virtualInfo.value = null;
  if (!props.enabled || !path) {
    payload.value = null;
    loading.value = false;
    return;
  }
  // SFTP / rclone remotes, bookmarks and other `awam://` items (plus guide /
  // recycle-bin pseudo paths) are not filesystem paths. Show the metadata we
  // already have instead of feeding them to `preview_load` and surfacing a raw
  // "Not a file" / "Path does not exist" error.
  if (isVirtualEntryPath(path)) {
    payload.value = null;
    virtualInfo.value = virtualEntryInfo(path, props.entry, {
      t,
      formatTime: formatDirTime,
    });
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
      link: null,
      dir: null,
    };
  } finally {
    if (my === seq) loading.value = false;
  }
}

const linkKindLabel = computed(() => {
  const link = payload.value?.link;
  if (!link) return "";
  return link.kind === "symlink"
    ? t("preview.linkKindSymlink")
    : t("preview.linkKindShortcut");
});

const targetKindLabel = computed(() => {
  const link = payload.value?.link;
  if (!link?.targetKind) return "";
  return link.targetKind === "dir"
    ? t("preview.linkTargetKindDir")
    : t("preview.linkTargetKindFile");
});

// Folder preview rows share the 属性 labels; timestamps follow the UI locale.
function formatDirTime(ms: number | null): string {
  if (ms == null) return t("properties.unknown");
  void locale.value;
  return new Date(ms).toLocaleString();
}

async function copyLinkTarget() {
  const target = payload.value?.link?.target;
  if (!target) return;
  try {
    await navigator.clipboard.writeText(target);
    copiedTarget.value = true;
    if (copyResetTimer !== null) window.clearTimeout(copyResetTimer);
    copyResetTimer = window.setTimeout(() => {
      copiedTarget.value = false;
      copyResetTimer = null;
    }, 1500);
  } catch {
    /* clipboard may be unavailable; keep the path selectable */
  }
}

// Missing target: keep the card a link card — only the red banner is reserved
// for fully dead links (target AND 原目录 both gone).
const targetMissing = computed(() => {
  const link = payload.value?.link;
  return Boolean(link?.target && !link.targetExists);
});

const showMissingBanner = computed(() => {
  const link = payload.value?.link;
  return Boolean(link?.target && !link.targetExists && !link.targetDirExists);
});

// Existing directory targets / 原目录 open in a new navigator tab.
const canOpenTargetInNewTab = computed(() => {
  const link = payload.value?.link;
  return Boolean(link?.target && link.targetExists && link.targetKind === "dir");
});

async function openTargetInNewTab() {
  const link = payload.value?.link;
  if (link?.target && link.targetExists && link.targetKind === "dir") {
    await newNavigatorTab(link.target);
  }
}

async function openTargetDirInNewTab() {
  const link = payload.value?.link;
  if (link?.targetDir && link.targetDirExists) {
    await newNavigatorTab(link.targetDir);
  }
}

watch(
  () => [props.enabled, props.path, props.entry] as const,
  ([enabled, path]) => {
    if (!enabled) {
      payload.value = null;
      virtualInfo.value = null;
      return;
    }
    void load(path);
  },
  { immediate: true },
);
</script>

<template>
  <div
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
    <div v-else-if="virtualInfo" class="preview-link">
      <div class="preview-link-head">
        <span class="preview-link-icon"><FluentIcon :name="virtualInfo.glyph" size="lg" /></span>
        <div class="preview-link-head-text">
          <p class="preview-link-kind">{{ virtualInfo.kindLabel }}</p>
        </div>
      </div>
      <p v-if="virtualInfo.hint" class="preview-info-hint">{{ virtualInfo.hint }}</p>
      <dl class="preview-link-rows">
        <div v-for="row in virtualInfo.rows" :key="row.key" class="preview-link-row">
          <dt>{{ row.label }}</dt>
          <dd :class="{ 'preview-link-path-text': row.mono }">
            <CopyableFieldValue
              v-if="row.copyable"
              :value="row.value"
              :label="t('virtualInfo.copyMagnet')"
            />
            <template v-else>{{ row.value }}</template>
          </dd>
        </div>
      </dl>
    </div>
    <template v-else-if="payload">
      <p v-if="payload.error" class="fluent-banner fluent-banner--error">{{ payload.error }}</p>
      <div
        v-else-if="payload.kind === 'link' && payload.link"
        class="preview-link"
      >
        <div class="preview-link-head">
          <span class="preview-link-icon"><FluentIcon name="link" size="lg" /></span>
          <div class="preview-link-head-text">
            <p class="preview-link-kind">{{ linkKindLabel }}</p>
          </div>
        </div>
        <p
          v-if="showMissingBanner"
          class="fluent-banner fluent-banner--error preview-link-missing"
        >
          {{ t("preview.linkTargetMissing") }}
        </p>
        <dl class="preview-link-rows">
          <div class="preview-link-row">
            <dt>{{ t("preview.linkTargetLabel") }}</dt>
            <dd class="preview-link-path">
              <button
                v-if="canOpenTargetInNewTab"
                type="button"
                class="preview-link-open"
                :title="t('contextMenu.openInNewTab')"
                @click="openTargetInNewTab"
                >{{ payload.link.target }}</button
              >
              <span
                v-else-if="payload.link.target"
                class="preview-link-path-text"
                :title="payload.link.target"
                >{{ payload.link.target }}</span
              >
              <span v-else class="preview-link-muted">{{
                t("preview.linkTargetUnresolved")
              }}</span>
              <span
                v-if="targetMissing"
                class="preview-link-badge"
                >{{ t("preview.linkTargetMissingBadge") }}</span
              >
              <button
                v-if="payload.link.target"
                type="button"
                class="fluent-control preview-link-copy"
                :aria-label="t('preview.linkCopyTarget')"
                :title="copiedTarget ? t('preview.linkCopiedTarget') : t('preview.linkCopyTarget')"
                @click="copyLinkTarget"
              >
                <FluentIcon :name="copiedTarget ? 'check' : 'copy'" size="sm" />
              </button>
            </dd>
          </div>
          <div v-if="targetKindLabel" class="preview-link-row">
            <dt>{{ t("preview.linkTargetKindLabel") }}</dt>
            <dd>{{ targetKindLabel }}</dd>
          </div>
          <div v-if="payload.link.targetDir" class="preview-link-row">
            <dt>{{ t("preview.linkTargetDirLabel") }}</dt>
            <dd class="preview-link-path">
              <button
                v-if="payload.link.targetDirExists"
                type="button"
                class="preview-link-open"
                :title="t('contextMenu.openInNewTab')"
                @click="openTargetDirInNewTab"
                >{{ payload.link.targetDir }}</button
              >
              <span v-else class="preview-link-path-text" :title="payload.link.targetDir">{{
                payload.link.targetDir
              }}</span>
            </dd>
          </div>
        </dl>
      </div>
      <div
        v-else-if="payload.kind === 'dir' && payload.dir"
        class="preview-link"
      >
        <div class="preview-link-head">
          <span class="preview-link-icon"><FluentIcon name="folder" size="lg" /></span>
          <div class="preview-link-head-text">
            <p class="preview-link-kind">{{ t("properties.kindFolder") }}</p>
          </div>
        </div>
        <dl class="preview-link-rows">
          <div v-if="payload.dir.fileCount != null" class="preview-link-row">
            <dt>{{ t("properties.fileCount") }}</dt>
            <dd>{{ payload.dir.fileCount }}</dd>
          </div>
          <div v-if="payload.dir.dirCount != null" class="preview-link-row">
            <dt>{{ t("properties.dirCount") }}</dt>
            <dd>{{ payload.dir.dirCount }}</dd>
          </div>
          <div v-if="payload.dir.fileCount == null" class="preview-link-row">
            <dt>{{ t("preview.dirContentsLabel") }}</dt>
            <dd><span class="preview-link-muted">{{ t("preview.dirUnreadable") }}</span></dd>
          </div>
          <div class="preview-link-row">
            <dt>{{ t("properties.modified") }}</dt>
            <dd>{{ formatDirTime(payload.dir.modifiedMs) }}</dd>
          </div>
          <div class="preview-link-row">
            <dt>{{ t("properties.created") }}</dt>
            <dd>{{ formatDirTime(payload.dir.createdMs) }}</dd>
          </div>
        </dl>
      </div>
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
  </div>
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

.preview-info-hint {
  margin: 0 0 var(--fluent-space-md);
  padding: var(--fluent-space-sm);
  border-radius: var(--fluent-radius, 6px);
  background: color-mix(in srgb, var(--fluent-color-fill-accent) 10%, transparent);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.preview-link {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: var(--fluent-space-md);
}

.preview-link-head {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  margin-bottom: var(--fluent-space-md);
}

.preview-link-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--fluent-radius, 6px);
  background: color-mix(in srgb, var(--fluent-color-fill-accent) 14%, transparent);
  color: var(--fluent-color-fill-accent);
}

.preview-link-kind {
  margin: 0;
  font-size: var(--fluent-font-size-body);
  font-weight: var(--fluent-font-weight-semibold);
}

.preview-link-missing {
  margin: 0 0 var(--fluent-space-sm);
}

.preview-link-rows {
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.preview-link-row {
  display: grid;
  grid-template-columns: minmax(72px, auto) 1fr;
  column-gap: var(--fluent-space-sm);
  align-items: start;
}

.preview-link-row dt {
  margin: 0;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  padding-top: 2px;
}

.preview-link-row dd {
  margin: 0;
  min-width: 0;
  font-size: var(--fluent-font-size-caption);
  word-break: break-word;
}

.preview-link-path {
  display: flex;
  align-items: start;
  gap: var(--fluent-space-xs);
}

.preview-link-path-text {
  min-width: 0;
  flex: 1;
  overflow-wrap: anywhere;
  font-family: var(--fluent-font-family-mono);
}

/* Existing dir targets / 原目录: inline link that opens a new navigator tab. */
.preview-link-open {
  min-width: 0;
  flex: 1;
  padding: 0;
  border: 0;
  background: none;
  text-align: left;
  font-family: var(--fluent-font-family-mono);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-fill-accent);
  cursor: pointer;
  overflow-wrap: anywhere;
}

.preview-link-open:hover,
.preview-link-open:focus-visible {
  text-decoration: underline;
}

/* Muted "missing" badge next to a dead target path (link card stays a card). */
.preview-link-badge {
  flex-shrink: 0;
  padding: 0 6px;
  border-radius: 999px;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  background: color-mix(in srgb, var(--fluent-color-text-secondary) 14%, transparent);
}

.preview-link-muted {
  color: var(--fluent-color-text-secondary);
}

.preview-link-copy {
  flex-shrink: 0;
  padding: var(--fluent-space-xs);
}
</style>
