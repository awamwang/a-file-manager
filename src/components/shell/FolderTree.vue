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
  shellPlaces,
  userFolderItems,
  userFolders,
  type UserFolderId,
} from "../../state/navigatorGuide";
import { isRecyclePath, RECYCLE_PATH } from "../../state/recyclePath";
import {
  isFolderTreeSectionExpanded,
  loadFolderTreeSectionCollapse,
  toggleFolderTreeSection,
  type FolderTreeSectionId,
} from "../../state/folderTreeSections";

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

function sectionExpanded(id: FolderTreeSectionId): boolean {
  return isFolderTreeSectionExpanded(id);
}

function onToggleSection(id: FolderTreeSectionId) {
  toggleFolderTreeSection(id);
}

onMounted(() => {
  loadFolderTreeSectionCollapse();
  void loadNavigatorPlaces();
});

function openGuide() {
  void navigateTo(GUIDE_PATH);
}

function openRecycleBin() {
  void navigateTo(RECYCLE_PATH);
}
</script>

<template>
  <div class="folder-tree" role="tree" :aria-label="t('guide.title')">
    <section class="folder-tree-section" data-section="folders">
      <button
        type="button"
        class="folder-tree-heading folder-tree-heading--toggle"
        :aria-expanded="sectionExpanded('folders')"
        @click="onToggleSection('folders')"
      >
        <FluentIcon
          name="chevronDown"
          size="sm"
          class="folder-tree-section-chevron"
          :class="{ 'folder-tree-section-chevron--closed': !sectionExpanded('folders') }"
        />
        <span>{{ t("guide.sections.folders") }}</span>
      </button>
      <div v-show="sectionExpanded('folders')" class="folder-tree-section-body">
        <FolderTreeBranch
          v-for="item in folderRows"
          :key="item.path"
          :path="item.path"
          :label="folderLabel(item.id)"
          highlight-mode="entry"
        />
        <p v-if="userFolders && !folderRows.length" class="folder-tree-empty">
          {{ t("guide.emptyFolders") }}
        </p>
      </div>
    </section>

    <section class="folder-tree-section" data-section="drives">
      <button
        type="button"
        class="folder-tree-heading folder-tree-heading--toggle"
        :class="{ 'folder-tree-heading--current': isGuidePath(currentPath) }"
        :aria-expanded="sectionExpanded('drives')"
        @click="onToggleSection('drives')"
      >
        <FluentIcon
          name="chevronDown"
          size="sm"
          class="folder-tree-section-chevron"
          :class="{ 'folder-tree-section-chevron--closed': !sectionExpanded('drives') }"
        />
        <span>{{ t("guide.title") }}</span>
      </button>
      <div v-show="sectionExpanded('drives')" class="folder-tree-section-body">
        <button type="button" class="folder-tree-leaf" @click="openGuide">
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
    </section>

    <section class="folder-tree-section" data-section="recycle">
      <button
        type="button"
        class="folder-tree-heading folder-tree-heading--toggle"
        :class="{ 'folder-tree-heading--current': isRecyclePath(currentPath) }"
        :aria-expanded="sectionExpanded('recycle')"
        @click="onToggleSection('recycle')"
      >
        <FluentIcon
          name="chevronDown"
          size="sm"
          class="folder-tree-section-chevron"
          :class="{ 'folder-tree-section-chevron--closed': !sectionExpanded('recycle') }"
        />
        <span>{{ t("recycleBin.tabName") }}</span>
      </button>
      <div v-show="sectionExpanded('recycle')" class="folder-tree-section-body">
        <button
          type="button"
          class="folder-tree-leaf"
          :class="{ 'folder-tree-leaf--current': isRecyclePath(currentPath) }"
          @click="openRecycleBin"
        >
          <FluentIcon name="close" size="sm" />
          <span>{{ t("recycleBin.tabName") }}</span>
        </button>
      </div>
    </section>

    <section v-if="shellPlaces?.onedrive" class="folder-tree-section" data-section="onedrive">
      <button
        type="button"
        class="folder-tree-heading folder-tree-heading--toggle"
        :aria-expanded="sectionExpanded('onedrive')"
        @click="onToggleSection('onedrive')"
      >
        <FluentIcon
          name="chevronDown"
          size="sm"
          class="folder-tree-section-chevron"
          :class="{ 'folder-tree-section-chevron--closed': !sectionExpanded('onedrive') }"
        />
        <span>{{ t("shellPlaces.onedrive") }}</span>
      </button>
      <div v-show="sectionExpanded('onedrive')" class="folder-tree-section-body">
        <FolderTreeBranch
          :path="shellPlaces.onedrive"
          :label="t('shellPlaces.onedrive')"
        />
      </div>
    </section>

    <section
      v-if="shellPlaces?.libraries?.length"
      class="folder-tree-section"
      data-section="libraries"
    >
      <button
        type="button"
        class="folder-tree-heading folder-tree-heading--toggle"
        :aria-expanded="sectionExpanded('libraries')"
        @click="onToggleSection('libraries')"
      >
        <FluentIcon
          name="chevronDown"
          size="sm"
          class="folder-tree-section-chevron"
          :class="{ 'folder-tree-section-chevron--closed': !sectionExpanded('libraries') }"
        />
        <span>{{ t("shellPlaces.libraries") }}</span>
      </button>
      <div v-show="sectionExpanded('libraries')" class="folder-tree-section-body">
        <FolderTreeBranch
          v-for="lib in shellPlaces.libraries"
          :key="lib.path"
          :path="lib.path"
          :label="t(`guide.folders.${lib.id}`)"
        />
      </div>
    </section>

    <section
      v-if="shellPlaces?.network?.length"
      class="folder-tree-section"
      data-section="network"
    >
      <button
        type="button"
        class="folder-tree-heading folder-tree-heading--toggle"
        :aria-expanded="sectionExpanded('network')"
        @click="onToggleSection('network')"
      >
        <FluentIcon
          name="chevronDown"
          size="sm"
          class="folder-tree-section-chevron"
          :class="{ 'folder-tree-section-chevron--closed': !sectionExpanded('network') }"
        />
        <span>{{ t("shellPlaces.network") }}</span>
      </button>
      <div v-show="sectionExpanded('network')" class="folder-tree-section-body">
        <FolderTreeBranch
          v-for="item in shellPlaces.network"
          :key="item.path"
          :path="item.path"
          :label="item.name"
        />
      </div>
    </section>
  </div>
</template>

<style scoped>
.folder-tree {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
  margin-top: var(--fluent-space-sm);
  padding-top: var(--fluent-space-sm);
  border-top: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  min-height: 0;
}

.folder-tree-section {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding-bottom: var(--fluent-space-xs);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.folder-tree-section:last-child {
  border-bottom: 0;
  padding-bottom: 0;
}

.folder-tree-heading {
  margin: 0;
  padding: 0 var(--fluent-space-xs);
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  letter-spacing: 0.02em;
  text-transform: uppercase;
  color: var(--fluent-color-text-secondary);
}

.folder-tree-heading--toggle {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  width: 100%;
  min-height: 1.75rem;
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

.folder-tree-heading--toggle:hover,
.folder-tree-heading--current {
  background: var(--fluent-color-fill-control-hover);
  color: var(--fluent-color-text-primary);
}

.folder-tree-section-chevron {
  flex-shrink: 0;
  transition: transform 120ms ease;
}

.folder-tree-section-chevron--closed {
  transform: rotate(-90deg);
}

.folder-tree-section-body {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding-left: 2px;
  border-left: 2px solid var(--fluent-color-stroke-divider);
  margin-left: 0.55rem;
}

.folder-tree-leaf {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  width: 100%;
  min-height: 1.5rem;
  margin: 0;
  padding: 0 var(--fluent-space-xs);
  border: 0;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: var(--fluent-color-text-primary);
  font: inherit;
  font-size: var(--fluent-font-size-body);
  text-align: left;
  cursor: pointer;
}

.folder-tree-leaf:hover {
  background: var(--fluent-color-fill-control-hover);
}

.folder-tree-leaf--current {
  font-weight: var(--fluent-font-weight-semibold);
  box-shadow: inset 2px 0 0 var(--fluent-color-stroke-accent-soft);
}

.folder-tree-empty {
  margin: 0;
  padding: 0 var(--fluent-space-xs);
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}
</style>
