<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import FluentIcon from "../icons/FluentIcon.vue";
import FolderTreeBranch from "./FolderTreeBranch.vue";
import { currentPath, navigateTo } from "../../state/navigatorBrowse";
import {
  GUIDE_PATH,
  drivesList,
  isGuidePath,
  loadNavigatorPlaces,
  userFolderItems,
  userFolders,
  type UserFolderId,
} from "../../state/navigatorGuide";

const { t } = useI18n();

const folderRows = computed(() => userFolderItems(userFolders.value));

function folderLabel(id: UserFolderId): string {
  return t(`guide.folders.${id}`);
}

function driveLabel(letter: string, label: string): string {
  if (label && label !== letter) {
    return t("guide.driveNamed", { letter, label });
  }
  return t("guide.drive", { letter });
}

onMounted(() => {
  void loadNavigatorPlaces();
});

function openGuide() {
  void navigateTo(GUIDE_PATH);
}
</script>

<template>
  <div class="folder-tree" role="tree" :aria-label="t('guide.title')">
    <p class="folder-tree-heading">{{ t("guide.sections.folders") }}</p>
    <FolderTreeBranch
      v-for="item in folderRows"
      :key="item.path"
      :path="item.path"
      :label="folderLabel(item.id)"
    />
    <p v-if="userFolders && !folderRows.length" class="folder-tree-empty">
      {{ t("guide.emptyFolders") }}
    </p>

    <button
      type="button"
      class="folder-tree-heading folder-tree-heading--button"
      :class="{ 'folder-tree-heading--active': isGuidePath(currentPath) }"
      @click="openGuide"
    >
      <FluentIcon name="folder" size="sm" />
      <span>{{ t("guide.title") }}</span>
    </button>
    <FolderTreeBranch
      v-for="drive in drivesList"
      :key="drive.path"
      :path="drive.path"
      :label="driveLabel(drive.letter, drive.label)"
    />
    <p v-if="!drivesList.length" class="folder-tree-empty">{{ t("guide.emptyDrives") }}</p>
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

.folder-tree-heading {
  margin: var(--fluent-space-xs) 0 0;
  padding: 0 var(--fluent-space-xs);
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  letter-spacing: 0.02em;
  text-transform: uppercase;
  color: var(--fluent-color-text-secondary);
}

.folder-tree-heading--button {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  width: 100%;
  margin-top: var(--fluent-space-sm);
  border: 0;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: var(--fluent-color-text-secondary);
  font: inherit;
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  letter-spacing: 0.02em;
  text-transform: uppercase;
  text-align: left;
  cursor: pointer;
}

.folder-tree-heading--button:hover,
.folder-tree-heading--active {
  background: var(--fluent-color-fill-control-hover);
  color: var(--fluent-color-text-primary);
}

.folder-tree-empty {
  margin: 0;
  padding: 0 var(--fluent-space-xs);
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}
</style>
