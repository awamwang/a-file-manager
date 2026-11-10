<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import FluentIcon from "../icons/FluentIcon.vue";
import FolderTreeBranch from "./FolderTreeBranch.vue";
import { currentPath, navigateTo } from "../../state/navigatorBrowse";
import {
  GUIDE_PATH,
  drivesList,
  isGuidePath,
  loadDrives,
} from "../../state/navigatorGuide";
import { isOsHierarchyPath } from "../../state/folderTree";
import { scheduleAfterPaint } from "../../state/scheduleIdle";

const { t } = useI18n();

/** This PC root: expanded to reveal drive roots (Explorer-like). */
const thisPcExpanded = ref(true);
const thisPcUserCollapsed = ref(false);

const guideSelected = computed(() => isGuidePath(currentPath.value));

function driveLabel(letter: string, label: string): string {
  if (label && label !== letter) {
    return t("guide.driveNamed", { letter, label });
  }
  return t("guide.drive", { letter });
}

function openGuide() {
  void navigateTo(GUIDE_PATH);
}

function toggleThisPc() {
  if (thisPcExpanded.value) {
    thisPcExpanded.value = false;
    thisPcUserCollapsed.value = true;
    return;
  }
  thisPcUserCollapsed.value = false;
  thisPcExpanded.value = true;
}

watch(
  () => currentPath.value,
  (path, prev) => {
    if (path !== prev) thisPcUserCollapsed.value = false;
    if (isOsHierarchyPath(path) && !thisPcUserCollapsed.value) {
      thisPcExpanded.value = true;
    }
  },
  { immediate: true },
);

onMounted(() => {
  // Defer drive roots past first navigator_list so tree IPC does not contend.
  scheduleAfterPaint(() => {
    void loadDrives().catch(() => {
      drivesList.value = [];
    });
  });
});
</script>

<template>
  <div class="folder-tree" role="tree" :aria-label="t('guide.rootLabel')">
    <div class="tree-branch tree-branch--root" data-tree-root="this-pc">
      <div class="tree-row">
        <button
          type="button"
          class="tree-twist"
          :aria-expanded="thisPcExpanded"
          :aria-label="thisPcExpanded ? t('chrome.collapseFolder') : t('chrome.expandFolder')"
          @click="toggleThisPc"
        >
          <FluentIcon
            name="chevronDown"
            size="sm"
            :class="{ 'tree-twist-icon--closed': !thisPcExpanded }"
          />
        </button>
        <button
          type="button"
          class="tree-label"
          :class="{ 'tree-label--selected': guideSelected }"
          :title="t('guide.rootTitle')"
          :aria-current="guideSelected ? 'page' : undefined"
          @click="openGuide"
        >
          <FluentIcon name="folder" size="sm" class="tree-label-icon" />
          <span>{{ t("guide.rootLabel") }}</span>
        </button>
      </div>
      <div v-if="thisPcExpanded" class="tree-children">
        <FolderTreeBranch
          v-for="drive in drivesList"
          :key="drive.path"
          :path="drive.path"
          :label="driveLabel(drive.letter, drive.label)"
          :depth="0"
        />
        <p v-if="!drivesList.length" class="folder-tree-empty">{{ t("guide.emptyDrives") }}</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.folder-tree {
  display: flex;
  flex-direction: column;
  gap: 1px;
  margin-top: var(--fluent-space-sm);
  padding-top: var(--fluent-space-sm);
  border-top: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  min-height: 0;
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
  font-size: var(--fluent-font-size-body);
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

.tree-label--selected {
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-primary);
}

.tree-children {
  border-left: 1px solid var(--fluent-color-stroke-divider);
  margin-left: 12px;
  padding-left: 2px;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.folder-tree-empty {
  margin: 0;
  padding: 0 var(--fluent-space-xs);
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}
</style>
