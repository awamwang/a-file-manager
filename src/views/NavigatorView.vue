<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
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
import { iconUrlFor, requestShellIcons } from "../state/shellIcons";
import {
  loadFavorites,
  loadFavoritesOsHookStatus,
} from "../state/favorites";
import {
  clearClipboard,
  createFile,
  createFolder,
  deleteEntries,
  hasClipboard,
  pasteClipboard,
  pathsWithoutRecycle,
  renameEntry,
  setClipboard,
  type FileOpsResult,
} from "../state/fileClipboard";
import {
  resolveNewFileDefaultName,
  resolveNewFolderDefaultName,
} from "../state/newItemNames";
import {
  clearSelection,
  isSelected,
  selectOnly,
  selectRange,
  selectionAnchor,
  selectedPaths,
  setSelection,
  toggleInSelection,
} from "../state/navigatorSelection";
import { sortedEntries, sortColumn, sortDirection, toggleSort } from "../state/navigatorSort";
import {
  activePane,
  dualPaneEnabled,
  navigateSecondary,
  secondaryError,
  secondaryListing,
  secondaryLoading,
  secondaryPath,
  setActivePane,
} from "../state/dualPane";
import {
  entryMatchesFilter,
  navigatorFilterQuery,
} from "../state/navigatorFilter";
import { toastError, toastSuccess } from "../state/toast";
import {
  registerShortcutHandler,
  unregisterShortcutHandler,
} from "../shortcuts/index.ts";
import FluentIcon from "../components/icons/FluentIcon.vue";
import NavigatorContextMenu from "../components/navigator/NavigatorContextMenu.vue";
import PropertiesPanel from "../components/navigator/PropertiesPanel.vue";
import { entryKindIcon } from "../components/icons/glyphs";

const { t } = useI18n();

const menuOpen = ref(false);
const menuEntry = ref<NavigatorEntry | null>(null);
const menuEmpty = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const propertiesOpen = ref(false);
const propertiesPath = ref<string | null>(null);

function openProperties(path: string) {
  propertiesPath.value = path;
  propertiesOpen.value = true;
}

function closeProperties() {
  propertiesOpen.value = false;
  propertiesPath.value = null;
}

const listRoot = ref<HTMLElement | null>(null);
const marquee = ref<{ x: number; y: number; w: number; h: number } | null>(null);
let marqueeOrigin: { x: number; y: number } | null = null;
let marqueeActive = false;
const dragOverPath = ref<string | null>(null);

const visibleEntries = computed(() => {
  if (!listing.value) return [];
  const sorted = sortedEntries(listing.value.entries);
  const q = navigatorFilterQuery.value;
  if (!q.trim()) return sorted;
  return sorted.filter((entry) => entryMatchesFilter(entry.name, entry.path, q));
});

const secondaryEntries = computed(() => {
  if (!secondaryListing.value) return [];
  const sorted = sortedEntries(secondaryListing.value.entries);
  const q = navigatorFilterQuery.value;
  if (!q.trim()) return sorted;
  return sorted.filter((entry) => entryMatchesFilter(entry.name, entry.path, q));
});

const orderedPaths = computed(() => visibleEntries.value.map((e) => e.path));

watch(
  visibleEntries,
  (entries) => {
    requestShellIcons(entries.map((e) => e.path));
  },
  { immediate: true },
);

watch(
  secondaryEntries,
  (entries) => {
    requestShellIcons(entries.map((e) => e.path));
  },
  { immediate: true },
);

onMounted(() => {
  registerNavFileShortcuts();
  window.addEventListener("mousemove", onMarqueeMove);
  window.addEventListener("mouseup", onMarqueeEnd);
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
  window.removeEventListener("mousemove", onMarqueeMove);
  window.removeEventListener("mouseup", onMarqueeEnd);
});

function formatSize(entry: NavigatorEntry): string {
  if (entry.kind === "dir") return "—";
  if (entry.size < 1024) return `${entry.size} B`;
  if (entry.size < 1024 * 1024) return `${(entry.size / 1024).toFixed(1)} KB`;
  return `${(entry.size / (1024 * 1024)).toFixed(1)} MB`;
}

function sortIndicator(column: "name" | "size"): string {
  if (sortColumn.value !== column) return "";
  return sortDirection.value === "asc" ? " ▲" : " ▼";
}

function onActivate(entry: NavigatorEntry) {
  if (activePane.value === "secondary" && dualPaneEnabled.value) {
    if (entry.kind === "dir") void navigateSecondary(entry.path);
    return;
  }
  void openEntry(entry);
}

function onActivateSecondary(entry: NavigatorEntry) {
  setActivePane("secondary");
  if (entry.kind === "dir") {
    void navigateSecondary(entry.path);
    return;
  }
  // AIA-118: file activate follows the same Settings policy as primary pane.
  void openEntry(entry);
}

function onRowClick(event: MouseEvent, entry: NavigatorEntry) {
  if (event.shiftKey) {
    selectRange(orderedPaths.value, selectionAnchor.value, entry.path);
    return;
  }
  if (event.ctrlKey || event.metaKey) {
    toggleInSelection(entry.path);
    return;
  }
  selectOnly(entry.path);
}

function onContextMenu(event: MouseEvent, entry: NavigatorEntry) {
  event.preventDefault();
  event.stopPropagation();
  if (!isSelected(entry.path)) {
    selectOnly(entry.path);
  }
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

function onNotice(message: string, kind: "success" | "error" = "success") {
  if (kind === "error") toastError(message);
  else toastSuccess(message);
}

async function refreshListing() {
  if (currentPath.value) {
    await navigateTo(currentPath.value, { syncOpenState: false });
  }
}

function primaryEntry(): NavigatorEntry | null {
  const path = selectedPaths.value[0];
  if (!path) return null;
  return visibleEntries.value.find((e) => e.path === path) ?? null;
}

function selectionPaths(): string[] {
  return selectedPaths.value.length
    ? [...selectedPaths.value]
    : primaryEntry()
      ? [primaryEntry()!.path]
      : [];
}

async function shortcutCopy() {
  const paths = selectionPaths();
  if (!paths.length) return;
  setClipboard(paths, "copy");
  onNotice(t("contextMenu.copied"));
}

async function shortcutCut() {
  const paths = selectionPaths();
  if (!paths.length) return;
  setClipboard(paths, "cut");
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
  const suggested = await resolveNewFolderDefaultName(t("contextMenu.newFolderDefault"));
  const name = window.prompt(t("contextMenu.newFolderPrompt"), suggested) ?? "";
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

async function shortcutNewFile() {
  if (!currentPath.value) return;
  const suggested = await resolveNewFileDefaultName(t("contextMenu.newFileDefault"));
  const name = window.prompt(t("contextMenu.newFilePrompt"), suggested) ?? "";
  const trimmed = name.trim();
  if (!trimmed) return;
  try {
    const result = await createFile(currentPath.value, trimmed);
    if (!result.ok) {
      onNotice(result.error ?? t("contextMenu.opFailed"));
      return;
    }
    onNotice(t("contextMenu.fileCreated"));
    await refreshListing();
  } catch (err) {
    onNotice(err instanceof Error ? err.message : String(err));
  }
}

async function shortcutDelete() {
  const paths = selectionPaths();
  if (!paths.length) return;
  try {
    const blocked = await pathsWithoutRecycle(paths);
    if (blocked.length) {
      onNotice(t("contextMenu.deleteNoRecycle"), "error");
      return;
    }
    const label =
      paths.length === 1
        ? visibleEntries.value.find((e) => e.path === paths[0])?.name ?? paths[0]
        : t("navigator.nItems", { count: paths.length });
    const ok = window.confirm(t("contextMenu.deleteConfirm", { name: label }));
    if (!ok) return;
    const result = await deleteEntries(paths);
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
  registerShortcutHandler("navigator.newFile", () => {
    void shortcutNewFile();
  });
  registerShortcutHandler("navigator.refresh", () => {
    void refreshListing();
  });
  registerShortcutHandler("navigator.delete", () => {
    void shortcutDelete();
  });
  registerShortcutHandler("navigator.selectAll", () => {
    setSelection(orderedPaths.value);
  });
}

function unregisterNavFileShortcuts() {
  for (const id of [
    "navigator.copy",
    "navigator.cut",
    "navigator.paste",
    "navigator.rename",
    "navigator.newFolder",
    "navigator.newFile",
    "navigator.refresh",
    "navigator.delete",
    "navigator.selectAll",
  ]) {
    unregisterShortcutHandler(id);
  }
}

/* —— Marquee (box) select —— */
function onListMouseDown(event: MouseEvent) {
  if (event.button !== 0) return;
  if (event.target !== event.currentTarget) return;
  const root = listRoot.value;
  if (!root) return;
  const rect = root.getBoundingClientRect();
  marqueeOrigin = { x: event.clientX - rect.left + root.scrollLeft, y: event.clientY - rect.top + root.scrollTop };
  marqueeActive = true;
  marquee.value = { x: marqueeOrigin.x, y: marqueeOrigin.y, w: 0, h: 0 };
  if (!event.ctrlKey && !event.metaKey && !event.shiftKey) {
    clearSelection();
  }
}

function onMarqueeMove(event: MouseEvent) {
  if (!marqueeActive || !marqueeOrigin || !listRoot.value) return;
  const root = listRoot.value;
  const rect = root.getBoundingClientRect();
  const x = event.clientX - rect.left + root.scrollLeft;
  const y = event.clientY - rect.top + root.scrollTop;
  const left = Math.min(marqueeOrigin.x, x);
  const top = Math.min(marqueeOrigin.y, y);
  const w = Math.abs(x - marqueeOrigin.x);
  const h = Math.abs(y - marqueeOrigin.y);
  marquee.value = { x: left, y: top, w, h };
  const hit = hitTestRows(left, top, w, h);
  setSelection(hit, hit[0] ?? null);
}

function onMarqueeEnd() {
  marqueeActive = false;
  marqueeOrigin = null;
  marquee.value = null;
}

function hitTestRows(x: number, y: number, w: number, h: number): string[] {
  const root = listRoot.value;
  if (!root || w < 2 || h < 2) return [];
  const box = { left: x, top: y, right: x + w, bottom: y + h };
  const hits: string[] = [];
  const rows = root.querySelectorAll<HTMLElement>("[data-entry-path]");
  for (const row of rows) {
    const path = row.dataset.entryPath;
    if (!path) continue;
    const top = row.offsetTop;
    const left = row.offsetLeft;
    const right = left + row.offsetWidth;
    const bottom = top + row.offsetHeight;
    if (left < box.right && right > box.left && top < box.bottom && bottom > box.top) {
      hits.push(path);
    }
  }
  return hits;
}

/* —— Drag into folder —— */
function onDragStart(event: DragEvent, entry: NavigatorEntry) {
  if (!isSelected(entry.path)) {
    selectOnly(entry.path);
  }
  const paths = selectionPaths();
  event.dataTransfer?.setData("application/x-afm-paths", JSON.stringify(paths));
  event.dataTransfer!.effectAllowed = "move";
}

function onDragOver(event: DragEvent, entry: NavigatorEntry) {
  if (entry.kind !== "dir") return;
  if (isSelected(entry.path)) return;
  event.preventDefault();
  event.dataTransfer!.dropEffect = "move";
  dragOverPath.value = entry.path;
}

function onDragLeave(entry: NavigatorEntry) {
  if (dragOverPath.value === entry.path) {
    dragOverPath.value = null;
  }
}

async function onDrop(event: DragEvent, entry: NavigatorEntry) {
  event.preventDefault();
  dragOverPath.value = null;
  if (entry.kind !== "dir") return;
  let paths: string[] = [];
  try {
    const raw = event.dataTransfer?.getData("application/x-afm-paths") ?? "[]";
    paths = JSON.parse(raw) as string[];
  } catch {
    return;
  }
  paths = paths.filter((p) => p !== entry.path);
  if (!paths.length) return;
  try {
    const result = await invoke<FileOpsResult>("file_ops_move", {
      sources: paths,
      destDir: entry.path,
    });
    if (!result.ok) {
      onNotice(result.error ?? t("contextMenu.opFailed"));
      return;
    }
    onNotice(t("navigator.movedInto", { count: result.paths.length, name: entry.name }));
    clearSelection();
    await refreshListing();
  } catch (err) {
    onNotice(err instanceof Error ? err.message : String(err));
  }
}
</script>

<template>
  <!-- AIA-78: no in-page title / Kernel status meta — path via shell address bar + pane-path; counts in shell statusbar. -->
  <section class="fluent-page fluent-page--navigator">
    <div class="panes" :class="{ 'panes--dual': dualPaneEnabled }">
    <div
      class="pane"
      :class="{ 'pane--active': activePane === 'primary' || !dualPaneEnabled }"
      @mousedown="setActivePane('primary')"
    >
    <p class="pane-path" :title="currentPath">{{ currentPath || t("navigator.noPath") }}</p>
    <p v-if="listLoading" class="fluent-banner">{{ t("navigator.loading") }}</p>
    <p v-else-if="listError" class="fluent-banner fluent-banner--error">{{ listError }}</p>
    <p v-else-if="listing && listing.entries.length === 0" class="fluent-banner">
      {{ t("navigator.empty") }}
    </p>
    <p
      v-else-if="listing && listing.entries.length > 0 && !visibleEntries.length"
      class="fluent-banner fluent-banner--info"
    >
      {{ t("navigator.filterEmpty") }}
    </p>

    <div
      v-if="listing"
      ref="listRoot"
      class="fluent-surface list"
      role="list"
      @contextmenu="onListContextMenu"
      @mousedown="onListMouseDown"
      @click.self="clearSelection()"
    >
      <div class="list-header" role="row">
        <button type="button" class="col-sort col-name" @click="toggleSort('name')">
          {{ t("navigator.colName") }}{{ sortIndicator("name") }}
        </button>
        <button type="button" class="col-sort col-size" @click="toggleSort('size')">
          {{ t("navigator.colSize") }}{{ sortIndicator("size") }}
        </button>
      </div>

      <div
        v-for="entry in visibleEntries"
        :key="entry.path"
        class="fluent-list-row fluent-list-row--density row"
        role="listitem"
        tabindex="0"
        draggable="true"
        :data-entry-path="entry.path"
        :aria-selected="isSelected(entry.path)"
        :class="{
          'row--selected': isSelected(entry.path),
          'row--drop': dragOverPath === entry.path,
        }"
        @click="onRowClick($event, entry)"
        @dblclick="onActivate(entry)"
        @keydown.enter.prevent="onActivate(entry)"
        @contextmenu="onContextMenu($event, entry)"
        @dragstart="onDragStart($event, entry)"
        @dragover="onDragOver($event, entry)"
        @dragleave="onDragLeave(entry)"
        @drop="onDrop($event, entry)"
      >
        <span
          class="kind"
          :data-kind="entry.kind"
          :title="entry.kind === 'dir' ? t('navigator.folder') : t('navigator.file')"
        >
          <img
            v-if="iconUrlFor(entry.path)"
            class="shell-icon"
            :src="iconUrlFor(entry.path)!"
            alt=""
            width="16"
            height="16"
          />
          <FluentIcon v-else :name="entryKindIcon(entry.kind === 'dir')" size="md" />
        </span>
        <span class="name">{{ entry.name }}</span>
        <span class="size">{{ formatSize(entry) }}</span>
      </div>

      <div
        v-if="marquee"
        class="marquee"
        :style="{
          left: `${marquee.x}px`,
          top: `${marquee.y}px`,
          width: `${marquee.w}px`,
          height: `${marquee.h}px`,
        }"
      />
    </div>
    </div>

    <div
      v-if="dualPaneEnabled"
      class="pane pane--secondary"
      :class="{ 'pane--active': activePane === 'secondary' }"
      @mousedown="setActivePane('secondary')"
    >
      <p class="pane-path" :title="secondaryPath">
        {{ secondaryPath || t("navigator.noPath") }}
      </p>
      <p v-if="secondaryLoading" class="fluent-banner">{{ t("navigator.loading") }}</p>
      <p v-else-if="secondaryError" class="fluent-banner fluent-banner--error">{{ secondaryError }}</p>
      <div v-else-if="secondaryListing" class="fluent-surface list" role="list">
        <div
          v-for="entry in secondaryEntries"
          :key="entry.path"
          class="fluent-list-row fluent-list-row--density row"
          role="listitem"
          tabindex="0"
          @dblclick="onActivateSecondary(entry)"
          @keydown.enter.prevent="onActivateSecondary(entry)"
        >
          <span class="kind" :data-kind="entry.kind">
            <img
              v-if="iconUrlFor(entry.path)"
              class="shell-icon"
              :src="iconUrlFor(entry.path)!"
              alt=""
              width="16"
              height="16"
            />
            <FluentIcon v-else :name="entryKindIcon(entry.kind === 'dir')" size="md" />
          </span>
          <span class="name">{{ entry.name }}</span>
          <span class="size">{{ formatSize(entry) }}</span>
        </div>
      </div>
    </div>
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
      @open-properties="openProperties"
    />
    <PropertiesPanel
      :open="propertiesOpen"
      :path="propertiesPath"
      @close="closeProperties"
    />
  </section>
</template>

<style scoped>
.fluent-page--navigator {
  gap: var(--fluent-space-sm);
  height: 100%;
  min-height: 0;
}

.panes {
  display: flex;
  flex: 1;
  min-height: 0;
  gap: 0;
}

.panes--dual {
  gap: var(--fluent-space-sm);
}

.panes--dual .pane {
  /* Narrow-window guard: keep each pane usable (UI audit leftover). */
  min-width: min(12rem, 40%);
  flex: 1 1 12rem;
}

.pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  border: var(--fluent-stroke-width) solid transparent;
  border-radius: var(--fluent-radius-md);
}

.pane--active {
  border-color: var(--fluent-color-stroke-focus);
}

.pane-path {
  margin: 0 0 var(--fluent-space-xs);
  padding: 0 var(--fluent-space-xs);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.list {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow: auto;
  user-select: none;
}

.list-header {
  display: grid;
  grid-template-columns: 2rem 1fr 5.5rem;
  gap: var(--fluent-space-sm);
  position: sticky;
  top: 0;
  z-index: 1;
  padding-inline: 0;
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
}

.col-sort {
  border: 0;
  background: transparent;
  color: var(--fluent-color-text-secondary);
  font: inherit;
  font-size: var(--fluent-font-size-caption);
  text-align: left;
  padding: var(--fluent-space-xs) var(--fluent-space-sm);
  cursor: pointer;
}

.col-sort:hover,
.col-sort:focus-visible {
  color: var(--fluent-color-text-primary);
  background: var(--fluent-color-fill-selected);
  outline: none;
}

.col-name {
  grid-column: 2;
}

.col-size {
  grid-column: 3;
  text-align: right;
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

.row--drop {
  outline: 2px solid var(--fluent-color-stroke-focus);
  outline-offset: -2px;
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

.shell-icon {
  width: 16px;
  height: 16px;
  object-fit: contain;
  flex-shrink: 0;
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

.marquee {
  position: absolute;
  pointer-events: none;
  border: 1px solid var(--fluent-color-stroke-focus);
  background: color-mix(in srgb, var(--fluent-color-fill-accent) 18%, transparent);
  z-index: 2;
}
</style>
