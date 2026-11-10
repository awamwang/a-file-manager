<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import FluentIcon from "../icons/FluentIcon.vue";
import { currentPath, navigateTo, type NavigatorListing } from "../../state/navigatorBrowse";
import {
  directoryChildren,
  treeRowHighlight,
  treeShouldAutoExpand,
  type FolderTreeChild,
  type TreeHighlightMode,
} from "../../state/folderTree";
import { showHiddenItems } from "../../state/showHidden";
import {
  acceptFolderDragOver,
  commitFolderDrop,
  createDropHoverScheduler,
  noticeKeyForDropMode,
} from "../../state/dnd";
import { toastError, toastSuccess } from "../../state/toast";
import { scheduleAfterPaint } from "../../state/scheduleIdle";
import { perfSpan } from "../../state/perfProbe";

const props = withDefaults(
  defineProps<{
    path: string;
    label: string;
    depth?: number;
    /** Legacy AIA-230 entry roots; Explorer tree uses `tree`. */
    highlightMode?: TreeHighlightMode;
  }>(),
  { depth: 0, highlightMode: "tree" },
);

const { t } = useI18n();
const expanded = ref(false);
const loading = ref(false);
const children = ref<FolderTreeChild[] | null>(null);
/** Manual collapse while still under this path — cleared on navigation. */
const userCollapsed = ref(false);
/** AIA-242: file-drag highlight on this folder row. */
const dropOver = ref(false);
const hoverExpand = createDropHoverScheduler();

const highlight = computed(() =>
  treeRowHighlight(currentPath.value, props.path, props.highlightMode),
);

async function loadChildren() {
  loading.value = true;
  try {
    let entryCount = 0;
    const listing = await perfSpan(
      "tree_list",
      async () => {
        const result = await invoke<NavigatorListing>("navigator_list", {
          path: props.path,
          showHidden: showHiddenItems.value,
        });
        entryCount = result.entries.length;
        return result;
      },
      () => ({ path: props.path, n: entryCount }),
    );
    children.value = directoryChildren(listing.entries);
  } catch {
    children.value = [];
  } finally {
    loading.value = false;
  }
}

async function expandAndLoad() {
  expanded.value = true;
  userCollapsed.value = false;
  if (children.value === null) {
    await loadChildren();
  }
}

async function toggle() {
  if (expanded.value) {
    expanded.value = false;
    userCollapsed.value = true;
    return;
  }
  userCollapsed.value = false;
  await expandAndLoad();
}

function open() {
  void navigateTo(props.path);
}

function onTreeDragOver(event: DragEvent) {
  const mode = acceptFolderDragOver(event, props.path, { rejectIfSelected: true });
  if (mode == null) {
    dropOver.value = false;
    hoverExpand.clear();
    return;
  }
  dropOver.value = true;
  if (!expanded.value) {
    hoverExpand.schedule(props.path, () => {
      void expandAndLoad();
    });
  } else {
    hoverExpand.clear();
  }
}

function onTreeDragLeave(event: DragEvent) {
  const related = event.relatedTarget as Node | null;
  const root = event.currentTarget as HTMLElement | null;
  if (root && related && root.contains(related)) return;
  dropOver.value = false;
  hoverExpand.clear();
}

async function onTreeDrop(event: DragEvent) {
  dropOver.value = false;
  hoverExpand.clear();
  const result = await commitFolderDrop(event, props.path);
  if (!result.ok) {
    if (result.error && result.error !== "empty") {
      toastError(result.error === "failed" ? t("contextMenu.opFailed") : result.error);
    }
    return;
  }
  if (result.mode && result.count != null) {
    toastSuccess(t(noticeKeyForDropMode(result.mode), { count: result.count, name: props.label }));
  }
}

/** Cancel stale deferred expands when the path changes again quickly (AIA-259). */
let expandPaintToken = 0;

watch(
  () => currentPath.value,
  (path, prev) => {
    if (path !== prev) userCollapsed.value = false;
    if (!(treeShouldAutoExpand(path, props.path) && !userCollapsed.value)) {
      return;
    }
    // Let the content-pane navigator_list win the first paint slot.
    const token = ++expandPaintToken;
    scheduleAfterPaint(() => {
      if (token !== expandPaintToken) return;
      if (!treeShouldAutoExpand(currentPath.value, props.path) || userCollapsed.value) {
        return;
      }
      void expandAndLoad();
    });
  },
  { immediate: true },
);

onUnmounted(() => {
  hoverExpand.clear();
});
</script>

<template>
  <div
    class="tree-branch"
    :class="{
      'tree-branch--entry': highlightMode === 'entry',
      'tree-branch--nested': depth > 0,
      'tree-branch--root': depth === 0,
    }"
    :data-depth="depth"
    data-drop-target="folder-tree"
    :data-entry-path="path"
  >
    <div
      class="tree-row"
      :class="{ 'tree-row--drop': dropOver }"
      :style="{ paddingLeft: `${depth * 12}px` }"
      @dragover="onTreeDragOver"
      @dragleave="onTreeDragLeave"
      @drop="onTreeDrop"
    >
      <button
        type="button"
        class="tree-twist"
        :aria-expanded="expanded"
        :aria-label="expanded ? t('chrome.collapseFolder') : t('chrome.expandFolder')"
        @click="toggle"
      >
        <FluentIcon name="chevronDown" size="sm" :class="{ 'tree-twist-icon--closed': !expanded }" />
      </button>
      <button
        type="button"
        class="tree-label"
        :class="{
          'tree-label--selected': highlight === 'selected',
          'tree-label--current': highlight === 'current',
          'tree-label--nested': depth > 0,
          'tree-label--drop': dropOver,
        }"
        :title="path"
        :aria-current="highlight === 'selected' || highlight === 'current' ? 'page' : undefined"
        @click="open"
      >
        <FluentIcon name="folder" size="sm" class="tree-label-icon" />
        <span>{{ label }}</span>
      </button>
    </div>
    <div v-if="expanded" class="tree-children" :style="{ marginLeft: `${12 + depth * 12}px` }">
      <p v-if="loading" class="tree-note">{{ t("common.loading") }}</p>
      <p v-else-if="children && !children.length" class="tree-note">{{ t("common.empty") }}</p>
      <FolderTreeBranch
        v-for="child in children ?? []"
        :key="child.path"
        :path="child.path"
        :label="child.name"
        :depth="depth + 1"
        highlight-mode="tree"
      />
    </div>
  </div>
</template>

<style scoped>
.tree-branch--root > .tree-row .tree-label {
  font-size: var(--fluent-font-size-body);
}

.tree-branch--nested > .tree-row .tree-label {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.tree-row {
  display: flex;
  align-items: center;
  min-width: 0;
  gap: 2px;
}

.tree-twist,
.tree-label {
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  cursor: pointer;
}

.tree-twist {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.25rem;
  height: 1.5rem;
  flex-shrink: 0;
  border-radius: var(--fluent-radius-sm);
  padding: 0;
}

.tree-twist:hover,
.tree-label:hover {
  background: var(--fluent-color-fill-control-hover);
}

.tree-twist-icon--closed {
  transform: rotate(-90deg);
}

.tree-label {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  min-width: 0;
  flex: 1;
  height: 1.5rem;
  padding: 0 var(--fluent-space-xs);
  border-radius: var(--fluent-radius-sm);
  text-align: left;
}

.tree-label span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tree-label-icon {
  flex-shrink: 0;
  opacity: 0.9;
}

.tree-label--nested .tree-label-icon {
  opacity: 0.7;
}

.tree-children {
  border-left: 1px solid var(--fluent-color-stroke-divider);
  padding-left: 2px;
}

/* Explorer current folder — selected semantics. */
.tree-label--selected {
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-primary);
}

/* Legacy AIA-230 entry soft cue (unused by AIA-236 tree roots). */
.tree-label--current {
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-primary);
  box-shadow: inset 2px 0 0 var(--fluent-color-stroke-accent-soft);
}

.tree-label--current:hover {
  background: var(--fluent-color-fill-control-hover);
}

.tree-row--drop .tree-label,
.tree-label--drop {
  background: var(--fluent-color-fill-selected);
  outline: 1px solid var(--fluent-color-stroke-accent-soft);
}

.tree-note {
  margin: 0;
  padding: 0 0 0 0.5rem;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}
</style>
