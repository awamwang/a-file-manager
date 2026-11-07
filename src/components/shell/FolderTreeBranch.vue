<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import FluentIcon from "../icons/FluentIcon.vue";
import { currentPath, navigateTo, type NavigatorListing } from "../../state/navigatorBrowse";
import {
  directoryChildren,
  treeRowHighlight,
  type FolderTreeChild,
  type TreeHighlightMode,
} from "../../state/folderTree";
import { showHiddenItems } from "../../state/showHidden";

const props = withDefaults(
  defineProps<{
    path: string;
    label: string;
    depth?: number;
    /** AIA-230: entry roots have no selected semantics. */
    highlightMode?: TreeHighlightMode;
  }>(),
  { depth: 0, highlightMode: "tree" },
);

const { t } = useI18n();
const expanded = ref(false);
const loading = ref(false);
const children = ref<FolderTreeChild[] | null>(null);

const highlight = computed(() =>
  treeRowHighlight(currentPath.value, props.path, props.highlightMode),
);

async function loadChildren() {
  loading.value = true;
  try {
    const listing = await invoke<NavigatorListing>("navigator_list", {
      path: props.path,
      showHidden: showHiddenItems.value,
    });
    children.value = directoryChildren(listing.entries);
  } catch {
    children.value = [];
  } finally {
    loading.value = false;
  }
}

async function toggle() {
  expanded.value = !expanded.value;
  if (expanded.value && children.value === null) {
    await loadChildren();
  }
}

function open() {
  void navigateTo(props.path);
}
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
  >
    <div class="tree-row" :style="{ paddingLeft: `${depth * 12}px` }">
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
        }"
        :title="path"
        :aria-current="highlight === 'current' ? 'page' : undefined"
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

/* Hierarchy selection (drives / expanded dirs) — selected semantics. */
.tree-label--selected {
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-primary);
}

/*
 * AIA-230 entry points (常用文件夹 / favorites roots):
 * ephemeral current-path cue only — never fill-selected, clears when path leaves.
 */
.tree-label--current {
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-primary);
  box-shadow: inset 2px 0 0 var(--fluent-color-stroke-accent-soft);
}

.tree-label--current:hover {
  background: var(--fluent-color-fill-control-hover);
}

.tree-note {
  margin: 0;
  padding: 0 0 0 0.5rem;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}
</style>
