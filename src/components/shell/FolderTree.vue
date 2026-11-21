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
  loadNetworkShares,
  networkShares,
  networkSharesLoaded,
  networkSharesLoading,
} from "../../state/guide";
import { isOsHierarchyPath } from "../../state/folderTree";
import { scheduleAfterPaint, scheduleIdle } from "../../state/scheduleIdle";

const { t } = useI18n();

/** This PC root: expanded to reveal drive roots (Explorer-like). */
const thisPcExpanded = ref(true);
const thisPcUserCollapsed = ref(false);

/** ADR 0026: Network root — sibling of This PC, like Windows Explorer. */
const networkExpanded = ref(false);

const guideSelected = computed(() => isGuidePath(currentPath.value));
const networkChildren = computed(() => networkShares.value);
// Shared with the startup pre-warm so the node reflects a background load.
const networkLoading = computed(() => networkSharesLoading.value);
const networkLoaded = computed(() => networkSharesLoaded.value);

function isUncPath(path: string | null | undefined): boolean {
  return !!path && path.trim().startsWith("\\\\");
}

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

/** Single-flight: reuses the in-flight pre-warm or triggers one on demand. */
function ensureNetworkLoaded() {
  void loadNetworkShares().catch(() => undefined);
}

function toggleNetwork() {
  if (networkExpanded.value) {
    networkExpanded.value = false;
    return;
  }
  networkExpanded.value = true;
  ensureNetworkLoaded();
}

watch(
  () => currentPath.value,
  (path, prev) => {
    if (path !== prev) thisPcUserCollapsed.value = false;
    if (isOsHierarchyPath(path) && !thisPcUserCollapsed.value) {
      thisPcExpanded.value = true;
    }
    // Reveal the Network root when navigation lands on a UNC share (ADR 0026).
    if (isUncPath(path)) {
      networkExpanded.value = true;
      ensureNetworkLoaded();
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
  // ADR 0026: pre-warm WS-Discovery in the background once the app is idle, so
  // the slow (cold ~30s) Network enumeration is already done by the time the
  // user expands 网络. Single-flight, so expand / Guide reuse it, never re-run.
  scheduleIdle(() => {
    ensureNetworkLoaded();
  });
});
</script>

<template>
  <div class="folder-tree" role="tree" :aria-label="t('guide.rootLabel')">
    <div class="tree-branch tree-branch--root" role="presentation" data-tree-root="this-pc">
      <div
        class="tree-row"
        role="treeitem"
        :aria-expanded="thisPcExpanded"
        aria-level="1"
        :aria-selected="guideSelected"
        tabindex="0"
        :aria-current="guideSelected ? 'page' : undefined"
        @click="openGuide"
        @keydown.enter.prevent="openGuide"
        @keydown.space.prevent="openGuide"
      >
        <span class="tree-twist" aria-hidden="true" @click.stop="toggleThisPc">
          <FluentIcon
            name="chevronDown"
            size="sm"
            :class="{ 'tree-twist-icon--closed': !thisPcExpanded }"
          />
        </span>
        <span class="tree-label" :class="{ 'tree-label--selected': guideSelected }" :title="t('guide.rootTitle')">
          <FluentIcon name="folder" size="sm" class="tree-label-icon" />
          <span>{{ t("guide.rootLabel") }}</span>
        </span>
      </div>
      <div v-if="thisPcExpanded" class="tree-children" role="group">
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

    <div class="tree-branch tree-branch--root" role="presentation" data-tree-root="network">
      <div
        class="tree-row"
        role="treeitem"
        :aria-expanded="networkExpanded"
        aria-level="1"
        tabindex="0"
        :title="t('shellPlaces.network')"
        @click="toggleNetwork"
        @keydown.enter.prevent="toggleNetwork"
        @keydown.space.prevent="toggleNetwork"
      >
        <span class="tree-twist" aria-hidden="true" @click.stop="toggleNetwork">
          <FluentIcon
            name="chevronDown"
            size="sm"
            :class="{ 'tree-twist-icon--closed': !networkExpanded }"
          />
        </span>
        <span class="tree-label" :title="t('shellPlaces.network')">
          <FluentIcon name="globe" size="sm" class="tree-label-icon" />
          <span>{{ t("shellPlaces.network") }}</span>
        </span>
      </div>
      <div v-if="networkExpanded" class="tree-children" role="group">
        <FolderTreeBranch
          v-for="share in networkChildren"
          :key="share.path"
          :path="share.path"
          :label="share.name"
          :depth="0"
        />
        <p v-if="networkLoading && !networkChildren.length" class="folder-tree-empty">
          {{ t("common.loading") }}
        </p>
        <p v-else-if="networkLoaded && !networkChildren.length" class="folder-tree-empty">
          {{ t("shellPlaces.networkEmpty") }}
        </p>
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
