<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import FluentIcon from "../icons/FluentIcon.vue";
import { favoritesDoc, loadFavorites } from "../../state/favorites";
import { quickAccessList, loadVisitStats } from "../../state/visitStats";
import {
  drivesList,
  loadNavigatorPlaces,
  shellPlaces,
  userFolderItems,
  userFolders,
  type UserFolderId,
} from "../../state/guide";
import { RECYCLE_PATH } from "../../state/recycle";

const props = defineProps<{
  /** Navigate the current tab to this OS path. */
  openPath: (path: string) => void | Promise<void>;
}>();

const { t } = useI18n();

const folderRows = computed(() => userFolderItems(userFolders.value));

function folderLabel(id: UserFolderId): string {
  return t(`guide.folders.${id}`);
}

function favoriteLabel(path: string): string {
  const parts = path.replace(/\\/g, "/").replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || path;
}

function driveLabel(letter: string, label: string): string {
  if (label && label !== letter) {
    return t("guide.driveNamed", { letter, label });
  }
  return t("guide.drive", { letter });
}

onMounted(() => {
  void loadNavigatorPlaces();
  void loadFavorites().catch(() => undefined);
  void loadVisitStats().catch(() => undefined);
});

function onOpen(path: string) {
  void props.openPath(path);
}
</script>

<template>
  <div class="navigator-guide" role="region" :aria-label="t('guide.title')">
    <header class="guide-header">
      <h1 class="guide-title">{{ t("guide.title") }}</h1>
      <p class="guide-subtitle">{{ t("guide.subtitle") }}</p>
    </header>

    <section class="guide-section" :aria-label="t('guide.sections.folders')">
      <h2 class="guide-section-title">{{ t("guide.sections.folders") }}</h2>
      <div class="guide-grid">
        <button
          v-for="item in folderRows"
          :key="item.id"
          type="button"
          class="guide-card fluent-surface"
          :title="item.path"
          @click="onOpen(item.path)"
        >
          <FluentIcon :name="item.id === 'home' ? 'home' : 'folder'" size="md" />
          <span class="guide-card-label">{{ folderLabel(item.id) }}</span>
          <span class="guide-card-path">{{ item.path }}</span>
        </button>
      </div>
      <p v-if="!folderRows.length" class="guide-empty">{{ t("guide.emptyFolders") }}</p>
    </section>

    <section class="guide-section" :aria-label="t('guide.sections.recent')">
      <h2 class="guide-section-title">{{ t("guide.sections.recent") }}</h2>
      <div class="guide-grid">
        <button
          v-for="item in quickAccessList"
          :key="item.path"
          type="button"
          class="guide-card fluent-surface"
          :title="item.path"
          @click="onOpen(item.path)"
        >
          <FluentIcon name="folderOpen" size="md" />
          <span class="guide-card-label">{{ favoriteLabel(item.path) }}</span>
          <span class="guide-card-path">{{ item.path }}</span>
        </button>
      </div>
      <p v-if="!quickAccessList.length" class="guide-empty">{{ t("quickAccess.empty") }}</p>
    </section>

    <section class="guide-section" :aria-label="t('guide.sections.favorites')">
      <h2 class="guide-section-title">{{ t("guide.sections.favorites") }}</h2>
      <div class="guide-grid">
        <button
          v-for="fav in favoritesDoc.favorites"
          :key="fav.path"
          type="button"
          class="guide-card fluent-surface"
          :title="fav.path"
          @click="onOpen(fav.path)"
        >
          <FluentIcon name="star" size="md" />
          <span class="guide-card-label">{{ favoriteLabel(fav.path) }}</span>
          <span class="guide-card-path">{{ fav.path }}</span>
        </button>
      </div>
      <p v-if="!favoritesDoc.favorites.length" class="guide-empty">
        {{ t("favorites.empty") }}
      </p>
    </section>

    <section class="guide-section" :aria-label="t('guide.sections.drives')">
      <h2 class="guide-section-title">{{ t("guide.sections.drives") }}</h2>
      <div class="guide-grid">
        <button
          v-for="drive in drivesList"
          :key="drive.path"
          type="button"
          class="guide-card fluent-surface"
          :title="drive.path"
          @click="onOpen(drive.path)"
        >
          <FluentIcon name="folder" size="md" />
          <span class="guide-card-label">{{ driveLabel(drive.letter, drive.label) }}</span>
          <span class="guide-card-path">{{ drive.path }}</span>
        </button>
      </div>
      <p v-if="!drivesList.length" class="guide-empty">{{ t("guide.emptyDrives") }}</p>
    </section>

    <section class="guide-section" :aria-label="t('recycleBin.section')">
      <h2 class="guide-section-title">{{ t("recycleBin.section") }}</h2>
      <div class="guide-grid">
        <button
          type="button"
          class="guide-card fluent-surface"
          :title="t('recycleBin.subtitle')"
          @click="onOpen(RECYCLE_PATH)"
        >
          <FluentIcon name="close" size="md" />
          <span class="guide-card-label">{{ t("recycleBin.tabName") }}</span>
          <span class="guide-card-path">{{ t("recycleBin.subtitle") }}</span>
        </button>
      </div>
    </section>

    <section
      v-if="shellPlaces?.onedrive"
      class="guide-section"
      :aria-label="t('shellPlaces.onedrive')"
    >
      <h2 class="guide-section-title">{{ t("shellPlaces.onedrive") }}</h2>
      <div class="guide-grid">
        <button
          type="button"
          class="guide-card fluent-surface"
          :title="shellPlaces.onedrive"
          @click="onOpen(shellPlaces.onedrive)"
        >
          <FluentIcon name="cloud" size="md" />
          <span class="guide-card-label">{{ t("shellPlaces.onedrive") }}</span>
          <span class="guide-card-path">{{ shellPlaces.onedrive }}</span>
        </button>
      </div>
    </section>

    <section
      v-if="shellPlaces?.libraries?.length"
      class="guide-section"
      :aria-label="t('shellPlaces.libraries')"
    >
      <h2 class="guide-section-title">{{ t("shellPlaces.libraries") }}</h2>
      <div class="guide-grid">
        <button
          v-for="lib in shellPlaces.libraries"
          :key="lib.id"
          type="button"
          class="guide-card fluent-surface"
          :title="lib.path"
          @click="onOpen(lib.path)"
        >
          <FluentIcon name="folder" size="md" />
          <span class="guide-card-label">{{ t(`guide.folders.${lib.id}`) }}</span>
          <span class="guide-card-path">{{ lib.path }}</span>
        </button>
      </div>
    </section>

    <section class="guide-section" :aria-label="t('shellPlaces.network')">
      <h2 class="guide-section-title">{{ t("shellPlaces.network") }}</h2>
      <div class="guide-grid">
        <button
          v-for="item in shellPlaces?.network ?? []"
          :key="item.path"
          type="button"
          class="guide-card fluent-surface"
          :title="item.path"
          @click="onOpen(item.path)"
        >
          <FluentIcon name="globe" size="md" />
          <span class="guide-card-label">{{ item.name }}</span>
          <span class="guide-card-path">{{ item.path }}</span>
        </button>
      </div>
      <p v-if="!(shellPlaces?.network?.length)" class="guide-empty">
        {{ t("shellPlaces.networkEmpty") }}
      </p>
    </section>
  </div>
</template>

<style scoped>
.navigator-guide {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: var(--fluent-space-lg) var(--fluent-space-xl);
  background:
    radial-gradient(
      ellipse 70% 45% at 8% 0%,
      color-mix(in srgb, var(--fluent-color-stroke-focus) 14%, transparent),
      transparent 60%
    ),
    var(--fluent-color-layer-default, transparent);
}

.guide-header {
  margin-bottom: var(--fluent-space-lg);
}

.guide-title {
  margin: 0;
  font-size: 1.35rem;
  font-weight: 600;
  color: var(--fluent-color-text-primary);
}

.guide-subtitle {
  margin: var(--fluent-space-xs) 0 0;
  color: var(--fluent-color-text-secondary);
  font-size: 0.875rem;
}

.guide-section {
  margin-bottom: var(--fluent-space-xl);
}

.guide-section-title {
  margin: 0 0 var(--fluent-space-sm);
  font-size: 0.8125rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  text-transform: uppercase;
  color: var(--fluent-color-text-secondary);
}

.guide-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(12.5rem, 1fr));
  gap: var(--fluent-space-sm);
}

.guide-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--fluent-space-xs);
  min-height: 4.5rem;
  padding: var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.guide-card:hover,
.guide-card:focus-visible {
  background: var(--fluent-color-fill-selected);
  outline: none;
}

.guide-card-label {
  font-weight: 600;
  color: var(--fluent-color-text-primary);
}

.guide-card-path {
  font-size: 0.75rem;
  color: var(--fluent-color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.guide-empty {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: 0.875rem;
}
</style>
