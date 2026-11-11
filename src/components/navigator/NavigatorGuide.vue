<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import FluentIcon from "../icons/FluentIcon.vue";
import type { FluentGlyphId } from "../icons/glyphs";
import { favoritesDoc, loadFavorites } from "../../state/favorites";
import { quickAccessList, loadVisitStats } from "../../state/visitStats";
import {
  drivesList,
  fetchDriveSpaces,
  loadNavigatorPlaces,
  shellPlaces,
  userFolderItems,
  userFolders,
  type DiskSpaceInfo,
  type DriveInfo,
  type DriveKind,
  type UserFolderId,
} from "../../state/guide";
import { formatBytes } from "../../state/formatBytes";
import { scheduleAfterPaint } from "../../state/scheduleIdle";
import { RECYCLE_PATH } from "../../state/recycle";
import {
  listProviderBackends,
  PROVIDER_ROOT,
  type ProviderBackendInfo,
} from "../../state/provider";

const props = defineProps<{
  /** Navigate the current tab to this OS path. */
  openPath: (path: string) => void | Promise<void>;
}>();

const { t } = useI18n();

const folderRows = computed(() => userFolderItems(userFolders.value));
const providerBackends = ref<ProviderBackendInfo[]>([]);
const driveSpace = ref<Record<string, DiskSpaceInfo>>({});

type GuideTone =
  | "home"
  | "desktop"
  | "documents"
  | "downloads"
  | "pictures"
  | "music"
  | "videos"
  | "recent"
  | "favorites"
  | "drive-fixed"
  | "drive-removable"
  | "drive-remote"
  | "drive-cdrom"
  | "drive-ramdisk"
  | "drive-unknown"
  | "recycle"
  | "providers"
  | "onedrive"
  | "libraries"
  | "network";

const FOLDER_TONE: Record<UserFolderId, GuideTone> = {
  home: "home",
  desktop: "desktop",
  documents: "documents",
  downloads: "downloads",
  pictures: "pictures",
  music: "music",
  videos: "videos",
};

const FOLDER_ICON: Record<UserFolderId, FluentGlyphId> = {
  home: "home",
  desktop: "desktop",
  documents: "document",
  downloads: "download",
  pictures: "picture",
  music: "musicNote",
  videos: "video",
};

function folderLabel(id: UserFolderId): string {
  return t(`guide.folders.${id}`);
}

function favoriteLabel(path: string): string {
  const parts = path.replace(/\\/g, "/").replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || path;
}

function normalizeDriveKind(kind: DriveInfo["kind"]): DriveKind {
  return kind ?? "unknown";
}

function driveTone(kind: DriveInfo["kind"]): GuideTone {
  switch (normalizeDriveKind(kind)) {
    case "removable":
      return "drive-removable";
    case "remote":
      return "drive-remote";
    case "cdrom":
      return "drive-cdrom";
    case "ramdisk":
      return "drive-ramdisk";
    case "fixed":
      return "drive-fixed";
    default:
      return "drive-unknown";
  }
}

function driveIcon(kind: DriveInfo["kind"]): FluentGlyphId {
  switch (normalizeDriveKind(kind)) {
    case "removable":
      return "usbDrive";
    case "remote":
      return "globe";
    case "cdrom":
      return "opticalDrive";
    case "ramdisk":
      return "flash";
    case "fixed":
    default:
      return "hardDrive";
  }
}

function driveLabel(drive: DriveInfo): string {
  const letter = drive.letter;
  const label = (drive.label || "").trim();
  if (label && label.toLowerCase() !== letter.toLowerCase()) {
    return t("guide.driveNamed", { letter, label });
  }
  switch (normalizeDriveKind(drive.kind)) {
    case "removable":
      return t("guide.driveRemovable", { letter });
    case "remote":
      return t("guide.driveRemote", { letter });
    case "cdrom":
      return t("guide.driveCdrom", { letter });
    case "ramdisk":
      return t("guide.driveRamdisk", { letter });
    default:
      return t("guide.drive", { letter });
  }
}

function providerDetail(backend: ProviderBackendInfo): string {
  return backend.detail || backend.description || `awam://${backend.id}`;
}

function capacityText(info: DiskSpaceInfo | undefined): string {
  if (!info || !info.totalBytes) return "";
  return t("guide.driveCapacity", {
    free: formatBytes(info.freeBytes),
    total: formatBytes(info.totalBytes),
  });
}

function usedRatio(info: DiskSpaceInfo | undefined): number {
  if (!info || !info.totalBytes) return 0;
  const used = Math.max(0, info.totalBytes - info.freeBytes);
  return Math.min(1, used / info.totalBytes);
}

function usedPercent(info: DiskSpaceInfo | undefined): number {
  return Math.round(usedRatio(info) * 100);
}

function capacityCritical(info: DiskSpaceInfo | undefined): boolean {
  return usedRatio(info) >= 0.9;
}

function refreshDriveSpaces() {
  const drives = drivesList.value;
  if (!drives.length) {
    driveSpace.value = {};
    return;
  }
  // FIL-25：容量查询延后到首帧后，避免引导页路径栏/切换卡顿。
  scheduleAfterPaint(() => {
    void fetchDriveSpaces(drives).then((map) => {
      driveSpace.value = map;
    });
  });
}

onMounted(() => {
  void loadNavigatorPlaces().then(() => {
    refreshDriveSpaces();
  });
  void loadFavorites().catch(() => undefined);
  void loadVisitStats().catch(() => undefined);
  void listProviderBackends()
    .then((list) => {
      providerBackends.value = list;
    })
    .catch(() => {
      providerBackends.value = [];
    });
});

watch(drivesList, () => {
  refreshDriveSpaces();
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
          :class="`guide-card--${FOLDER_TONE[item.id]}`"
          :title="item.path"
          @click="onOpen(item.path)"
        >
          <span class="guide-card-icon" aria-hidden="true">
            <FluentIcon :name="FOLDER_ICON[item.id]" size="md" />
          </span>
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
          class="guide-card fluent-surface guide-card--recent"
          :title="item.path"
          @click="onOpen(item.path)"
        >
          <span class="guide-card-icon" aria-hidden="true">
            <FluentIcon name="folderOpen" size="md" />
          </span>
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
          class="guide-card fluent-surface guide-card--favorites"
          :title="fav.path"
          @click="onOpen(fav.path)"
        >
          <span class="guide-card-icon" aria-hidden="true">
            <FluentIcon name="star" size="md" />
          </span>
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
      <div class="guide-grid guide-grid--drives">
        <button
          v-for="drive in drivesList"
          :key="drive.path"
          type="button"
          class="guide-card fluent-surface guide-card--drive"
          :class="`guide-card--${driveTone(drive.kind)}`"
          :title="drive.path"
          @click="onOpen(drive.path)"
        >
          <span class="guide-card-icon" aria-hidden="true">
            <FluentIcon :name="driveIcon(drive.kind)" size="md" />
          </span>
          <span class="guide-card-label">{{ driveLabel(drive) }}</span>
          <span
            v-if="driveSpace[drive.path]"
            class="guide-capacity"
            :class="{ 'guide-capacity--critical': capacityCritical(driveSpace[drive.path]) }"
          >
            <span
              class="guide-capacity-track"
              role="progressbar"
              :aria-valuemin="0"
              :aria-valuemax="100"
              :aria-valuenow="usedPercent(driveSpace[drive.path])"
              :aria-label="capacityText(driveSpace[drive.path])"
            >
              <span
                class="guide-capacity-fill"
                :style="{ width: `${usedPercent(driveSpace[drive.path])}%` }"
              />
            </span>
            <span class="guide-capacity-text">{{ capacityText(driveSpace[drive.path]) }}</span>
          </span>
          <span v-else class="guide-card-path">{{ drive.path }}</span>
        </button>
      </div>
      <p v-if="!drivesList.length" class="guide-empty">{{ t("guide.emptyDrives") }}</p>
    </section>

    <section class="guide-section" :aria-label="t('recycleBin.section')">
      <h2 class="guide-section-title">{{ t("recycleBin.section") }}</h2>
      <div class="guide-grid">
        <button
          type="button"
          class="guide-card fluent-surface guide-card--recycle"
          :title="t('recycleBin.subtitle')"
          @click="onOpen(RECYCLE_PATH)"
        >
          <span class="guide-card-icon" aria-hidden="true">
            <FluentIcon name="recycle" size="md" />
          </span>
          <span class="guide-card-label">{{ t("recycleBin.tabName") }}</span>
          <span class="guide-card-path">{{ t("recycleBin.subtitle") }}</span>
        </button>
      </div>
    </section>

    <section class="guide-section" :aria-label="t('guide.sections.providers')">
      <h2 class="guide-section-title">{{ t("guide.sections.providers") }}</h2>
      <div class="guide-grid">
        <button
          type="button"
          class="guide-card fluent-surface guide-card--providers"
          :title="PROVIDER_ROOT"
          @click="onOpen(PROVIDER_ROOT)"
        >
          <span class="guide-card-icon" aria-hidden="true">
            <FluentIcon name="cloud" size="md" />
          </span>
          <span class="guide-card-label">{{ t("guide.providersRoot") }}</span>
          <span class="guide-card-path">{{ PROVIDER_ROOT }}</span>
        </button>
        <button
          v-for="backend in providerBackends"
          :key="backend.id"
          type="button"
          class="guide-card fluent-surface guide-card--providers"
          :title="`awam://${backend.id}`"
          @click="onOpen(`awam://${backend.id}`)"
        >
          <span class="guide-card-icon" aria-hidden="true">
            <FluentIcon name="cloud" size="md" />
          </span>
          <span class="guide-card-label">{{ backend.name || backend.id }}</span>
          <span class="guide-card-path">{{ providerDetail(backend) }}</span>
        </button>
      </div>
      <p v-if="!providerBackends.length" class="guide-empty">
        {{ t("guide.emptyProviders") }}
      </p>
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
          class="guide-card fluent-surface guide-card--onedrive"
          :title="shellPlaces.onedrive"
          @click="onOpen(shellPlaces.onedrive)"
        >
          <span class="guide-card-icon" aria-hidden="true">
            <FluentIcon name="cloud" size="md" />
          </span>
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
          class="guide-card fluent-surface guide-card--libraries"
          :title="lib.path"
          @click="onOpen(lib.path)"
        >
          <span class="guide-card-icon" aria-hidden="true">
            <FluentIcon name="library" size="md" />
          </span>
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
          class="guide-card fluent-surface guide-card--network"
          :title="item.path"
          @click="onOpen(item.path)"
        >
          <span class="guide-card-icon" aria-hidden="true">
            <FluentIcon name="globe" size="md" />
          </span>
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

.guide-grid--drives {
  grid-template-columns: repeat(auto-fill, minmax(14.5rem, 1fr));
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
  --guide-tone: var(--fluent-color-fill-accent);
  --guide-tone-wash: var(--fluent-color-icon-wash);
}

.guide-card:hover,
.guide-card:focus-visible {
  background: var(--fluent-color-fill-selected);
  outline: none;
}

.guide-card-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.75rem;
  height: 1.75rem;
  border-radius: var(--fluent-radius-sm);
  background: var(--guide-tone-wash);
  color: var(--guide-tone);
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

.guide-capacity {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
  margin-top: 2px;
  --guide-capacity-fill: var(--fluent-color-fill-accent);
}

.guide-capacity--critical {
  --guide-capacity-fill: var(--fluent-color-status-danger);
}

.guide-capacity-track {
  display: block;
  width: 100%;
  height: 4px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--fluent-color-stroke-divider) 80%, transparent);
  overflow: hidden;
}

.guide-capacity-fill {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--guide-capacity-fill);
  transition: width 160ms ease;
}

.guide-capacity-text {
  font-size: 0.75rem;
  color: var(--fluent-color-text-secondary);
  line-height: 1.3;
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

.guide-card--home {
  --guide-tone: #005fb8;
  --guide-tone-wash: color-mix(in srgb, #005fb8 14%, transparent);
}
.guide-card--desktop {
  --guide-tone: #0f7b7b;
  --guide-tone-wash: color-mix(in srgb, #0f7b7b 14%, transparent);
}
.guide-card--documents {
  --guide-tone: #2b6cb0;
  --guide-tone-wash: color-mix(in srgb, #2b6cb0 14%, transparent);
}
.guide-card--downloads {
  --guide-tone: #0f7b0f;
  --guide-tone-wash: color-mix(in srgb, #0f7b0f 14%, transparent);
}
.guide-card--pictures {
  --guide-tone: #c45c26;
  --guide-tone-wash: color-mix(in srgb, #c45c26 14%, transparent);
}
.guide-card--music {
  --guide-tone: #b23a6d;
  --guide-tone-wash: color-mix(in srgb, #b23a6d 14%, transparent);
}
.guide-card--videos {
  --guide-tone: #1f5f8b;
  --guide-tone-wash: color-mix(in srgb, #1f5f8b 14%, transparent);
}
.guide-card--recent {
  --guide-tone: #9d5d00;
  --guide-tone-wash: color-mix(in srgb, #9d5d00 14%, transparent);
}
.guide-card--favorites {
  --guide-tone: #b8860b;
  --guide-tone-wash: color-mix(in srgb, #b8860b 14%, transparent);
}
.guide-card--drive-fixed {
  --guide-tone: #005fb8;
  --guide-tone-wash: color-mix(in srgb, #005fb8 14%, transparent);
}
.guide-card--drive-removable {
  --guide-tone: #0f7b0f;
  --guide-tone-wash: color-mix(in srgb, #0f7b0f 14%, transparent);
}
.guide-card--drive-remote {
  --guide-tone: #0078d4;
  --guide-tone-wash: color-mix(in srgb, #0078d4 14%, transparent);
}
.guide-card--drive-cdrom {
  --guide-tone: #5d6b7a;
  --guide-tone-wash: color-mix(in srgb, #5d6b7a 14%, transparent);
}
.guide-card--drive-ramdisk {
  --guide-tone: #0f7b7b;
  --guide-tone-wash: color-mix(in srgb, #0f7b7b 14%, transparent);
}
.guide-card--drive-unknown {
  --guide-tone: #5d5d5d;
  --guide-tone-wash: color-mix(in srgb, #5d5d5d 14%, transparent);
}
.guide-card--recycle {
  --guide-tone: #c42b1c;
  --guide-tone-wash: color-mix(in srgb, #c42b1c 12%, transparent);
}
.guide-card--providers {
  --guide-tone: #0078d4;
  --guide-tone-wash: color-mix(in srgb, #0078d4 14%, transparent);
}
.guide-card--onedrive {
  --guide-tone: #094ab2;
  --guide-tone-wash: color-mix(in srgb, #094ab2 14%, transparent);
}
.guide-card--libraries {
  --guide-tone: #486581;
  --guide-tone-wash: color-mix(in srgb, #486581 14%, transparent);
}
.guide-card--network {
  --guide-tone: #0f6e56;
  --guide-tone-wash: color-mix(in srgb, #0f6e56 14%, transparent);
}

@media (max-width: 640px) {
  .navigator-guide {
    padding: var(--fluent-space-md);
  }

  .guide-grid,
  .guide-grid--drives {
    grid-template-columns: 1fr;
  }

  .guide-capacity-text {
    white-space: normal;
  }
}
</style>
