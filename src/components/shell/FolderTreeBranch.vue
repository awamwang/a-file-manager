<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import FluentIcon from "../icons/FluentIcon.vue";
import { currentPath, navigateTo, type NavigatorListing } from "../../state/navigatorBrowse";
import { directoryChildren, treePathActive, type FolderTreeChild } from "../../state/folderTree";
import { showHiddenItems } from "../../state/showHidden";

const props = withDefaults(
  defineProps<{
    path: string;
    label: string;
    depth?: number;
  }>(),
  { depth: 0 },
);

const { t } = useI18n();
const expanded = ref(false);
const loading = ref(false);
const children = ref<FolderTreeChild[] | null>(null);

const active = computed(() => treePathActive(currentPath.value, props.path));

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
  <div class="tree-branch">
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
        :class="{ 'tree-label--active': active }"
        :title="path"
        @click="open"
      >
        <FluentIcon name="folder" size="sm" />
        <span>{{ label }}</span>
      </button>
    </div>
    <div v-if="expanded">
      <p v-if="loading" class="tree-note">{{ t("common.loading") }}</p>
      <p v-else-if="children && !children.length" class="tree-note">{{ t("common.empty") }}</p>
      <FolderTreeBranch
        v-for="child in children ?? []"
        :key="child.path"
        :path="child.path"
        :label="child.name"
        :depth="depth + 1"
      />
    </div>
  </div>
</template>

<style scoped>
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

.tree-label--active {
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
}

.tree-note {
  margin: 0;
  padding: 0 0 0 1.75rem;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}
</style>
