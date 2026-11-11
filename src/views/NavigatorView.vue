<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  bootstrapNavigatorOpenState,
  closeNavigatorTab,
  currentPath,
  listing,
  listError,
  listLoading,
  liveOpenState,
  navigateTo,
  openEntry,
  refreshActiveListing,
  switchNavigatorTab,
  type NavigatorEntry,
} from "../state/navigatorBrowse";
import {
  dirEntryFromPath,
  folderContextMenuRequest,
} from "../state/folderContextMenu";
import { isGuidePath } from "../state/guide";
import { isRecyclePath } from "../state/recycle";
import { isProviderPath } from "../state/provider";
import { restoreRecyclePaths } from "../state/recycle";
import NavigatorGuide from "../components/navigator/NavigatorGuide.vue";
import { hydrateSettings } from "../state/settingsHydrate";
import {
  cancelShellIconRequests,
  clearShellIconQueue,
  requestShellIcons,
} from "../state/shellIcons";
import { loadFavorites, loadFavoritesOsHookStatus } from "../state/favorites";
import { scheduleAfterPaint, scheduleIdle } from "../state/scheduleIdle";
import { startupMark } from "../state/startupTiming";
import {
  clearClipboard,
  clipboardMode,
  clipboardPaths,
  createFile,
  createFolder,
  deleteEntries,
  hasClipboard,
  isClipboardCutPath,
  pasteClipboard,
  pathsWithoutRecycle,
  refreshClipboardFromSystem,
  renameEntry,
  setClipboard,
} from "../state/fileClipboard";
import {
  recordCopyUndo,
  recordCreateUndo,
  recordDeleteUndo,
  recordMoveUndo,
  recordRenameUndo,
  undoLast,
} from "../state/fileUndo";
import { planMultiRename } from "../state/multiRename";
import { loadNewFileTemplates } from "../state/newFileTemplates";
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
import {
  arrowDirectionFromKey,
  estimateIconColumns,
  moveSelectionByArrow,
} from "../state/navigatorKeyboardNav";
import {
  DETAILS_HEADER_HEIGHT_PX,
  estimatePlaceholderCount,
  ICON_CAPTION_EXTRA_PX,
  iconSlotCssSize,
  type PlaceholderViewMode,
} from "../state/listPlaceholders";
import {
  computeListVirtualWindow,
  computeThumbRequestWindow,
  computeVariableListVirtualWindow,
  estimateListColumns,
  resolveListRowHeightPx,
  type ListVirtualWindow,
} from "../state/listVirtualWindow";
import {
  flattenEntryItems,
  flattenGroupedItems,
  virtItemHeightPx,
  type ListVirtItem,
} from "../state/listVirtItems";
import { sortedEntries, sortColumn, sortDirection, toggleSort } from "../state/navigatorSort";
import {
  isContentView,
  isDetailsView,
  isIconView,
  isListView,
  isTilesView,
  loadNavigatorViewMode,
  navigatorGroupByType,
  navigatorViewMode,
  nudgeIconViewMode,
  setNavigatorViewMode,
  VIEW_ICON_SIZE,
} from "../state/navigatorViewMode";
import { entryTypeLabel } from "../state/entryType";
import {
  DEFAULT_DETAILS_COLUMN_WIDTHS,
  detailsGridTemplate,
  resizeColumnWidth,
  type DetailsColumnId,
  type DetailsColumnWidths,
} from "../state/navigatorColumns";
import { isEditableTarget } from "../shortcuts/chord.ts";
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
import {
  acceptFolderDragOver,
  armNativeFileDragOut,
  commitFolderDrop,
  noticeKeyForDropMode,
  previewDropMode,
  previewSourcePaths,
  setOutboundPathPayload,
} from "../state/dnd";
import {
  dismissPreviewBridge,
  invokePreviewBridge,
  previewBridgeForcePane,
  previewBridgeSessionActive,
  switchPreviewBridge,
} from "../state/previewBridge";
import {
  registerShortcutHandler,
  unregisterShortcutHandler,
} from "../shortcuts/index.ts";
import NavigatorContextMenu from "../components/navigator/NavigatorContextMenu.vue";
import PreviewPane from "../components/navigator/PreviewPane.vue";
import DetailsPane from "../components/navigator/DetailsPane.vue";
import PropertiesPanel from "../components/navigator/PropertiesPanel.vue";
import ShellIconImg from "../components/navigator/ShellIconImg.vue";

/** KeepAlive include name in App.vue (AIA-276). */
defineOptions({ name: "NavigatorView" });

const { t } = useI18n();

const menuOpen = ref(false);
const menuEntry = ref<NavigatorEntry | null>(null);
const menuEmpty = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const propertiesOpen = ref(false);
const propertiesPath = ref<string | null>(null);
const previewPaneSetting = ref(false);
/** Session override for Alt+P. Null follows experimental.previewPane. */
const previewPaneSession = ref<boolean | null>(null);
/** AIA-235: Alt+Shift+P details pane (separate from Alt+Enter properties). */
const detailsPaneOpen = ref(false);

/**
 * 引导页与目录列表互斥展示，但各自首次出现后用 v-show 保活，
 * 避免从 afm://guide 切回目录时整表 DOM 卸载/重挂造成关标签卡顿。
 */
const guideSurfaceMounted = ref(false);
const listSurfaceMounted = ref(false);
const showingGuide = computed(() => isGuidePath(currentPath.value));
watch(
  currentPath,
  (path) => {
    if (isGuidePath(path)) guideSurfaceMounted.value = true;
    else if (path.trim()) listSurfaceMounted.value = true;
  },
  { immediate: true },
);

const previewPaneEnabled = computed(() => {
  const base = previewPaneSession.value ?? previewPaneSetting.value;
  return base || previewBridgeForcePane.value;
});

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

function shortcutProperties() {
  if (isGuidePath(currentPath.value) && !selectedPaths.value.length) return;
  const selected = selectedPaths.value.find((path) => path && !isGuidePath(path));
  const path = selected || (isGuidePath(currentPath.value) ? "" : currentPath.value);
  if (!path) return;
  openProperties(path);
}

function openProperties(path: string) {
  propertiesPath.value = path;
  propertiesOpen.value = true;
}

function shortcutTogglePreviewPane() {
  const currently = previewPaneSession.value ?? previewPaneSetting.value;
  previewPaneSession.value = !currently;
}

/**
 * AIA-244: × on PreviewPane. Hide the pane and end any Space bridge session so
 * selection changes alone do not force it open again.
 */
function closePreviewPane() {
  previewPaneSession.value = false;
  if (followTimer) {
    clearTimeout(followTimer);
    followTimer = null;
  }
  followSeq += 1;
  void dismissPreviewBridge();
}

function shortcutToggleDetailsPane() {
  detailsPaneOpen.value = !detailsPaneOpen.value;
}

function closeDetailsPane() {
  detailsPaneOpen.value = false;
}

function closeProperties() {
  propertiesOpen.value = false;
  propertiesPath.value = null;
}

const listRoot = ref<HTMLElement | null>(null);
const secondaryListRoot = ref<HTMLElement | null>(null);
/** AIA-267: measure list surface so loading placeholders match the viewport. */
const listViewport = ref({ w: 0, h: 0 });
const secondaryViewport = ref({ w: 0, h: 0 });
/** 大目录窗口化：跟踪滚动位置以切片挂载行。 */
const primaryScrollTop = ref(0);
const secondaryScrollTop = ref(0);
let listSizeObserver: ResizeObserver | null = null;
let secondarySizeObserver: ResizeObserver | null = null;
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
/** 改名期间监听框外 pointerdown；仅点击/快捷键结束，鼠标移出不退出。 */
let renameOutsideListening = false;

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

function armRenameOutsideListener() {
  if (renameOutsideListening) return;
  renameOutsideListening = true;
  window.addEventListener("pointerdown", onRenameOutsidePointerDown, true);
}

function disarmRenameOutsideListener() {
  if (!renameOutsideListening) return;
  renameOutsideListening = false;
  window.removeEventListener("pointerdown", onRenameOutsidePointerDown, true);
}

/**
 * 资源管理器：在改名框外按下才结束改名（提交）；鼠标移出或拖选松开不结束。
 */
function onRenameOutsidePointerDown(event: PointerEvent) {
  if (!renamingPath.value) return;
  const input = renameInputEl.value;
  const target = event.target;
  if (!(target instanceof Node)) return;
  if (input && (target === input || input.contains(target))) return;
  void commitInlineRename();
}

/**
 * 改名框紧贴文字：高度随 wrap 行数（图标最多 2 行）；宽度跟内容。
 * 图标视图下最小与图标同宽，短文本不收缩；更长文本仍可长到格子上限。
 */
function syncRenameInputSize() {
  const input = renameInputEl.value;
  if (!input) return;
  const maxLines = isIconView.value ? 3 : 1;
  input.rows = 1;
  input.style.height = "auto";
  input.style.width = "auto";
  const style = getComputedStyle(input);
  const lineHeight = Number.parseFloat(style.lineHeight) || Number.parseFloat(style.fontSize) * 1.25 || 16;
  const padY =
    (Number.parseFloat(style.paddingTop) || 0) + (Number.parseFloat(style.paddingBottom) || 0);
  const borderY =
    (Number.parseFloat(style.borderTopWidth) || 0) +
    (Number.parseFloat(style.borderBottomWidth) || 0);
  const padX =
    (Number.parseFloat(style.paddingLeft) || 0) + (Number.parseFloat(style.paddingRight) || 0);
  const borderX =
    (Number.parseFloat(style.borderLeftWidth) || 0) +
    (Number.parseFloat(style.borderRightWidth) || 0);
  const parent = input.parentElement;
  const maxWidth = parent?.clientWidth || input.clientWidth || 0;
  let iconMinWidth = 0;
  if (isIconView.value) {
    const kind = parent?.querySelector(".kind");
    if (kind instanceof HTMLElement) {
      iconMinWidth = Math.ceil(kind.getBoundingClientRect().width);
    }
    if (iconMinWidth <= 0) {
      iconMinWidth = Math.ceil(Number.parseFloat(style.getPropertyValue("--nav-icon-size")) || 0);
    }
  }
  const contentWidth = Math.ceil(input.scrollWidth + borderX);
  const fitted = Math.max(
    contentWidth,
    iconMinWidth,
    Math.ceil(lineHeight + padX + borderX),
  );
  const widthPx = maxWidth > 0 ? Math.min(fitted, maxWidth) : fitted;
  input.style.width = `${widthPx}px`;
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
  armRenameOutsideListener();
  void nextTick(() => {
    focusRenameInput(entry.name, entry.kind);
  });
}

function cancelInlineRename() {
  clearPendingNameRename();
  disarmRenameOutsideListener();
  renamingPath.value = null;
  renameDraft.value = "";
}

/**
 * 短暂失焦（如拖选文字后在框外松开、行被 focus）不退出改名；拉回焦点。
 * 正式退出：Enter 提交 / Esc 取消 / 框外 pointerdown 提交。
 */
function onRenameBlur() {
  queueMicrotask(() => {
    if (!renamingPath.value) return;
    const input = renameInputEl.value;
    if (!input || document.activeElement === input) return;
    input.focus({ preventScroll: true });
  });
}

async function commitInlineRename() {
  const path = renamingPath.value;
  if (!path) return;
  disarmRenameOutsideListener();
  const entry = visibleEntries.value.find((e) => e.path === path) ?? null;
  const next = renameDraft.value.trim();
  renamingPath.value = null;
  renameDraft.value = "";
  if (!entry || !next) return;
  try {
    const selected = selectedPaths.value.includes(entry.path)
      ? orderedPaths.value.filter((path) => selectedPaths.value.includes(path))
      : [entry.path];
    if (selected.length > 1) {
      const plan = planMultiRename(selected, next);
      const renamedPaths: string[] = [];
      for (const item of plan) {
        if (item.newName === item.path.split(/[/\\]/).pop()) continue;
        const result = await renameEntry(item.path, item.newName);
        if (!result.ok) {
          onNotice(result.error ?? t("contextMenu.opFailed"), "error");
          await refreshListing();
          return;
        }
        if (result.paths[0]) {
          recordRenameUndo(item.path, result.paths[0]);
          renamedPaths.push(result.paths[0]);
        }
      }
      onNotice(t("contextMenu.renamedMulti", { count: renamedPaths.length || plan.length }));
      await refreshListing();
      if (renamedPaths.length) setSelection(renamedPaths);
      return;
    }
    if (next === entry.name) return;
    const result = await renameEntry(entry.path, next);
    if (!result.ok) {
      onNotice(result.error ?? t("contextMenu.opFailed"), "error");
      return;
    }
    const renamed = result.paths[0];
    if (renamed) recordRenameUndo(entry.path, renamed);
    onNotice(t("contextMenu.renamed"));
    await refreshListing();
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

function isCompactListDensity(): boolean {
  try {
    return document.documentElement.dataset.listDensity === "compact";
  } catch {
    return false;
  }
}

function listViewportHeightPx(viewport: { w: number; h: number }): number {
  const mode = navigatorViewMode.value as PlaceholderViewMode;
  if (mode === "details") {
    return Math.max(0, viewport.h - DETAILS_HEADER_HEIGHT_PX);
  }
  return viewport.h;
}

let lastListWindowProbeKey = "";

function probeListWindow(win: ListVirtualWindow, total: number, grouped: boolean): void {
  try {
    if (globalThis.localStorage?.getItem("afm.perfProbe") !== "1") return;
    if (!win.active) return;
    const mounted = win.endIndex - win.startIndex;
    const key = `${grouped ? 1 : 0}:${total}:${mounted}:${win.startIndex}`;
    if (key === lastListWindowProbeKey) return;
    lastListWindowProbeKey = key;
    console.info(
      `[AFM_perf] list_window mounted=${mounted} total=${total} grouped=${grouped ? 1 : 0}`,
    );
  } catch {
    // ignore
  }
}

const primaryVirtItems = computed((): ListVirtItem[] => {
  const entries = visibleEntries.value;
  if (navigatorGroupByType.value) {
    return flattenGroupedItems(entries, typeLabels.value);
  }
  return flattenEntryItems(entries);
});

function inactiveVirtualWindow(total: number): ListVirtualWindow {
  return {
    active: false,
    startIndex: 0,
    endIndex: total,
    offsetBeforePx: 0,
    offsetAfterPx: 0,
  };
}

/** 图标/平铺为 CSS grid：窗口化（含 padding 偏移）仍会压扁格子，一律关闭。 */
function isGridIconOrTilesMode(mode: string): boolean {
  return mode.startsWith("icons") || mode === "tiles";
}

const primaryVirtual = computed((): ListVirtualWindow => {
  const items = primaryVirtItems.value;
  const mode = navigatorViewMode.value as PlaceholderViewMode;
  if (isGridIconOrTilesMode(mode)) {
    return inactiveVirtualWindow(items.length);
  }
  const viewportHeight = listViewportHeightPx(listViewport.value);
  const rowHeight = resolveListRowHeightPx(mode, isCompactListDensity());
  const grouped = navigatorGroupByType.value;
  let win: ListVirtualWindow;
  if (grouped) {
    // 组头变高：按项高前缀和窗口化（仅纵向列表视图）。
    const heights = items.map((item) => virtItemHeightPx(item, rowHeight));
    win = computeVariableListVirtualWindow({
      heights,
      scrollTop: primaryScrollTop.value,
      viewportHeight,
    });
  } else {
    win = computeListVirtualWindow({
      total: items.length,
      scrollTop: primaryScrollTop.value,
      viewportHeight,
      rowHeight,
      columns: estimateListColumns(mode, listViewport.value.w),
      mode,
    });
  }
  probeListWindow(win, items.length, grouped);
  return win;
});

const secondaryVirtual = computed((): ListVirtualWindow => {
  const total = secondaryEntries.value.length;
  const mode = navigatorViewMode.value as PlaceholderViewMode;
  if (isGridIconOrTilesMode(mode)) {
    return inactiveVirtualWindow(total);
  }
  return computeListVirtualWindow({
    total,
    scrollTop: secondaryScrollTop.value,
    viewportHeight: listViewportHeightPx(secondaryViewport.value),
    rowHeight: resolveListRowHeightPx(mode, isCompactListDensity()),
    columns: estimateListColumns(mode, secondaryViewport.value.w),
    mode,
  });
});

const primaryWindowedItems = computed(() => {
  const win = primaryVirtual.value;
  const items = primaryVirtItems.value;
  if (!win.active) return items;
  return items.slice(win.startIndex, win.endIndex);
});

const secondaryWindowedEntries = computed(() => {
  const win = secondaryVirtual.value;
  if (!win.active) return secondaryEntries.value;
  return secondaryEntries.value.slice(win.startIndex, win.endIndex);
});

function onPrimaryListScroll(event: Event) {
  const el = event.target;
  if (!(el instanceof HTMLElement)) return;
  primaryScrollTop.value = el.scrollTop;
  scheduleViewportIconObservers();
}

function onSecondaryListScroll(event: Event) {
  const el = event.target;
  if (!(el instanceof HTMLElement)) return;
  secondaryScrollTop.value = el.scrollTop;
  scheduleViewportIconObservers();
}

watch(currentPath, () => {
  primaryScrollTop.value = 0;
  if (listRoot.value) listRoot.value.scrollTop = 0;
});

watch(secondaryPath, () => {
  secondaryScrollTop.value = 0;
  if (secondaryListRoot.value) secondaryListRoot.value.scrollTop = 0;
});

function requestIconsFor(paths: string[], priority = false) {
  // Provider URI 无本地 shell 图标；走 Fluent 回退即可。
  const localPaths = paths.filter((path) => !isProviderPath(path));
  if (!localPaths.length) return;
  const size = VIEW_ICON_SIZE[navigatorViewMode.value];
  // Icon views: prefer system thumbnails for images; list/details use small shell icons.
  requestShellIcons(localPaths, size, isIconView.value, { priority });
}

/**
 * AIA-182 / AIA-278 / FIL-50：按滚动几何请求缩略图（整屏 + 行高 overscan）。
 * 不用 IntersectionObserver 重绑：图标格全量 DOM 时每次滚动 disconnect/observe
 * 会把慢速外部缩略图器进行中的视口排队项 leave/cancel，最终只剩约一批（≈两行）。
 */
let iconRequestSyncTimer: ReturnType<typeof requestAnimationFrame> | null = null;
/** 当前缩略图请求窗内的路径；离开窗口时 diff cancel（FIL-33）。 */
let primaryThumbPaths = new Set<string>();
let secondaryThumbPaths = new Set<string>();

function pathsLeavingWindow(prev: Set<string>, next: Iterable<string>): string[] {
  const nextSet = next instanceof Set ? next : new Set(next);
  const leaving: string[] = [];
  for (const path of prev) {
    if (!nextSet.has(path)) leaving.push(path);
  }
  return leaving;
}

function resetThumbRequestWindows() {
  primaryThumbPaths = new Set();
  secondaryThumbPaths = new Set();
}

function syncThumbRequestsForPane(
  entries: { path: string }[],
  scrollTop: number,
  viewport: { w: number; h: number },
  previousPaths: Set<string>,
): Set<string> {
  const mode = navigatorViewMode.value;
  const size = VIEW_ICON_SIZE[mode];
  const columns = estimateListColumns(mode, viewport.w);
  const rowHeight = resolveListRowHeightPx(
    mode as PlaceholderViewMode,
    isCompactListDensity(),
  );
  const win = computeThumbRequestWindow({
    total: entries.length,
    scrollTop,
    viewportHeight: listViewportHeightPx(viewport),
    rowHeight,
    columns,
    mode,
  });
  const nextPaths: string[] = [];
  for (let i = win.startIndex; i < win.endIndex; i += 1) {
    const path = entries[i]?.path;
    if (path) nextPaths.push(path);
  }
  const nextSet = new Set(nextPaths);
  const left = pathsLeavingWindow(previousPaths, nextSet);
  // 仅取消离开请求窗的排队/in-flight；已成功写入的缓存保留（仍挂载项回滚可直接命中）。
  if (left.length) cancelShellIconRequests(left, size);
  if (nextPaths.length) requestIconsFor(nextPaths, true);
  return nextSet;
}

function refreshViewportIconRequests() {
  primaryThumbPaths = syncThumbRequestsForPane(
    visibleEntries.value,
    primaryScrollTop.value,
    listViewport.value,
    primaryThumbPaths,
  );
  secondaryThumbPaths = syncThumbRequestsForPane(
    secondaryEntries.value,
    secondaryScrollTop.value,
    secondaryViewport.value,
    secondaryThumbPaths,
  );
}

function scheduleViewportIconObservers() {
  if (iconRequestSyncTimer != null) return;
  iconRequestSyncTimer = requestAnimationFrame(() => {
    iconRequestSyncTimer = null;
    refreshViewportIconRequests();
  });
}

watch(
  [
    visibleEntries,
    secondaryEntries,
    navigatorViewMode,
    listViewport,
    secondaryViewport,
  ],
  () => {
    scheduleViewportIconObservers();
  },
  { immediate: true },
);

/**
 * AIA-238 / AIA-269: debounce overlapping DOM focus + Tauri onFocusChanged.
 * Returning from Explorer often fires both; keep work to clipboard + re-list only
 * (view-mode / preview settings do not change on focus).
 */
let lastFocusRefreshAt = 0;
let unlistenWindowFocus: (() => void) | null = null;
const FOCUS_REFRESH_DEBOUNCE_MS = 400;

function onWindowFocus() {
  const now = Date.now();
  if (now - lastFocusRefreshAt < FOCUS_REFRESH_DEBOUNCE_MS) return;
  lastFocusRefreshAt = now;
  void refreshClipboardFromSystem();
  refreshActiveListing();
  if (dualPaneEnabled.value && secondaryPath.value.trim()) {
    void navigateSecondary(secondaryPath.value);
  }
}

onMounted(() => {
  registerNavFileShortcuts();
  window.addEventListener("mousemove", onMarqueeMove);
  window.addEventListener("mouseup", onMarqueeEnd);
  window.addEventListener("focus", onWindowFocus);
  // Tauri: returning from Explorer often skips DOM focus; listen to OS focus.
  void getCurrentWindow()
    .onFocusChanged(({ payload: focused }) => {
      if (focused) onWindowFocus();
    })
    .then((unlisten) => {
      unlistenWindowFocus = unlisten;
    })
    .catch(() => {
      /* non-Tauri / permission — DOM focus still covers browser/dev */
    });
  // Critical: OpenState + listing first. Preview/viewMode already sync-applied
  // from main.ts settings hydrate (viewMode) or default until after-paint.
  startupMark("navigator:bootstrap_start");
  void (async () => {
    await bootstrapNavigatorOpenState();
    startupMark("navigator:open_state_ready");
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
  scheduleAfterPaint(() => {
    void refreshPreviewSetting();
    void loadNavigatorViewMode();
  });
  scheduleIdle(() => {
    void loadNewFileTemplates();
    void loadFavoritesOsHookStatus().catch(() => undefined);
  }, 3000);
});

onUnmounted(() => {
  clearPendingNameRename();
  disarmRenameOutsideListener();
  resetThumbRequestWindows();
  if (iconRequestSyncTimer != null) {
    cancelAnimationFrame(iconRequestSyncTimer);
    iconRequestSyncTimer = null;
  }
  clearShellIconQueue();
  listSizeObserver?.disconnect();
  listSizeObserver = null;
  secondarySizeObserver?.disconnect();
  secondarySizeObserver = null;
  unregisterNavFileShortcuts();
  window.removeEventListener("mousemove", onMarqueeMove);
  window.removeEventListener("mouseup", onMarqueeEnd);
  window.removeEventListener("focus", onWindowFocus);
  if (unlistenWindowFocus) {
    unlistenWindowFocus();
    unlistenWindowFocus = null;
  }
});

watch(currentPath, () => {
  // AIA-278: abandon prior-folder thumb IPC before the new listing paints.
  clearShellIconQueue();
  if (renamingPath.value) cancelInlineRename();
});

watch(secondaryPath, (next, prev) => {
  if (next === prev) return;
  clearShellIconQueue();
});

watch(navigatorViewMode, () => {
  // Size-keyed cache; drop prior-size queue so scroll does not wait on stale work.
  clearShellIconQueue();
});

function formatSize(entry: NavigatorEntry): string {
  if (entry.kind === "dir") return "—";
  if (entry.size < 1024) return `${entry.size} B`;
  if (entry.size < 1024 * 1024) return `${(entry.size / 1024).toFixed(1)} KB`;
  if (entry.size < 1024 * 1024 * 1024) return `${(entry.size / (1024 * 1024)).toFixed(1)} MB`;
  return `${(entry.size / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/** AIA-269: reuse one formatter — toLocaleString-per-row was costly on large details lists. */
let modifiedDateFormatter: Intl.DateTimeFormat | null = null;
function formatModified(entry: NavigatorEntry): string {
  const ms = entry.modifiedMs;
  if (ms == null || !Number.isFinite(ms)) return "—";
  try {
    if (!modifiedDateFormatter) {
      modifiedDateFormatter = new Intl.DateTimeFormat(undefined, {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
      });
    }
    return modifiedDateFormatter.format(ms);
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

function sortIndicator(column: "name" | "size" | "modified" | "type"): string {
  if (sortColumn.value !== column) return "";
  return sortDirection.value === "asc" ? " ▲" : " ▼";
}

const columnWidths = ref<DetailsColumnWidths>({ ...DEFAULT_DETAILS_COLUMN_WIDTHS });
const detailsGridStyle = computed(() => ({
  gridTemplateColumns: detailsGridTemplate(columnWidths.value),
}));

const typeLabels = computed(() => ({
  folder: t("navigator.typeFolder"),
  file: t("navigator.typeFile"),
  typedFile: (ext: string) => t("navigator.typeFileExt", { ext }),
}));

function formatEntryType(entry: NavigatorEntry): string {
  return entryTypeLabel(entry.kind, entry.name, typeLabels.value);
}

let columnResize: {
  column: DetailsColumnId;
  startX: number;
  widths: DetailsColumnWidths;
} | null = null;

function onColumnResizeStart(event: PointerEvent, column: DetailsColumnId) {
  event.preventDefault();
  event.stopPropagation();
  columnResize = {
    column,
    startX: event.clientX,
    widths: { ...columnWidths.value },
  };
  const target = event.currentTarget;
  if (target instanceof HTMLElement) {
    target.setPointerCapture(event.pointerId);
  }
  window.addEventListener("pointermove", onColumnResizeMove);
  window.addEventListener("pointerup", onColumnResizeEnd);
}

function onColumnResizeMove(event: PointerEvent) {
  if (!columnResize) return;
  const delta = event.clientX - columnResize.startX;
  columnWidths.value = resizeColumnWidth(
    columnResize.widths,
    columnResize.column,
    delta,
  );
}

function onColumnResizeEnd() {
  columnResize = null;
  window.removeEventListener("pointermove", onColumnResizeMove);
  window.removeEventListener("pointerup", onColumnResizeEnd);
}

const iconPixelSize = computed(() => VIEW_ICON_SIZE[navigatorViewMode.value]);

/**
 * 图标/平铺为 CSS grid：用 padding 承担窗口化前后偏移。
 * grid 内再插 virt-spacer 会挤乱行高，大目录缩略图会被压成细条。
 */
function gridVirtualPaddingStyle(win: ListVirtualWindow): Record<string, string> {
  if (!win.active) return {};
  if (!(isIconView.value || isTilesView.value)) return {};
  // 分组纵向流（list--virt-flat）仍用 flex spacer，不走 padding。
  if (navigatorGroupByType.value) return {};
  return {
    paddingTop: `calc(var(--fluent-space-sm, 8px) + ${win.offsetBeforePx}px)`,
    paddingBottom: `calc(var(--fluent-space-sm, 8px) + ${win.offsetAfterPx}px)`,
  };
}

function usesVirtSpacers(win: ListVirtualWindow): boolean {
  if (!win.active) return false;
  if ((isIconView.value || isTilesView.value) && !navigatorGroupByType.value) {
    return false;
  }
  return true;
}

/** Reserve shell-icon slot at the active view size (AIA-267). */
const primaryListStyle = computed(() => ({
  "--nav-icon-size": iconSlotCssSize(navigatorViewMode.value),
  "--nav-icon-caption-extra": `${ICON_CAPTION_EXTRA_PX}px`,
  ...gridVirtualPaddingStyle(primaryVirtual.value),
}));

const secondaryListStyle = computed(() => ({
  "--nav-icon-size": iconSlotCssSize(navigatorViewMode.value),
  "--nav-icon-caption-extra": `${ICON_CAPTION_EXTRA_PX}px`,
  ...gridVirtualPaddingStyle(secondaryVirtual.value),
}));

const showPrimaryVirtSpacers = computed(() => usesVirtSpacers(primaryVirtual.value));
const showSecondaryVirtSpacers = computed(() => usesVirtSpacers(secondaryVirtual.value));

const showListPlaceholders = computed(
  () => listLoading.value && visibleEntries.value.length === 0,
);
const showSecondaryPlaceholders = computed(
  () => secondaryLoading.value && secondaryEntries.value.length === 0,
);
const placeholderCount = computed(() =>
  estimatePlaceholderCount(
    navigatorViewMode.value,
    listViewport.value.w,
    listViewport.value.h,
  ),
);
const secondaryPlaceholderCount = computed(() =>
  estimatePlaceholderCount(
    navigatorViewMode.value,
    secondaryViewport.value.w,
    secondaryViewport.value.h,
  ),
);
const placeholderIndices = computed(() =>
  Array.from({ length: placeholderCount.value }, (_, i) => i),
);
const secondaryPlaceholderIndices = computed(() =>
  Array.from({ length: secondaryPlaceholderCount.value }, (_, i) => i),
);

function syncListViewport(
  el: HTMLElement | null,
  target: typeof listViewport,
): void {
  if (!el) {
    target.value = { w: 0, h: 0 };
    return;
  }
  target.value = { w: el.clientWidth, h: el.clientHeight };
}

function bindListSizeObserver(
  el: HTMLElement | null,
  previous: ResizeObserver | null,
  target: typeof listViewport,
): ResizeObserver | null {
  previous?.disconnect();
  syncListViewport(el, target);
  if (!el || typeof ResizeObserver === "undefined") return null;
  const observer = new ResizeObserver(() => syncListViewport(el, target));
  observer.observe(el);
  return observer;
}

watch([listRoot, secondaryListRoot], () => {
  listSizeObserver = bindListSizeObserver(
    listRoot.value,
    listSizeObserver,
    listViewport,
  );
  secondarySizeObserver = bindListSizeObserver(
    secondaryListRoot.value,
    secondarySizeObserver,
    secondaryViewport,
  );
});

async function restoreSelection() {
  const paths = selectionPaths();
  if (!paths.length) return;
  try {
    const result = await restoreRecyclePaths(paths);
    if (!result.ok) {
      onNotice(result.error ?? t("contextMenu.opFailed"), "error");
      return;
    }
    onNotice(t("recycleBin.restored", { count: result.paths.length }));
    clearSelection();
    await refreshListing();
  } catch (err) {
    onNotice(err instanceof Error ? err.message : String(err), "error");
  }
}

function onActivate(entry: NavigatorEntry) {
  clearPendingNameRename();
  if (renamingPath.value) cancelInlineRename();
  if (isRecyclePath(currentPath.value)) {
    selectOnly(entry.path);
    void restoreSelection();
    return;
  }
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

function focusEntryRow(path: string) {
  const root = listRoot.value;
  if (!root) return;
  const row = root.querySelector<HTMLElement>(`[data-entry-path="${CSS.escape(path)}"]`);
  row?.focus({ preventScroll: true });
  row?.scrollIntoView({ block: "nearest", inline: "nearest" });
}

/** AIA-232: Ctrl+wheel zooms icon size (Explorer-like). */
function onListWheel(event: WheelEvent) {
  if (!event.ctrlKey && !event.metaKey) return;
  if (isGuidePath(currentPath.value)) return;
  event.preventDefault();
  if (event.deltaY === 0) return;
  const direction: 1 | -1 = event.deltaY < 0 ? 1 : -1;
  const next = nudgeIconViewMode(navigatorViewMode.value, direction);
  if (next === navigatorViewMode.value) return;
  void setNavigatorViewMode(next);
}

function onListKeyDown(event: KeyboardEvent) {
  if (event.altKey || event.metaKey || event.ctrlKey) return;
  if (isEditableTarget(event.target)) return;
  const direction = arrowDirectionFromKey(event.key);
  if (!direction) return;
  const columns = isIconView.value
    ? estimateIconColumns(listRoot.value?.clientWidth ?? 0, navigatorViewMode.value)
    : 1;
  const moved = moveSelectionByArrow(
    orderedPaths.value,
    direction,
    columns,
    event.shiftKey,
    {
      selectedPaths: selectedPaths.value,
      selectionAnchor: selectionAnchor.value,
    },
  );
  if (!moved) return;
  selectedPaths.value = moved.state.selectedPaths;
  selectionAnchor.value = moved.state.selectionAnchor;
  event.preventDefault();
  event.stopPropagation();
  void nextTick(() => focusEntryRow(moved.path));
}

/**
 * AIA-280 / Explorer press→drag: do **not** preventDefault on mousedown.
 * Chromium/WebView2 cancels HTML5 dragstart when mousedown default is
 * prevented (AIA-268 unconditional preventDefault broke all row drags —
 * primary, dual-pane secondary). Raise mitigation stays on tabindex=-1 +
 * host `drag_focus` (MA_NOACTIVATE / WebView 子 HWND subclass / z-order，FIL-43)。
 * 禁止在此 focus()/setFocus；选中与行焦点在 `@click`（mouse-up）完成；
 * dragstart 可 selectOnly 且不调用 focusEntryRow。
 */
function onRowMouseDown(event: MouseEvent, _entry: NavigatorEntry) {
  if (event.button !== 0) return;
}

function onRowClick(event: MouseEvent, entry: NavigatorEntry) {
  // 改名会话占有焦点：框外 pointerdown 已先提交并清空 renamingPath；
  // 拖选文字后在框外松开触发的 click 仍带着 renamingPath，忽略以免抢焦点。
  if (renamingPath.value) return;
  // Clicking away from a pending name-rename (or another row) cancels the delay.
  if (pendingNameRenamePath && pendingNameRenamePath !== entry.path) {
    clearPendingNameRename();
  }
  // Mouse-up without a completed drag — finalize selection (click path).
  if (event.shiftKey) {
    clearPendingNameRename();
    selectRange(orderedPaths.value, selectionAnchor.value, entry.path);
    focusEntryRow(entry.path);
    return;
  }
  if (event.ctrlKey || event.metaKey) {
    clearPendingNameRename();
    toggleInSelection(entry.path);
    focusEntryRow(entry.path);
    return;
  }
  selectOnly(entry.path);
  focusEntryRow(entry.path);
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
  if (!currentPath.value) return;
  await navigateTo(currentPath.value, {
    syncOpenState: false,
    recordHistory: false,
    recordVisit: false,
  });
  if (dualPaneEnabled.value && secondaryPath.value.trim()) {
    await navigateSecondary(secondaryPath.value);
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
  if (!currentPath.value) return;
  try {
    await refreshClipboardFromSystem();
    if (!hasClipboard.value) return;
    const sources = [...clipboardPaths.value];
    const wasCut = clipboardMode.value === "cut";
    const result = await pasteClipboard(currentPath.value);
    if (!result.ok) {
      onNotice(result.error ?? t("contextMenu.opFailed"));
      return;
    }
    if (wasCut) recordMoveUndo(sources, result.paths);
    else recordCopyUndo(result.paths);
    onNotice(t("contextMenu.pasted", { count: result.paths.length }));
    await refreshListing();
  } catch (err) {
    onNotice(err instanceof Error ? err.message : String(err));
  }
}

async function shortcutUndo() {
  try {
    const result = await undoLast();
    if (!result.ok) {
      if (result.error && result.error !== "Nothing to undo") {
        onNotice(result.error, "error");
      }
      return;
    }
    onNotice(t("contextMenu.undone"));
    await refreshListing();
  } catch (err) {
    onNotice(err instanceof Error ? err.message : String(err), "error");
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
    if (result.paths[0]) recordCreateUndo(result.paths[0]);
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
    if (result.paths[0]) recordCreateUndo(result.paths[0]);
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
        recordDeleteUndo(recyclable);
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
    recordDeleteUndo(paths);
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
    // AIA-238: F5 must not await list IPC (same non-blocking pattern as AIA-225).
    refreshActiveListing();
    if (dualPaneEnabled.value && secondaryPath.value.trim()) {
      void navigateSecondary(secondaryPath.value);
    }
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
  registerShortcutHandler("navigator.undo", () => {
    void shortcutUndo();
  });
  registerShortcutHandler("navigator.previewBridge", () => {
    void shortcutPreviewBridge();
  });
  registerShortcutHandler("navigator.properties", () => {
    shortcutProperties();
  });
  registerShortcutHandler("navigator.previewPane", () => {
    shortcutTogglePreviewPane();
  });
  registerShortcutHandler("navigator.detailsPane", () => {
    shortcutToggleDetailsPane();
  });
  registerShortcutHandler("navigator.closeTab", () => {
    if (!liveOpenState.value.tabGroups.length) return;
    void closeNavigatorTab(liveOpenState.value.currentTabGroupIndex);
  });
  for (let tabNumber = 1; tabNumber <= 9; tabNumber += 1) {
    const index = tabNumber - 1;
    registerShortcutHandler(`navigator.jumpTab${tabNumber}`, () => {
      if (index >= liveOpenState.value.tabGroups.length) return;
      void switchNavigatorTab(index);
    });
  }
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
    "navigator.undo",
    "navigator.previewBridge",
    "navigator.properties",
    "navigator.previewPane",
    "navigator.detailsPane",
    "navigator.closeTab",
    "navigator.jumpTab1",
    "navigator.jumpTab2",
    "navigator.jumpTab3",
    "navigator.jumpTab4",
    "navigator.jumpTab5",
    "navigator.jumpTab6",
    "navigator.jumpTab7",
    "navigator.jumpTab8",
    "navigator.jumpTab9",
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

/* —— Drag in (AFM + Explorer/external) · drag out (OLE / CF_HDROP + link) —— */
function onDragStart(event: DragEvent, entry: NavigatorEntry) {
  // Drag past threshold: select for payload only — never focusEntryRow / raise.
  if (!isSelected(entry.path)) {
    selectOnly(entry.path);
  }
  const paths = selectionPaths();
  if (event.dataTransfer) {
    setOutboundPathPayload(event.dataTransfer, paths);
    // all: copy / move / link (Alt); Hybrid OLE offers the same mask.
    event.dataTransfer.effectAllowed = "all";
  }
  armNativeFileDragOut(paths);
}

function onDragOver(event: DragEvent, entry: NavigatorEntry) {
  if (entry.kind !== "dir") return;
  if (isProviderPath(entry.path) || isProviderPath(currentPath.value)) return;
  const mode = acceptFolderDragOver(event, entry.path, { rejectIfSelected: true });
  if (mode == null) return;
  dragOverPath.value = entry.path;
}

function onDragLeave(entry: NavigatorEntry) {
  if (dragOverPath.value === entry.path) {
    dragOverPath.value = null;
  }
}

function onListDragOver(event: DragEvent, destPath: string | null | undefined) {
  if (!destPath || isGuidePath(destPath) || isRecyclePath(destPath) || isProviderPath(destPath)) {
    return;
  }
  // Row handlers stopPropagation when targeting a folder; background accepts drops.
  event.preventDefault();
  const mode = previewDropMode(event, destPath, previewSourcePaths(event));
  if (event.dataTransfer) event.dataTransfer.dropEffect = mode;
  if (!dragOverPath.value) dragOverPath.value = destPath;
}

function onListDragLeave(event: DragEvent, destPath: string | null | undefined) {
  if (!destPath) return;
  const related = event.relatedTarget as Node | null;
  const root = event.currentTarget as HTMLElement | null;
  if (root && related && root.contains(related)) return;
  if (dragOverPath.value === destPath) dragOverPath.value = null;
}

async function transferDroppedPaths(
  event: DragEvent,
  destPath: string,
  destLabel: string,
) {
  dragOverPath.value = null;
  const result = await commitFolderDrop(event, destPath);
  if (!result.ok) {
    if (result.error && result.error !== "empty") {
      onNotice(result.error === "failed" ? t("contextMenu.opFailed") : result.error, "error");
    }
    return;
  }
  if (result.mode && result.count != null) {
    onNotice(t(noticeKeyForDropMode(result.mode), { count: result.count, name: destLabel }));
  }
  // Dual-pane secondary listing is outside refreshActiveListing — keep prior behavior.
  await refreshListing();
}

async function onDrop(event: DragEvent, entry: NavigatorEntry) {
  if (entry.kind !== "dir") return;
  if (isProviderPath(entry.path) || isProviderPath(currentPath.value)) return;
  await transferDroppedPaths(event, entry.path, entry.name);
}

async function onListDrop(event: DragEvent, destPath: string | null | undefined) {
  if (!destPath || isGuidePath(destPath) || isRecyclePath(destPath) || isProviderPath(destPath)) {
    return;
  }
  const base = destPath.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || destPath;
  await transferDroppedPaths(event, destPath, base);
}
</script>

<template>
  <!-- AIA-78: no in-page title / Kernel status meta — path via shell address bar + pane-path; counts in shell statusbar. -->
  <section class="fluent-page fluent-page--navigator">
    <div
      class="navigator-body"
      :class="{
        'navigator-body--preview': previewPaneEnabled,
        'navigator-body--details': detailsPaneOpen,
      }"
    >
    <div class="panes" :class="{ 'panes--dual': dualPaneEnabled }">
    <div
      class="pane"
      :class="{ 'pane--active': activePane === 'primary' || !dualPaneEnabled }"
      @mousedown="setActivePane('primary')"
    >
    <NavigatorGuide
      v-if="guideSurfaceMounted"
      v-show="showingGuide"
      :open-path="(path) => navigateTo(path)"
    />
    <div v-if="listSurfaceMounted" v-show="!showingGuide" class="pane-list-surface">
    <p class="pane-path" :title="currentPath">{{ currentPath || t("navigator.noPath") }}</p>
    <p v-if="listError && !listLoading" class="fluent-banner fluent-banner--error">{{ listError }}</p>
    <p v-else-if="folderSearchActive && primarySearchLoading" class="fluent-banner">
      {{ t("navigator.filterSearching") }}
    </p>
    <p
      v-else-if="folderSearchActive && !primarySearchLoading && !visibleEntries.length"
      class="fluent-banner fluent-banner--info"
    >
      {{ t("navigator.filterEmpty") }}
    </p>
    <p v-else-if="listing && listing.entries.length === 0 && !listLoading" class="fluent-banner">
      {{ t("navigator.empty") }}
    </p>
    <p
      v-if="folderSearchActive && primarySearch?.truncated"
      class="fluent-banner fluent-banner--info"
    >
      {{ t("navigator.filterTruncated", { count: visibleEntries.length }) }}
    </p>

    <div
      v-if="listing || listLoading"
      ref="listRoot"
      class="fluent-surface list"
      :class="{
        'list--details': isDetailsView,
        'list--list': isListView,
        'list--icons': isIconView,
        'list--icons-extra-large': navigatorViewMode === 'iconsExtraLarge',
        'list--icons-large': navigatorViewMode === 'iconsLarge',
        'list--icons-medium': navigatorViewMode === 'iconsMedium',
        'list--icons-small': navigatorViewMode === 'iconsSmall',
        'list--tiles': isTilesView,
        'list--content': isContentView,
        'list--grouped': navigatorGroupByType,
        'list--virt-flat': navigatorGroupByType && primaryVirtual.active,
        'list--drop': dragOverPath === currentPath,
      }"
      :style="primaryListStyle"
      role="list"
      tabindex="-1"
      data-focus-default
      :aria-busy="listLoading ? 'true' : undefined"
      :aria-label="listLoading ? t('navigator.loading') : undefined"
      @keydown="onListKeyDown"
      @wheel="onListWheel"
      @scroll="onPrimaryListScroll"
      @contextmenu="onListContextMenu"
      @mousedown="onListMouseDown"
      @click.self="clearSelection()"
      @dragover="onListDragOver($event, currentPath)"
      @dragleave="onListDragLeave($event, currentPath)"
      @drop="onListDrop($event, currentPath)"
    >
      <div
        v-if="isDetailsView"
        class="list-header"
        role="row"
        :style="detailsGridStyle"
      >
        <button type="button" class="col-sort col-name" @click="toggleSort('name')">
          {{ t("navigator.colName") }}{{ sortIndicator("name") }}
          <span
            class="col-resizer"
            role="separator"
            aria-orientation="vertical"
            @pointerdown.stop="onColumnResizeStart($event, 'name')"
          />
        </button>
        <button type="button" class="col-sort col-type" @click="toggleSort('type')">
          {{ t("navigator.colType") }}{{ sortIndicator("type") }}
          <span
            class="col-resizer"
            role="separator"
            aria-orientation="vertical"
            @pointerdown.stop="onColumnResizeStart($event, 'type')"
          />
        </button>
        <button type="button" class="col-sort col-size" @click="toggleSort('size')">
          {{ t("navigator.colSize") }}{{ sortIndicator("size") }}
          <span
            class="col-resizer"
            role="separator"
            aria-orientation="vertical"
            @pointerdown.stop="onColumnResizeStart($event, 'size')"
          />
        </button>
        <button type="button" class="col-sort col-modified" @click="toggleSort('modified')">
          {{ t("navigator.colModified") }}{{ sortIndicator("modified") }}
          <span
            class="col-resizer"
            role="separator"
            aria-orientation="vertical"
            @pointerdown.stop="onColumnResizeStart($event, 'modified')"
          />
        </button>
      </div>

      <template v-if="showListPlaceholders">
        <div
          v-for="index in placeholderIndices"
          :key="`ph-${index}`"
          class="fluent-list-row fluent-list-row--density row row--placeholder"
          :class="{
            'row--icon': isIconView,
            'row--tile': isTilesView,
            'row--content': isContentView,
          }"
          :style="isDetailsView ? detailsGridStyle : undefined"
          aria-hidden="true"
        >
          <span class="kind kind--placeholder" />
          <span class="name-block">
            <span class="name name--placeholder" />
            <span v-if="isTilesView || isContentView" class="row-meta row-meta--placeholder" />
          </span>
          <span v-if="isDetailsView" class="type type--placeholder" />
          <span v-if="isDetailsView" class="size size--placeholder" />
          <span v-if="isDetailsView" class="modified modified--placeholder" />
        </div>
      </template>
      <template v-else>
      <div
        v-if="showPrimaryVirtSpacers && primaryVirtual.offsetBeforePx > 0"
        class="virt-spacer"
        aria-hidden="true"
        :style="{ height: `${primaryVirtual.offsetBeforePx}px` }"
      />
      <template v-for="item in primaryWindowedItems" :key="item.key">
        <div
          v-if="item.kind === 'header'"
          class="list-group-header"
          role="presentation"
        >
          {{ item.label }}
          <span class="list-group-count">{{ item.count }}</span>
        </div>
        <div
          v-else
          class="fluent-list-row fluent-list-row--density row"
          :class="{
            'row--icon': isIconView,
            'row--tile': isTilesView,
            'row--content': isContentView,
            'row--selected': isSelected(item.entry.path),
            'row--cut': isClipboardCutPath(item.entry.path),
            'row--drop': dragOverPath === item.entry.path,
          }"
          :style="isDetailsView ? detailsGridStyle : undefined"
          role="listitem"
          tabindex="-1"
          draggable="true"
          :data-entry-path="item.entry.path"
          :aria-selected="isSelected(item.entry.path)"
          :title="entryHoverTitle(item.entry)"
          @mousedown="onRowMouseDown($event, item.entry)"
          @click="onRowClick($event, item.entry)"
          @dblclick="onActivate(item.entry)"
          @keydown.enter.prevent="onActivate(item.entry)"
          @contextmenu="onContextMenu($event, item.entry)"
          @dragstart="onDragStart($event, item.entry)"
          @dragover="onDragOver($event, item.entry)"
          @dragleave="onDragLeave(item.entry)"
          @drop="onDrop($event, item.entry)"
        >
          <span class="kind" :data-kind="item.entry.kind">
            <ShellIconImg
              :path="item.entry.path"
              :size="iconPixelSize"
              :is-dir="item.entry.kind === 'dir'"
            />
          </span>
          <textarea
            v-if="renamingPath === item.entry.path"
            :ref="setRenameInputEl"
            v-model="renameDraft"
            class="fluent-control inline-rename"
            rows="1"
            wrap="soft"
            spellcheck="false"
            :aria-label="t('contextMenu.rename')"
            @mousedown.stop
            @click.stop
            @dblclick.stop
            @input="onRenameDraftInput"
            @keydown.enter.prevent.stop="commitInlineRename"
            @keydown.escape.prevent.stop="cancelInlineRename"
            @blur="onRenameBlur"
          />
          <span v-else class="name-block">
            <span class="name" @click="onNameClick($event, item.entry)">{{ item.entry.name }}</span>
            <span v-if="folderSearchActive" class="row-location">{{ entryLocation(item.entry.path) }}</span>
            <span v-if="isTilesView || isContentView" class="row-meta">
              {{ formatEntryType(item.entry) }}
              <template v-if="item.entry.kind !== 'dir'"> · {{ formatSize(item.entry) }}</template>
            </span>
          </span>
          <span v-if="isDetailsView" class="type">{{ formatEntryType(item.entry) }}</span>
          <span v-if="isDetailsView" class="size">{{ formatSize(item.entry) }}</span>
          <span v-if="isDetailsView" class="modified">{{ formatModified(item.entry) }}</span>
        </div>
      </template>
      <div
        v-if="showPrimaryVirtSpacers && primaryVirtual.offsetAfterPx > 0"
        class="virt-spacer"
        aria-hidden="true"
        :style="{ height: `${primaryVirtual.offsetAfterPx}px` }"
      />
      </template>

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
      <p
        v-if="secondaryError && !secondaryLoading"
        class="fluent-banner fluent-banner--error"
      >
        {{ secondaryError }}
      </p>
      <div
        v-if="secondaryListing || secondaryLoading"
        ref="secondaryListRoot"
        class="fluent-surface list"
        :class="{
          'list--details': isDetailsView,
          'list--list': isListView,
          'list--icons': isIconView,
          'list--icons-extra-large': navigatorViewMode === 'iconsExtraLarge',
          'list--icons-large': navigatorViewMode === 'iconsLarge',
          'list--icons-medium': navigatorViewMode === 'iconsMedium',
          'list--icons-small': navigatorViewMode === 'iconsSmall',
          'list--tiles': isTilesView,
          'list--content': isContentView,
          'list--drop': dragOverPath === secondaryPath,
        }"
        :style="secondaryListStyle"
        role="list"
        :aria-busy="secondaryLoading ? 'true' : undefined"
        :aria-label="secondaryLoading ? t('navigator.loading') : undefined"
        @wheel="onListWheel"
        @scroll="onSecondaryListScroll"
        @dragover="onListDragOver($event, secondaryPath)"
        @dragleave="onListDragLeave($event, secondaryPath)"
        @drop="onListDrop($event, secondaryPath)"
      >
        <template v-if="showSecondaryPlaceholders">
          <div
            v-for="index in secondaryPlaceholderIndices"
            :key="`sph-${index}`"
            class="fluent-list-row fluent-list-row--density row row--placeholder"
            :class="{
              'row--icon': isIconView,
              'row--tile': isTilesView,
              'row--content': isContentView,
            }"
            :style="isDetailsView ? detailsGridStyle : undefined"
            aria-hidden="true"
          >
            <span class="kind kind--placeholder" />
            <span class="name-block">
              <span class="name name--placeholder" />
              <span v-if="isTilesView || isContentView" class="row-meta row-meta--placeholder" />
            </span>
            <span v-if="isDetailsView" class="type type--placeholder" />
            <span v-if="isDetailsView" class="size size--placeholder" />
            <span v-if="isDetailsView" class="modified modified--placeholder" />
          </div>
        </template>
        <template v-else>
        <div
          v-if="showSecondaryVirtSpacers && secondaryVirtual.offsetBeforePx > 0"
          class="virt-spacer"
          aria-hidden="true"
          :style="{ height: `${secondaryVirtual.offsetBeforePx}px` }"
        />
        <div
          v-for="entry in secondaryWindowedEntries"
          :key="entry.path"
          class="fluent-list-row fluent-list-row--density row"
          :class="{
            'row--icon': isIconView,
            'row--tile': isTilesView,
            'row--content': isContentView,
            'row--selected': isSelected(entry.path),
            'row--cut': isClipboardCutPath(entry.path),
            'row--drop': dragOverPath === entry.path,
          }"
          :style="isDetailsView ? detailsGridStyle : undefined"
          role="listitem"
          tabindex="-1"
          draggable="true"
          :data-entry-path="entry.path"
          :aria-selected="isSelected(entry.path)"
          :title="entryHoverTitle(entry)"
          @mousedown="onRowMouseDown($event, entry)"
          @click="onRowClick($event, entry)"
          @dblclick="onActivateSecondary(entry)"
          @keydown.enter.prevent="onActivateSecondary(entry)"
          @contextmenu="onContextMenu($event, entry)"
          @dragstart="onDragStart($event, entry)"
          @dragover="onDragOver($event, entry)"
          @dragleave="onDragLeave(entry)"
          @drop="onDrop($event, entry)"
        >
          <span class="kind" :data-kind="entry.kind">
            <ShellIconImg
              :path="entry.path"
              :size="iconPixelSize"
              :is-dir="entry.kind === 'dir'"
            />
          </span>
          <span class="name-block">
            <span class="name">{{ entry.name }}</span>
            <span v-if="folderSearchActive" class="row-location">{{ entryLocation(entry.path) }}</span>
            <span v-if="isTilesView || isContentView" class="row-meta">
              {{ formatEntryType(entry) }}
              <template v-if="entry.kind !== 'dir'"> · {{ formatSize(entry) }}</template>
            </span>
          </span>
          <span v-if="isDetailsView" class="type">{{ formatEntryType(entry) }}</span>
          <span v-if="isDetailsView" class="size">{{ formatSize(entry) }}</span>
          <span v-if="isDetailsView" class="modified">{{ formatModified(entry) }}</span>
        </div>
        <div
          v-if="showSecondaryVirtSpacers && secondaryVirtual.offsetAfterPx > 0"
          class="virt-spacer"
          aria-hidden="true"
          :style="{ height: `${secondaryVirtual.offsetAfterPx}px` }"
        />
        </template>
      </div>
    </div>
    </div>
    <PreviewPane
      :enabled="previewPaneEnabled"
      :path="previewPath"
      @close="closePreviewPane"
    />
    <DetailsPane
      :enabled="detailsPaneOpen"
      :paths="selectedPaths"
      :entries="visibleEntries"
      @close="closeDetailsPane"
    />
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
      @attributes-changed="refreshListing"
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
  /* AIA-252: if host is still narrower than 2×floor, scroll instead of crush. */
  overflow-x: auto;
}

.panes--dual .pane {
  /*
   * AIA-252 hard floor (= DUAL_PANE_MIN_PANE_PX / 15rem).
   * Do not min() with a % — that let panes shrink below usable width.
   */
  min-width: 15rem;
  flex: 1 1 15rem;
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

.pane-list-surface {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
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

.virt-spacer {
  flex: 0 0 auto;
  width: 100%;
  pointer-events: none;
  grid-column: 1 / -1;
}

.list--list .row {
  grid-template-columns: 2rem 1fr;
}

.list--details .list-header,
.list--details .row {
  grid-template-columns: 2rem minmax(0, 1fr) 7.5rem 5.5rem 9.5rem;
}

.list--tiles {
  display: grid;
  align-content: start;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-sm);
  grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr));
}

.list--content {
  display: flex;
  flex-direction: column;
}

/* 分组窗口化启用时强制纵向流，使变高 spacer 与滚动位置一致。 */
.list--virt-flat.list--icons,
.list--virt-flat.list--tiles {
  display: flex;
  flex-direction: column;
  align-content: stretch;
  grid-template-columns: none;
}

.list-group-header {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-sm) var(--fluent-space-sm);
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-secondary);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  grid-column: 1 / -1;
  flex: 0 0 auto;
  width: 100%;
}

.list-group-count {
  color: var(--fluent-color-text-tertiary, var(--fluent-color-text-secondary));
}

.list--icons {
  display: grid;
  align-content: start;
  /* 列间保留间距；行间无纯空白条，行高由格子自身占满。 */
  column-gap: var(--fluent-space-sm);
  row-gap: 0;
  padding: var(--fluent-space-sm);
}

.list--icons-extra-large {
  grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
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
  min-width: 0;
  height: calc(var(--nav-icon-size, 16px) + var(--nav-icon-caption-extra, 64px));
  min-height: calc(var(--nav-icon-size, 16px) + var(--nav-icon-caption-extra, 64px));
  padding: var(--fluent-space-sm);
  border-bottom: 0;
  border-radius: var(--fluent-radius-md);
  text-align: center;
  overflow: hidden;
}

.list--icons .row--icon .name-block {
  width: 100%;
  max-width: 100%;
  min-width: 0;
  flex: 1 1 auto;
  min-height: 0;
  overflow: hidden;
  justify-content: flex-start;
  align-items: center;
}

.list--icons .row--icon .name {
  width: 100%;
  max-width: 100%;
  white-space: normal;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
  overflow-wrap: anywhere;
  line-height: 1.25;
  font-size: var(--fluent-font-size-caption);
}

.list--icons .row--icon .inline-rename {
  width: auto;
  max-width: 100%;
  /* 改名时最小与图标同宽，短文件名不缩成窄条。 */
  min-width: var(--nav-icon-size, 16px);
  margin-inline: auto;
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
  width: var(--nav-icon-size, 16px);
  height: var(--nav-icon-size, 16px);
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
  position: relative;
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

.col-resizer {
  position: absolute;
  top: 0;
  right: -3px;
  width: 6px;
  height: 100%;
  cursor: col-resize;
  z-index: 2;
}

.col-name {
  grid-column: 2;
}

.col-type {
  grid-column: 3;
  text-align: left;
}

.col-size {
  grid-column: 4;
  text-align: right;
}

.col-modified {
  grid-column: 5;
  text-align: left;
}

.row .type {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.row--tile,
.row--content {
  display: grid;
  grid-template-columns: auto 1fr;
  align-items: center;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-sm);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.list--tiles .row--tile {
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  min-height: 4.5rem;
}

.row-meta {
  display: block;
  margin-top: 0.15rem;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
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

/* 剪切态：图标与名称半透明，近似资源管理器待剪切外观。 */
.row--cut .kind,
.row--cut .name-block {
  opacity: 0.5;
}

.row--drop {
  outline: 2px solid var(--fluent-color-stroke-focus);
  outline-offset: -2px;
}

.list--drop {
  outline: 2px dashed var(--fluent-color-stroke-focus);
  outline-offset: -4px;
}

.kind {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--nav-icon-size, 16px);
  height: var(--nav-icon-size, 16px);
  flex-shrink: 0;
  color: var(--fluent-color-text-secondary);
}

.kind[data-kind="dir"] {
  color: var(--fluent-color-fill-accent);
}

/* Keep FluentIcon fallback at view size until shell icons arrive (AIA-267). */
.kind :deep(.fluent-icon) {
  width: var(--nav-icon-size, 16px);
  height: var(--nav-icon-size, 16px);
  font-size: var(--nav-icon-size, 16px);
}

.shell-icon {
  width: var(--nav-icon-size, 16px);
  height: var(--nav-icon-size, 16px);
  object-fit: contain;
  flex-shrink: 0;
}

.row--placeholder {
  pointer-events: none;
}

.kind--placeholder,
.name--placeholder,
.row-meta--placeholder,
.type--placeholder,
.size--placeholder,
.modified--placeholder {
  display: block;
  border-radius: var(--fluent-radius-sm);
  background: color-mix(in srgb, var(--fluent-color-text-secondary) 16%, transparent);
}

.kind--placeholder {
  width: var(--nav-icon-size, 16px);
  height: var(--nav-icon-size, 16px);
}

.name--placeholder {
  width: min(12rem, 70%);
  height: 0.85rem;
  margin-block: 0.2rem;
}

.list--icons .name--placeholder {
  width: 80%;
  margin-inline: auto;
}

.row-meta--placeholder {
  width: min(9rem, 55%);
  height: 0.7rem;
  margin-top: 0.25rem;
}

.type--placeholder,
.size--placeholder,
.modified--placeholder {
  width: 70%;
  height: 0.75rem;
  margin-block: 0.35rem;
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
  /* 覆盖 fluent-control 的标准控件高度/内边距，使边框紧贴文字。 */
  min-width: 0;
  width: max-content;
  max-width: 100%;
  min-height: 0;
  height: auto;
  box-sizing: border-box;
  padding: 0 2px;
  border-radius: 2px;
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
  field-sizing: content;
  user-select: text;
  caret-color: var(--fluent-color-text-primary);
  cursor: text;
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
