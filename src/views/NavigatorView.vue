<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  bootstrapNavigatorOpenState,
  currentPath,
  listing,
  listError,
  listLoading,
  navigateTo,
  openEntry,
  type NavigatorEntry,
} from "../state/navigatorBrowse";
import {
  favoritesDoc,
  loadFavorites,
  loadFavoritesOsHookStatus,
} from "../state/favorites";
import {
  clearClipboard,
  createFolder,
  deleteEntries,
  hasClipboard,
  pasteClipboard,
  pathsWithoutRecycle,
  renameEntry,
  setClipboard,
} from "../state/fileClipboard";
import {
  clearSelection,
  isSelected,
  selectOnly,
  selectedPaths,
} from "../state/navigatorSelection";
import {
  registerShortcutHandler,
  unregisterShortcutHandler,
} from "../shortcuts/index.ts";
import FluentIcon from "../components/icons/FluentIcon.vue";
import NavigatorContextMenu from "../components/navigator/NavigatorContextMenu.vue";
import { entryKindIcon } from "../components/icons/glyphs";

const { t } = useI18n();

const menuOpen = ref(false);
const menuEntry = ref<NavigatorEntry | null>(null);
const menuEmpty = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const notice = ref<string | null>(null);
let noticeTimer: ReturnType<typeof setTimeout> | null = null;

onMounted(() => {
  registerNavFileShortcuts();
  void (async () => {
    await bootstrapNavigatorOpenState();
    await loadFavorites().catch(() => undefined);
    await loadFavoritesOsHookStatus().catch(() => undefined);
    if (currentPath.value && !listing.value && !listError.value && !listLoading.value) {
      await navigateTo(currentPath.value, { syncOpenState: false });
    }
  })();
});

onUnmounted(() => {
  unregisterNavFileShortcuts();
});

async function openFavorite(path: string) {
  await navigateTo(path);
}

function formatSize(entry: NavigatorEntry): string {
  if (entry.kind === "dir") return "—";
  if (entry.size < 1024) return `${entry.size} B`;
  if (entry.size < 1024 * 1024) return `${(entry.size / 1024).toFixed(1)} KB`;
  return `${(entry.size / (1024 * 1024)).toFixed(1)} MB`;
}

function onActivate(entry: NavigatorEntry) {
  void openEntry(entry);
}

function onSelect(entry: NavigatorEntry) {
  selectOnly(entry.path);
}

function onContextMenu(event: MouseEvent, entry: NavigatorEntry) {
  event.preventDefault();
  event.stopPropagation();
  selectOnly(entry.path);
  menuEntry.value = entry;
  menuEmpty.value = false;
  menuX.value = event.clientX;
  menuY.value = event.clientY;
  menuOpen.value = true;
}

function onListContextMenu(event: MouseEvent) {
  if (event.target !== event.currentTarget) return;
  event.preventDefault();
  menuEntry.value = null;
  menuEmpty.value = true;
  menuX.value = event.clientX;
  menuY.value = event.clientY;
  menuOpen.value = true;
}

function closeMenu() {
  menuOpen.value = false;
  menuEmpty.value = false;
}

function onNotice(message: string) {
  notice.value = message;
  if (noticeTimer) clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => {
    notice.value = null;
    noticeTimer = null;
  }, 2800);
}

async function refreshListing() {
  if (currentPath.value) {
    await navigateTo(currentPath.value, { syncOpenState: false });
  }
}

function primaryEntry(): NavigatorEntry | null {
  const path = selectedPaths.value[0];
  if (!path || !listing.value) return null;
  return listing.value.entries.find((e) => e.path === path) ?? null;
}

async function shortcutCopy() {
  const entry = primaryEntry();
  if (!entry) return;
  setClipboard([entry.path], "copy");
  onNotice(t("contextMenu.copied"));
}

async function shortcutCut() {
  const entry = primaryEntry();
  if (!entry) return;
  setClipboard([entry.path], "cut");
  onNotice(t("contextMenu.cutDone"));
}

async function shortcutPaste() {
  if (!hasClipboard.value || !currentPath.value) return;
  try {
    const result = await pasteClipboard(currentPath.value);
    if (!result.ok) {
      onNotice(result.error ?? t("contextMenu.opFailed"));
      return;
    }
    onNotice(t("contextMenu.pasted", { count: result.paths.length }));
    await refreshListing();
  } catch (err) {
    onNotice(err instanceof Error ? err.message : String(err));
  }
}

async function shortcutRename() {
  const entry = primaryEntry();
  if (!entry) return;
  const next = window.prompt(t("contextMenu.renamePrompt"), entry.name);
  if (next == null) return;
  const trimmed = next.trim();
  if (!trimmed || trimmed === entry.name) return;
  try {
    const result = await renameEntry(entry.path, trimmed);
    if (!result.ok) {
      onNotice(result.error ?? t("contextMenu.opFailed"));
      return;
    }
    onNotice(t("contextMenu.renamed"));
    clearSelection();
    await refreshListing();
  } catch (err) {
    onNotice(err instanceof Error ? err.message : String(err));
  }
}

async function shortcutNewFolder() {
  if (!currentPath.value) return;
  const name =
    window.prompt(t("contextMenu.newFolderPrompt"), t("contextMenu.newFolderDefault")) ??
    "";
  const trimmed = name.trim();
  if (!trimmed) return;
  try {
    const result = await createFolder(currentPath.value, trimmed);
    if (!result.ok) {
      onNotice(result.error ?? t("contextMenu.opFailed"));
      return;
    }
    onNotice(t("contextMenu.folderCreated"));
    await refreshListing();
  } catch (err) {
    onNotice(err instanceof Error ? err.message : String(err));
  }
}

async function shortcutDelete() {
  const entry = primaryEntry();
  if (!entry) return;
  try {
    const blocked = await pathsWithoutRecycle([entry.path]);
    if (blocked.length) {
      onNotice(t("contextMenu.deleteNoRecycle"));
      return;
    }
    const ok = window.confirm(t("contextMenu.deleteConfirm", { name: entry.name }));
    if (!ok) return;
    const result = await deleteEntries([entry.path]);
    if (!result.ok) {
      onNotice(result.error ?? t("contextMenu.opFailed"));
      return;
    }
    onNotice(t("contextMenu.deleted"));
    clearSelection();
    clearClipboard();
    await refreshListing();
  } catch (err) {
    onNotice(err instanceof Error ? err.message : String(err));
  }
}

function registerNavFileShortcuts() {
  registerShortcutHandler("navigator.copy", () => {
    void shortcutCopy();
  });
  registerShortcutHandler("navigator.cut", () => {
    void shortcutCut();
  });
  registerShortcutHandler("navigator.paste", () => {
    void shortcutPaste();
  });
  registerShortcutHandler("navigator.rename", () => {
    void shortcutRename();
  });
  registerShortcutHandler("navigator.newFolder", () => {
    void shortcutNewFolder();
  });
  registerShortcutHandler("navigator.delete", () => {
    void shortcutDelete();
  });
}

function unregisterNavFileShortcuts() {
  for (const id of [
    "navigator.copy",
    "navigator.cut",
    "navigator.paste",
    "navigator.rename",
    "navigator.newFolder",
    "navigator.delete",
  ]) {
    unregisterShortcutHandler(id);
  }
}
</script>

<template>
  <section class="fluent-page">
    <header class="fluent-page-header">
      <h1 class="fluent-page-title">{{ t("navigator.title") }}</h1>
      <p class="fluent-page-meta">
        <template v-if="listing">
          {{
            t("navigator.metaConnected", {
              dirs: listing.dirCount,
              files: listing.fileCount,
            })
          }}
        </template>
        <template v-else>{{ t("navigator.metaDisconnected") }}</template>
      </p>
    </header>

    <p v-if="notice" class="fluent-banner fluent-banner--success">{{ notice }}</p>

    <div
      v-if="favoritesDoc.favorites.length"
      class="favorites fluent-surface"
      role="navigation"
      :aria-label="t('favorites.label')"
    >
      <span class="favorites-label">{{ t("favorites.label") }}</span>
      <button
        v-for="fav in favoritesDoc.favorites"
        :key="fav.path"
        type="button"
        class="favorites-chip"
        :title="fav.path"
        @click="openFavorite(fav.path)"
      >
        {{ fav.path.split(/[\\/]/).filter(Boolean).pop() || fav.path }}
      </button>
    </div>

    <p v-if="listLoading" class="fluent-banner">{{ t("navigator.loading") }}</p>
    <p v-else-if="listError" class="fluent-banner fluent-banner--error">{{ listError }}</p>
    <p v-else-if="listing && listing.entries.length === 0" class="fluent-banner">
      {{ t("navigator.empty") }}
    </p>

    <div
      v-if="listing"
      class="fluent-surface list"
      role="list"
      @contextmenu="onListContextMenu"
      @click.self="clearSelection()"
    >
      <button
        v-for="entry in listing.entries"
        :key="entry.path"
        type="button"
        class="fluent-list-row fluent-list-row--density row"
        role="listitem"
        :aria-selected="isSelected(entry.path)"
        :class="{ 'row--selected': isSelected(entry.path) }"
        @click="onSelect(entry)"
        @dblclick="onActivate(entry)"
        @keydown.enter.prevent="onActivate(entry)"
        @contextmenu="onContextMenu($event, entry)"
      >
        <span
          class="kind"
          :data-kind="entry.kind"
          :title="entry.kind === 'dir' ? t('navigator.folder') : t('navigator.file')"
        >
          <FluentIcon :name="entryKindIcon(entry.kind === 'dir')" size="md" />
        </span>
        <span class="name">{{ entry.name }}</span>
        <span class="size">{{ formatSize(entry) }}</span>
      </button>
    </div>

    <NavigatorContextMenu
      :open="menuOpen"
      :entry="menuEntry"
      :empty-area="menuEmpty"
      :x="menuX"
      :y="menuY"
      @close="closeMenu"
      @notice="onNotice"
      @favorites-changed="loadFavorites"
      @listing-changed="refreshListing"
    />
  </section>
</template>

<style scoped>
.favorites {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fluent-space-xs);
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  margin-bottom: var(--fluent-space-sm);
}

.favorites-label {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  margin-right: var(--fluent-space-xs);
}

.favorites-chip {
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: inherit;
  font: inherit;
  font-size: var(--fluent-font-size-caption);
  padding: 0.15rem 0.55rem;
  max-width: 12rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  cursor: pointer;
}

.favorites-chip:hover,
.favorites-chip:focus-visible {
  background: var(--fluent-color-fill-selected);
  outline: none;
}

.list {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.row {
  width: 100%;
  border: 0;
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: 0;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: default;
  display: grid;
  grid-template-columns: 2rem 1fr 5.5rem;
  gap: var(--fluent-space-sm);
}

.row:last-child {
  border-bottom: 0;
}

.row:hover,
.row:focus-visible {
  background: var(--fluent-color-fill-selected);
  outline: none;
}

.row--selected {
  background: var(--fluent-color-fill-selected);
}

.kind {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--fluent-color-text-secondary);
}

.kind[data-kind="dir"] {
  color: var(--fluent-color-fill-accent);
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.size {
  text-align: right;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  font-variant-numeric: tabular-nums;
}
</style>
