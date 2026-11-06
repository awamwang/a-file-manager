<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
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
  dirEntryFromPath,
  folderContextMenuRequest,
} from "../state/folderContextMenu";
import { isGuidePath } from "../state/navigatorGuide";
import NavigatorGuide from "../components/navigator/NavigatorGuide.vue";
import { hydrateSettings } from "../state/settingsHydrate";
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
  refreshClipboardFromSystem,
  renameEntry,
  setClipboard,
  transferEntries,
} from "../state/fileClipboard";
import { confirmDeleteIfNeeded, confirmPermanentDelete } from "../state/confirmBeforeDelete";
import { openPromptDialog } from "../state/promptDialog";
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
  isDetailsView,
  isIconView,
  isListView,
  loadNavigatorViewMode,
  navigatorViewMode,
  VIEW_ICON_SIZE,
} from "../state/navigatorViewMode";
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
  navigatorFilterQuery,
  primarySearch,
  primarySearchLoading,
  scheduleFolderSearch,
  secondarySearch,
} from "../state/navigatorFilter";
import { toastError, toastInfo, toastSuccess } from "../state/toast";
import { armNativeFileDragOut } from "../state/fileDragOut";
import { dropTransferMode } from "../state/dropMode";
import {
  invokePreviewBridge,
  previewBridgeForcePane,
  previewBridgeSessionActive,
  switchPreviewBridge,
} from "../state/previewBridge";
import {
  registerShortcutHandler,
  unregisterShortcutHandler,
} from "../shortcuts/index.ts";
import FluentIcon from "../components/icons/FluentIcon.vue";
import NavigatorContextMenu from "../components/navigator/NavigatorContextMenu.vue";
import PreviewPane from "../components/navigator/PreviewPane.vue";
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
const previewPaneSetting = ref(false);

const previewPaneEnabled = computed(
  () => previewPaneSetting.value || previewBridgeForcePane.value,
);

const previewPath = computed(() => {
  const paths = selectedPaths.value;
  if (paths.length === 1) return paths[0] ?? null;
  return null;
});

async function refreshPreviewSetting() {
  try {
    const doc = await hydrateSettings();
    previewPaneSetting.value = doc.values["experimental.previewPane"] === true;
  } catch {
    previewPaneSetting.value = false;
  }
}

async function shortcutPreviewBridge() {
  if (isGuidePath(currentPath.value)) return;
  const paths = selectedPaths.value;
  if (!paths.length) return;
  try {
    const result = await invokePreviewBridge(paths);
    if (result.backend === "off") return;
    if (!result.success && result.backend === "none") {
      toastInfo(result.error ?? t("previewBridge.noPath"));
      return;
    }
    if (result.fallbackBuiltin && result.error) {
      toastInfo(t("previewBridge.fallbackBuiltin"));
    }
  } catch (err) {
    toastError(err instanceof Error ? err.message : String(err));
  }
}

/** AIA-174: while Space preview session is open, push selection changes. */
let followTimer: ReturnType<typeof setTimeout> | null = null;
let followSeq = 0;
watch(
  selectedPaths,
  (paths) => {
    if (!previewBridgeSessionActive.value) return;
    if (isGuidePath(currentPath.value)) return;
    if (!paths.length) return;
    if (followTimer) clearTimeout(followTimer);
    // Debounce Peek kill/respawn under arrow-key scrubbing; QL Switch is cheap.
    followTimer = setTimeout(() => {
      followTimer = null;
      const my = ++followSeq;
      const snapshot = [...paths];
      void (async () => {
        try {
          await switchPreviewBridge(snapshot);
          if (my !== followSeq) return;
        } catch {
          // Selection follow is best-effort; avoid toast spam while scrubbing.
        }
      })();
    }, 120);
  },
  { deep: true },
);

function openProperties(path: string) {
  propertiesPath.value = path;
  propertiesOpen.value = true;
}

function closeProperties() {
  propertiesOpen.value = false;
  propertiesPath.value = null;
}

const listRoot = ref<HTMLElement | null>(null);
const secondaryListRoot = ref<HTMLElement | null>(null);
const marquee = ref<{ x: number; y: number; w: number; h: number } | null>(null);
let marqueeOrigin: { x: number; y: number } | null = null;
let marqueeActive = false;
const dragOverPath = ref<string | null>(null);

/** Windows Explorer–style inline rename (no modal). */
const renamingPath = ref<string | null>(null);
const renameDraft = ref("");
const renameInputEl = ref<HTMLTextAreaElement | null>(null);
/** Delay before name-click rename so double-click can still open. */
const NAME_RENAME_DELAY_MS = 500;
let pendingNameRenameTimer: ReturnType<typeof setTimeout> | null = null;
let pendingNameRenamePath: string | null = null;

function setRenameInputEl(el: unknown) {
  renameInputEl.value = el instanceof HTMLTextAreaElement ? el : null;
}

function clearPendingNameRename() {
  if (pendingNameRenameTimer != null) {
    clearTimeout(pendingNameRenameTimer);
    pendingNameRenameTimer = null;
  }
  pendingNameRenamePath = null;
}

/**
 * Match rename box height to how many lines the name wraps in the list
 * (icon views clamp at 2; list/details stay single-line). Never pad empty lines.
 */
function syncRenameInputSize() {
  const input = renameInputEl.value;
  if (!input) return;
  const maxLines = isIconView.value ? 2 : 1;
  input.rows = 1;
  input.style.height = "auto";
  const style = getComputedStyle(input);
  const lineHeight = Number.parseFloat(style.lineHeight) || Number.parseFloat(style.fontSize) * 1.25 || 16;
  const padY =
    (Number.parseFloat(style.paddingTop) || 0) + (Number.parseFloat(style.paddingBottom) || 0);
  const borderY =
    (Number.parseFloat(style.borderTopWidth) || 0) +
    (Number.parseFloat(style.borderBottomWidth) || 0);
  const maxHeight = lineHeight * maxLines + padY + borderY;
  const next = Math.min(input.scrollHeight, maxHeight);
  input.style.height = `${Math.max(next, lineHeight + padY + borderY)}px`;
}

/** Select basename for files (keep extension); full name for folders. */
function focusRenameInput(name: string, kind: NavigatorEntry["kind"]) {
  const input = renameInputEl.value;
  if (!input) return;
  syncRenameInputSize();
  input.focus();
  if (kind === "dir") {
    input.select();
    return;
  }
  const dot = name.lastIndexOf(".");
  if (dot > 0) {
    input.setSelectionRange(0, dot);
  } else {
    input.select();
  }
}

function onRenameDraftInput() {
  // Filenames are single-line tokens; keep Enter for commit, never embed newlines.
  if (renameDraft.value.includes("\n") || renameDraft.value.includes("\r")) {
    renameDraft.value = renameDraft.value.replace(/[\r\n]+/g, "");
  }
  syncRenameInputSize();
}

function startInlineRename(entry: NavigatorEntry) {
  clearPendingNameRename();
  selectOnly(entry.path);
  renamingPath.value = entry.path;
  renameDraft.value = entry.name;
  void nextTick(() => {
    focusRenameInput(entry.name, entry.kind);
  });
}

function cancelInlineRename() {
  clearPendingNameRename();
  renamingPath.value = null;
  renameDraft.value = "";
}

function onRenameBlur() {
  // Defer so Enter-commit can clear `renamingPath` first in the same turn.
  queueMicrotask(() => {
    if (renamingPath.value) cancelInlineRename();
  });
}

async function commitInlineRename() {
  const path = renamingPath.value;
  if (!path) return;
  const entry = visibleEntries.value.find((e) => e.path === path) ?? null;
  const next = renameDraft.value.trim();
  renamingPath.value = null;
  renameDraft.value = "";
  if (!entry || !next || next === entry.name) return;
  try {
    const result = await renameEntry(entry.path, next);
    if (!result.ok) {
      onNotice(result.error ?? t("contextMenu.opFailed"), "error");
      return;
    }
    onNotice(t("contextMenu.renamed"));
    await refreshListing();
    const renamed = result.paths[0];
    if (renamed) selectOnly(renamed);
  } catch (err) {
    onNotice(err instanceof Error ? err.message : String(err), "error");
  }
}

/**
 * Explorer-like: click the name of an already-selected sole item → rename
 * (delayed so dblclick still opens).
 */
function onNameClick(event: MouseEvent, entry: NavigatorEntry) {
  if (event.shiftKey || event.ctrlKey || event.metaKey) return;
  if (renamingPath.value === entry.path) {
    event.stopPropagation();
    return;
  }
  const soleSelected =
    isSelected(entry.path) &&
    selectedPaths.value.length === 1 &&
    selectedPaths.value[0] === entry.path;
  if (!soleSelected) return;
  event.stopPropagation();
  clearPendingNameRename();
  pendingNameRenamePath = entry.path;
  pendingNameRenameTimer = setTimeout(() => {
    pendingNameRenameTimer = null;
    if (pendingNameRenamePath !== entry.path) return;
    pendingNameRenamePath = null;
    if (
      isSelected(entry.path) &&
      selectedPaths.value.length === 1 &&
      selectedPaths.value[0] === entry.path
    ) {
      startInlineRename(entry);
    }
  }, NAME_RENAME_DELAY_MS);
}

function onRequestRename(path: string) {
  const entry = visibleEntries.value.find((e) => e.path === path);
  if (entry) startInlineRename(entry);
}

const folderSearchActive = computed(() => navigatorFilterQuery.value.trim().length > 0);

const visibleEntries = computed(() => {
  if (folderSearchActive.value) {
    return sortedEntries(primarySearch.value?.entries ?? []);
  }
  if (!listing.value) return [];
  return sortedEntries(listing.value.entries);
});

const secondaryEntries = computed(() => {
  if (folderSearchActive.value) {
    return sortedEntries(secondarySearch.value?.entries ?? []);
  }
  if (!secondaryListing.value) return [];
  return sortedEntries(secondaryListing.value.entries);
});

watch([navigatorFilterQuery, currentPath], () => {
  scheduleFolderSearch("primary", currentPath.value);
});

watch([navigatorFilterQuery, secondaryPath, dualPaneEnabled], () => {
  if (!dualPaneEnabled.value) return;
  scheduleFolderSearch("secondary", secondaryPath.value);
});

function entryLocation(path: string): string {
  const norm = path.replace(/\//g, "\\");
  const idx = norm.lastIndexOf("\\");
  return idx > 0 ? norm.slice(0, idx) : norm;
}

const orderedPaths = computed(() => visibleEntries.value.map((e) => e.path));

function requestIconsFor(paths: string[]) {
  const size = VIEW_ICON_SIZE[navigatorViewMode.value];
  // Icon views: prefer system thumbnails for images; list/details use small shell icons.
  requestShellIcons(paths, size, isIconView.value);
}

/** Only request shell icons / thumbnails for rows in (or near) the scroll viewport (AIA-182). */
let primaryIconObserver: IntersectionObserver | null = null;
let secondaryIconObserver: IntersectionObserver | null = null;

function disconnectIconObservers() {
  primaryIconObserver?.disconnect();
  primaryIconObserver = null;
  secondaryIconObserver?.disconnect();
  secondaryIconObserver = null;
}

function bindIconObserver(
  root: HTMLElement | null,
  previous: IntersectionObserver | null,
): IntersectionObserver | null {
  previous?.disconnect();
  if (!root) return null;
  const observer = new IntersectionObserver(
    (entries) => {
      const paths: string[] = [];
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        const path = (entry.target as HTMLElement).dataset.entryPath;
        if (path) paths.push(path);
      }
      if (paths.length) requestIconsFor(paths);
    },
    // Prefetch a little past the fold so scroll feels warm without loading the whole listing.
    { root, rootMargin: "120px 0px", threshold: 0 },
  );
  for (const el of root.querySelectorAll<HTMLElement>("[data-entry-path]")) {
    observer.observe(el);
  }
  return observer;
}

async function refreshViewportIconObservers() {
  await nextTick();
  primaryIconObserver = bindIconObserver(listRoot.value, primaryIconObserver);
  secondaryIconObserver = bindIconObserver(
    secondaryListRoot.value,
    secondaryIconObserver,
  );
}

watch(
  [visibleEntries, secondaryEntries, navigatorViewMode, listRoot, secondaryListRoot],
  () => {
    void refreshViewportIconObservers();
  },
  { immediate: true },
);

function onWindowFocus() {
  void refreshPreviewSetting();
  void loadNavigatorViewMode();
  void refreshClipboardFromSystem();
}

onMounted(() => {
  registerNavFileShortcuts();
  window.addEventListener("mousemove", onMarqueeMove);
  window.addEventListener("mouseup", onMarqueeEnd);
  window.addEventListener("focus", onWindowFocus);
  void (async () => {
    await refreshPreviewSetting();
    await loadNavigatorViewMode();
    await bootstrapNavigatorOpenState();
    await loadFavorites().catch(() => undefined);
    await loadFavoritesOsHookStatus().catch(() => undefined);
    if (
      currentPath.value &&
      !isGuidePath(currentPath.value) &&
      !listing.value &&
      !listError.value &&
      !listLoading.value
    ) {
      await navigateTo(currentPath.value, { syncOpenState: false });
    }
  })();
});

onUnmounted(() => {
  clearPendingNameRename();
  disconnectIconObservers();
  unregisterNavFileShortcuts();
  window.removeEventListener("mousemove", onMarqueeMove);
  window.removeEventListener("mouseup", onMarqueeEnd);
  window.removeEventListener("focus", onWindowFocus);
});

watch(currentPath, () => {
  if (renamingPath.value) cancelInlineRename();
});

function formatSize(entry: NavigatorEntry): string {
  if (entry.kind === "dir") return "—";
  if (entry.size < 1024) return `${entry.size} B`;
  if (entry.size < 1024 * 1024) return `${(entry.size / 1024).toFixed(1)} KB`;
  if (entry.size < 1024 * 1024 * 1024) return `${(entry.size / (1024 * 1024)).toFixed(1)} MB`;
  return `${(entry.size / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatModified(entry: NavigatorEntry): string {
  const ms = entry.modifiedMs;
  if (ms == null || !Number.isFinite(ms)) return "—";
  try {
    return new Date(ms).toLocaleString(undefined, {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return "—";
  }
}

/** Native hover tip (Win Explorer–like): kind + size (files) + locale modified time. */
function entryHoverTitle(entry: NavigatorEntry): string {
  const lines: string[] = [
    entry.kind === "dir" ? t("navigator.folder") : t("navigator.file"),
  ];
  // Folders: omit size (no listing child-count field today). Files: human size.
  if (entry.kind !== "dir") {
    lines.push(`${t("navigator.colSize")}: ${formatSize(entry)}`);
  }
  const modified = formatModified(entry);
  if (modified !== "—") {
    lines.push(`${t("navigator.colModified")}: ${modified}`);
  }
  return lines.join("\n");
}

function sortIndicator(column: "name" | "size" | "modified"): string {
  if (sortColumn.value !== column) return "";
  return sortDirection.value === "asc" ? " ▲" : " ▼";
}

const iconPixelSize = computed(() => VIEW_ICON_SIZE[navigatorViewMode.value]);

function shellIcon(path: string): string | null {
  return iconUrlFor(path, iconPixelSize.value);
}

function onActivate(entry: NavigatorEntry) {
  clearPendingNameRename();
  if (renamingPath.value) cancelInlineRename();
  // Primary list always uses openEntry; secondary pane has its own handler.
  void openEntry(entry).catch((err) => {
    toastError(err instanceof Error ? err.message : String(err));
  });
}

function onActivateSecondary(entry: NavigatorEntry) {
  setActivePane("secondary");
  if (entry.kind === "dir") {
    void navigateSecondary(entry.path);
    return;
  }
  void openEntry(entry).catch((err) => {
    toastError(err instanceof Error ? err.message : String(err));
  });
}

function onRowClick(event: MouseEvent, entry: NavigatorEntry) {
  // Clicking away from a pending name-rename (or another row) cancels the delay.
  if (pendingNameRenamePath && pendingNameRenamePath !== entry.path) {
    clearPendingNameRename();
  }
  if (event.shiftKey) {
    clearPendingNameRename();
    selectRange(orderedPaths.value, selectionAnchor.value, entry.path);
    return;
  }
  if (event.ctrlKey || event.metaKey) {
    clearPendingNameRename();
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
  openFolderMenu(entry, event.clientX, event.clientY);
}

function openFolderMenu(entry: NavigatorEntry, x: number, y: number) {
  menuEntry.value = entry;
  menuEmpty.value = false;
  menuX.value = x;
  menuY.value = y;
  menuOpen.value = true;
}

function onListContextMenu(event: MouseEvent) {
  // Background / empty listing: same folder verbs as the current directory entry.
  if (event.target !== event.currentTarget) return;
  event.preventDefault();
  const path = currentPath.value;
  if (!path || isGuidePath(path)) return;
  openFolderMenu(dirEntryFromPath(path), event.clientX, event.clientY);
}

function closeMenu() {
  menuOpen.value = false;
  menuEmpty.value = false;
}

watch(folderContextMenuRequest, (req) => {
  if (!req) return;
  openFolderMenu(req.entry, req.x, req.y);
});

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

async function shortcutCopyPath() {
  const entry = primaryEntry();
  if (!entry) return;
  try {
    await navigator.clipboard.writeText(entry.path);
    onNotice(t("contextMenu.pathCopied"));
  } catch (err) {
    onNotice(err instanceof Error ? err.message : String(err));
  }
}

function shortcutRename() {
  const entry = primaryEntry();
  if (!entry) return;
  startInlineRename(entry);
}

async function shortcutNewFolder() {
  if (!currentPath.value) return;
  const suggested = await resolveNewFolderDefaultName(t("contextMenu.newFolderDefault"));
  const prompted = await openPromptDialog({
    title: t("contextMenu.newFolder"),
    label: t("contextMenu.newFolderPrompt"),
    defaultValue: suggested,
    confirmLabel: t("common.create"),
  });
  if (!prompted.confirmed) return;
  const trimmed = prompted.value.trim();
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
  const prompted = await openPromptDialog({
    title: t("contextMenu.newFile"),
    label: t("contextMenu.newFilePrompt"),
    defaultValue: suggested,
    confirmLabel: t("common.create"),
  });
  if (!prompted.confirmed) return;
  const trimmed = prompted.value.trim();
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

async function shortcutDelete(permanent = false) {
  const paths = selectionPaths();
  if (!paths.length) return;
  try {
    const blocked = permanent ? paths : await pathsWithoutRecycle(paths);
    const label =
      paths.length === 1
        ? visibleEntries.value.find((e) => e.path === paths[0])?.name ?? paths[0]
        : t("navigator.nItems", { count: paths.length });
    if (permanent || blocked.length) {
      const ok = await confirmPermanentDelete({
        title: t("dialog.deleteTitle"),
        message: t("contextMenu.deletePermanentConfirm", { name: label }),
        confirmLabel: t("common.delete"),
        cancelLabel: t("common.cancel"),
      });
      if (!ok) return;
      const recyclable = permanent ? [] : paths.filter((path) => !blocked.includes(path));
      const doomed = permanent ? paths : blocked;
      if (recyclable.length) {
        const recycled = await deleteEntries(recyclable, false);
        if (!recycled.ok) {
          onNotice(recycled.error ?? t("contextMenu.opFailed"));
          return;
        }
      }
      const result = await deleteEntries(doomed, true);
      if (!result.ok) {
        onNotice(result.error ?? t("contextMenu.opFailed"));
        return;
      }
      onNotice(t("contextMenu.deletedPermanent"));
      clearSelection();
      clearClipboard();
      await refreshListing();
      return;
    }
    const ok = await confirmDeleteIfNeeded({
      title: t("dialog.deleteTitle"),
      message: t("contextMenu.deleteConfirm", { name: label }),
      confirmLabel: t("common.delete"),
      cancelLabel: t("common.cancel"),
    });
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
    shortcutRename();
  });
  registerShortcutHandler("navigator.copyPath", () => {
    void shortcutCopyPath();
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
    void shortcutDelete(false);
  });
  registerShortcutHandler("navigator.deleteKey", () => {
    void shortcutDelete(false);
  });
  registerShortcutHandler("navigator.deletePermanent", () => {
    void shortcutDelete(true);
  });
  registerShortcutHandler("navigator.selectAll", () => {
    setSelection(orderedPaths.value);
  });
  registerShortcutHandler("navigator.previewBridge", () => {
    void shortcutPreviewBridge();
  });
}

function unregisterNavFileShortcuts() {
  for (const id of [
    "navigator.copy",
    "navigator.cut",
    "navigator.paste",
    "navigator.rename",
    "navigator.copyPath",
    "navigator.newFolder",
    "navigator.newFile",
    "navigator.refresh",
    "navigator.delete",
    "navigator.deleteKey",
    "navigator.deletePermanent",
    "navigator.selectAll",
    "navigator.previewBridge",
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

/* —— Drag into folder (HTML5) + drag out to external apps (OLE / CF_HDROP) —— */
function onDragStart(event: DragEvent, entry: NavigatorEntry) {
  if (!isSelected(entry.path)) {
    selectOnly(entry.path);
  }
  const paths = selectionPaths();
  event.dataTransfer?.setData("application/x-afm-paths", JSON.stringify(paths));
  // copyMove: modifiers and volume pick copy vs move; OLE drag-out stays copy.
  event.dataTransfer!.effectAllowed = "copyMove";
  armNativeFileDragOut(paths);
}

function onDragOver(event: DragEvent, entry: NavigatorEntry) {
  if (entry.kind !== "dir") return;
  if (isSelected(entry.path)) return;
  event.preventDefault();
  const mode = dropTransferMode(selectionPaths(), entry.path, {
    ctrl: event.ctrlKey,
    shift: event.shiftKey,
  });
  event.dataTransfer!.dropEffect = mode;
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
    const mode = dropTransferMode(paths, entry.path, {
      ctrl: event.ctrlKey,
      shift: event.shiftKey,
    });
    const result = await transferEntries(paths, entry.path, mode);
    if (!result.ok) {
      onNotice(result.error ?? t("contextMenu.opFailed"));
      return;
    }
    const noticeKey = mode === "copy" ? "navigator.copiedInto" : "navigator.movedInto";
    onNotice(t(noticeKey, { count: result.paths.length, name: entry.name }));
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
    <div class="navigator-body" :class="{ 'navigator-body--preview': previewPaneEnabled }">
    <div class="panes" :class="{ 'panes--dual': dualPaneEnabled }">
    <div
      class="pane"
      :class="{ 'pane--active': activePane === 'primary' || !dualPaneEnabled }"
      @mousedown="setActivePane('primary')"
    >
    <NavigatorGuide
      v-if="isGuidePath(currentPath)"
      :open-path="(path) => navigateTo(path)"
    />
    <template v-else>
    <p class="pane-path" :title="currentPath">{{ currentPath || t("navigator.noPath") }}</p>
    <p v-if="listLoading" class="fluent-banner">{{ t("navigator.loading") }}</p>
    <p v-else-if="listError" class="fluent-banner fluent-banner--error">{{ listError }}</p>
    <p v-else-if="folderSearchActive && primarySearchLoading" class="fluent-banner">
      {{ t("navigator.filterSearching") }}
    </p>
    <p
      v-else-if="folderSearchActive && !primarySearchLoading && !visibleEntries.length"
      class="fluent-banner fluent-banner--info"
    >
      {{ t("navigator.filterEmpty") }}
    </p>
    <p v-else-if="listing && listing.entries.length === 0" class="fluent-banner">
      {{ t("navigator.empty") }}
    </p>
    <p
      v-if="folderSearchActive && primarySearch?.truncated"
      class="fluent-banner fluent-banner--info"
    >
      {{ t("navigator.filterTruncated", { count: visibleEntries.length }) }}
    </p>

    <div
      v-if="listing"
      ref="listRoot"
      class="fluent-surface list"
      :class="{
        'list--details': isDetailsView,
        'list--list': isListView,
        'list--icons': isIconView,
        'list--icons-large': navigatorViewMode === 'iconsLarge',
        'list--icons-medium': navigatorViewMode === 'iconsMedium',
        'list--icons-small': navigatorViewMode === 'iconsSmall',
      }"
      role="list"
      @contextmenu="onListContextMenu"
      @mousedown="onListMouseDown"
      @click.self="clearSelection()"
    >
      <div v-if="isDetailsView" class="list-header" role="row">
        <button type="button" class="col-sort col-name" @click="toggleSort('name')">
          {{ t("navigator.colName") }}{{ sortIndicator("name") }}
        </button>
        <button type="button" class="col-sort col-size" @click="toggleSort('size')">
          {{ t("navigator.colSize") }}{{ sortIndicator("size") }}
        </button>
        <button type="button" class="col-sort col-modified" @click="toggleSort('modified')">
          {{ t("navigator.colModified") }}{{ sortIndicator("modified") }}
        </button>
      </div>

      <div
        v-for="entry in visibleEntries"
        :key="entry.path"
        class="fluent-list-row fluent-list-row--density row"
        :class="{
          'row--icon': isIconView,
          'row--selected': isSelected(entry.path),
          'row--drop': dragOverPath === entry.path,
        }"
        role="listitem"
        tabindex="0"
        draggable="true"
        :data-entry-path="entry.path"
        :aria-selected="isSelected(entry.path)"
        :title="entryHoverTitle(entry)"
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
        >
          <img
            v-if="shellIcon(entry.path)"
            class="shell-icon"
            :src="shellIcon(entry.path)!"
            alt=""
            :width="iconPixelSize"
            :height="iconPixelSize"
          />
          <FluentIcon v-else :name="entryKindIcon(entry.kind === 'dir')" size="md" />
        </span>
        <textarea
          v-if="renamingPath === entry.path"
          :ref="setRenameInputEl"
          v-model="renameDraft"
          class="fluent-control inline-rename"
          rows="1"
          wrap="soft"
          spellcheck="false"
          :aria-label="t('contextMenu.rename')"
          @click.stop
          @dblclick.stop
          @input="onRenameDraftInput"
          @keydown.enter.prevent.stop="commitInlineRename"
          @keydown.escape.prevent.stop="cancelInlineRename"
          @blur="onRenameBlur"
        />
        <span v-else class="name-block">
          <span class="name" @click="onNameClick($event, entry)">{{ entry.name }}</span>
          <span v-if="folderSearchActive" class="row-location">{{ entryLocation(entry.path) }}</span>
        </span>
        <span v-if="isDetailsView" class="size">{{ formatSize(entry) }}</span>
        <span v-if="isDetailsView" class="modified">{{ formatModified(entry) }}</span>
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
    </template>
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
      <div
        v-else-if="secondaryListing"
        ref="secondaryListRoot"
        class="fluent-surface list"
        :class="{
          'list--details': isDetailsView,
          'list--list': isListView,
          'list--icons': isIconView,
          'list--icons-large': navigatorViewMode === 'iconsLarge',
          'list--icons-medium': navigatorViewMode === 'iconsMedium',
          'list--icons-small': navigatorViewMode === 'iconsSmall',
        }"
        role="list"
      >
        <div
          v-for="entry in secondaryEntries"
          :key="entry.path"
          class="fluent-list-row fluent-list-row--density row"
          :class="{ 'row--icon': isIconView }"
          role="listitem"
          tabindex="0"
          :data-entry-path="entry.path"
          :title="entryHoverTitle(entry)"
          @dblclick="onActivateSecondary(entry)"
          @keydown.enter.prevent="onActivateSecondary(entry)"
        >
          <span class="kind" :data-kind="entry.kind">
            <img
              v-if="shellIcon(entry.path)"
              class="shell-icon"
              :src="shellIcon(entry.path)!"
              alt=""
              :width="iconPixelSize"
              :height="iconPixelSize"
            />
            <FluentIcon v-else :name="entryKindIcon(entry.kind === 'dir')" size="md" />
          </span>
          <span class="name-block">
            <span class="name">{{ entry.name }}</span>
            <span v-if="folderSearchActive" class="row-location">{{ entryLocation(entry.path) }}</span>
          </span>
          <span v-if="isDetailsView" class="size">{{ formatSize(entry) }}</span>
          <span v-if="isDetailsView" class="modified">{{ formatModified(entry) }}</span>
        </div>
      </div>
    </div>
    </div>
    <PreviewPane :enabled="previewPaneEnabled" :path="previewPath" />
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
      @request-rename="onRequestRename"
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

.navigator-body {
  display: flex;
  flex: 1;
  min-height: 0;
  min-width: 0;
  gap: 0;
}

.navigator-body--preview {
  gap: 0;
}

.panes {
  display: flex;
  flex: 1;
  min-height: 0;
  min-width: 0;
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

.list--list .row {
  grid-template-columns: 2rem 1fr;
}

.list--details .list-header,
.list--details .row {
  grid-template-columns: 2rem minmax(0, 1fr) 5.5rem 9.5rem;
}

.list--icons {
  display: grid;
  align-content: start;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-sm);
}

.list--icons-large {
  grid-template-columns: repeat(auto-fill, minmax(7.5rem, 1fr));
}

.list--icons-medium {
  grid-template-columns: repeat(auto-fill, minmax(6rem, 1fr));
}

.list--icons-small {
  grid-template-columns: repeat(auto-fill, minmax(5rem, 1fr));
}

.list--icons .row--icon {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-start;
  gap: var(--fluent-space-xs);
  min-height: auto;
  padding: var(--fluent-space-sm);
  border-bottom: 0;
  border-radius: var(--fluent-radius-md);
  text-align: center;
}

.list--icons .row--icon .name {
  width: 100%;
  white-space: normal;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  line-height: 1.25;
  font-size: var(--fluent-font-size-caption);
}

.list--icons .row--icon .inline-rename {
  width: 100%;
  min-width: 0;
  text-align: center;
  font-size: var(--fluent-font-size-caption);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  line-height: 1.25;
  /* Height-capped wrap: scroll so drag-select can reach every line. */
  overflow-x: hidden;
  overflow-y: auto;
}

.list--icons .shell-icon {
  width: auto;
  height: auto;
  max-width: 100%;
}

.list-header {
  display: grid;
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

.col-modified {
  grid-column: 4;
  text-align: left;
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

.name-block {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.row-location {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.75rem;
  opacity: 0.72;
}

.inline-rename {
  /* Slightly wider than a collapsed name cell; still fills the column. */
  min-width: 10rem;
  width: 100%;
  min-height: 0;
  height: auto;
  box-sizing: border-box;
  padding: 1px var(--fluent-space-xs);
  font: inherit;
  line-height: inherit;
  resize: none;
  /*
   * Overflow must scroll (not clip): mouse-drag selection auto-scrolls through
   * the field so every character is reachable. Hide scrollbars like Explorer.
   */
  overflow-x: auto;
  overflow-y: hidden;
  white-space: nowrap;
  field-sizing: fixed;
  user-select: text;
  scrollbar-width: none;
}

.inline-rename::-webkit-scrollbar {
  display: none;
}

.size {
  text-align: right;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  font-variant-numeric: tabular-nums;
}

.modified {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  font-variant-numeric: tabular-nums;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.marquee {
  position: absolute;
  pointer-events: none;
  border: 1px solid var(--fluent-color-stroke-focus);
  background: color-mix(in srgb, var(--fluent-color-fill-accent) 18%, transparent);
  z-index: 2;
}
</style>
