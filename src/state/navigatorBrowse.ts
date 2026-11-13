// SPDX-License-Identifier: GPL-3.0-or-later

import { ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  activeStripTab,
  activeWorkGroupId,
  applyLiveOpenState,
  cancelOpenStatePersist,
  flushOpenStatePersist,
  hydrateWorkGroups,
  liveOpenState,
  mergePinnedIntoOpenState,
  normalizeRestoredOpenState,
  openStateAfterCloseAllGroups,
  openStateAfterCloseGroup,
  openStateAfterCloseGroupsToRight,
  openStateAfterCloseOtherGroups,
  openStateAfterNavigate,
  openStateAfterReorderGroups,
  openStateAfterSwitchAdjacent,
  openStateAfterSwitchGroup,
  openStateEnsureMemberPaths,
  openStateFromPaths,
  openStateOpenOrFocusPath,
  openStateWithNewTab,
  rememberHydratedWorkGroups,
  scheduleOpenStatePersist,
  snapshotLeavingOpenState,
  type SwitchAdjacentDirection,
  type UserWorkGroupsDoc,
  type WorkGroupOpenState,
} from "./openState/index.ts";
import { navigableParentPath, recordNavHistory } from "./navHistory.ts";
import { GUIDE_PATH, isGuidePath } from "./guide/index.ts";
import { isRecyclePath, openRecycleBinInSystem } from "./recycle/index.ts";
import {
  isBlockedSearchProvider,
  isProviderPath,
  loadProviderListing,
  openProviderEntry,
  toProviderUri,
  type ProviderQueryParam,
} from "./provider/index.ts";
import { readNewTabBehavior } from "./newTabBehavior.ts";
import { hydrateSettings } from "./settingsHydrate.ts";
import { recordPathVisit } from "./visitStats.ts";
import { clearNavigatorFilter } from "./navigatorFilter.ts";
import { showHiddenItems } from "./showHidden/index.ts";
import { forgetPinnedTabs, pinnedIdSet } from "./tabPins.ts";
import { clearSelection } from "./navigatorSelection.ts";
import { normalizePathKey, toNativeFsPath } from "./pathKey.ts";
import { perfSpan } from "./perfProbe.ts";
import {
  getCachedListing,
  invalidateListingCache,
  isListingCacheFresh,
  listingCacheKey,
  putCachedListing,
} from "./navigatorListingCache.ts";

export {
  activeWorkGroupId,
  applyLiveOpenState,
  cancelOpenStatePersist,
  flushOpenStatePersist,
  hydrateWorkGroups,
  liveOpenState,
  rememberHydratedWorkGroups,
  snapshotLeavingOpenState,
  snapshotLiveOpenState,
  stripTabs,
  type UserWorkGroupsDoc,
} from "./openState/index.ts";

export type NavigatorEntry = {
  name: string;
  path: string;
  kind: "dir" | "file" | string;
  size: number;
  /** Last write time as Unix epoch milliseconds when reported. */
  modifiedMs?: number | null;
};

export type NavigatorListing = {
  path: string;
  entries: NavigatorEntry[];
  dirCount: number;
  fileCount: number;
  requiredQuery?: ProviderQueryParam[] | null;
  error?: string | null;
};

/** Wire shape: `path` may be omitted when equal to join(listing.path, name). */
type NavigatorEntryWire = Omit<NavigatorEntry, "path"> & { path?: string | null };

type NavigatorListingWire = Omit<NavigatorListing, "entries"> & {
  entries: NavigatorEntryWire[];
};

/** Reconstruct `parent\name` for entries that omitted `path` (Phase C IPC trim). */
export function joinListingChildPath(parent: string, name: string): string {
  const native = toNativeFsPath(parent);
  if (!native) return name;
  if (isProviderPath(native)) {
    const uri = toProviderUri(native);
    return `${uri.replace(/\/+$/, "")}/${name}`;
  }
  if (/^[a-zA-Z]:\\?$/i.test(native)) {
    return `${native.slice(0, 2)}\\${name}`;
  }
  return `${native.replace(/\\+$/, "")}\\${name}`;
}

/** Fill omitted entry paths after `navigator_list` / search-shaped listings. */
export function hydrateNavigatorListing(listing: NavigatorListingWire): NavigatorListing {
  const parent = listing.path;
  return {
    path: listing.path,
    dirCount: listing.dirCount,
    fileCount: listing.fileCount,
    requiredQuery: listing.requiredQuery,
    error: listing.error,
    entries: listing.entries.map((entry) => {
      const trimmed = entry.path?.trim() ?? "";
      return {
        name: entry.name,
        kind: entry.kind,
        size: entry.size,
        modifiedMs: entry.modifiedMs,
        path: trimmed || joinListingChildPath(parent, entry.name),
      };
    }),
  };
}

export const currentPath = ref("");
export const listing = shallowRef<NavigatorListing | null>(null);
export const listError = ref<string | null>(null);
export const listLoading = ref(false);

/** 副窗格路径与 listing（会话编排；布局开关仍在 dualPane）。 */
export const secondaryPath = ref("");
export const secondaryListing = shallowRef<NavigatorListing | null>(null);
export const secondaryError = ref<string | null>(null);
export const secondaryLoading = ref(false);

let bootstrapPromise: Promise<void> | null = null;
/**
 * AIA-225: bump on every navigateTo so rapid tab-group switches drop stale
 * `navigator_list` results instead of painting the wrong folder.
 */
let browseEpoch = 0;

/**
 * Restore tab strip immediately; directory listing fills asynchronously
 * so chrome / tabs paint before `navigator_list` returns (AIA-181 P0).
 */
export function bootstrapNavigatorOpenState(): Promise<void> {
  if (!bootstrapPromise) {
    bootstrapPromise = (async () => {
      try {
        const doc = await hydrateWorkGroups();
        activeWorkGroupId.value = doc.activeWorkGroupId;
        const active = doc.workGroups.find((g) => g.id === doc.activeWorkGroupId);
        applyLiveOpenState(active?.openState ?? null);
        const activeTab = activeStripTab(liveOpenState.value);
        if (activeTab?.path) {
          // Show restored path + loading skeleton; do not block bootstrap on listing.
          currentPath.value = activeTab.path;
          listLoading.value = true;
          listError.value = null;
          void navigateTo(activeTab.path, { syncOpenState: false });
        } else if (liveOpenState.value.tabGroups.length === 0) {
          void (async () => {
            await loadDefaultPath();
            await flushOpenStatePersist();
          })();
        }
      } catch {
        void (async () => {
          await loadDefaultPath();
          await flushOpenStatePersist();
        })();
      }
    })();
  }
  return bootstrapPromise;
}

function normalizeBrowseKey(path: string): string {
  return normalizePathKey(path);
}

/**
 * AIA-225: paint tab-group content chrome immediately; fill the listing off the
 * critical path (same tabs-first pattern as bootstrapNavigatorOpenState).
 * Open-state persist stays debounced so disk IPC does not contend with list.
 * AIA-259: drop stale entries when the target path changes so the prior WG/tab
 * folder does not keep painting under the new strip.
 */
/** Paint chrome immediately; return the listing promise for optional perf spans. */
function revealPathAsync(path: string): Promise<void> {
  const trimmed = path.trim();
  if (!trimmed) return Promise.resolve();
  // 回收站标签：走系统资源管理器，避免慢路径枚举把切标签拖死。
  if (isRecyclePath(trimmed)) {
    void openRecycleBinInSystem();
    if (isRecyclePath(currentPath.value)) {
      return navigateTo(GUIDE_PATH, { syncOpenState: false, recordHistory: false });
    }
    return Promise.resolve();
  }
  const priorKey = normalizeBrowseKey(listing.value?.path ?? currentPath.value);
  currentPath.value = trimmed;
  listError.value = null;
  const cacheKey = listingCacheKey(trimmed, showHiddenItems.value);
  const cached = !isGuidePath(trimmed) && !isProviderPath(trimmed)
    ? getCachedListing<NavigatorListing>(cacheKey)
    : null;
  if (cached) {
    // FIL-58：切标签先粉缓存，避免大目录空白等待。
    listing.value = cached;
    listLoading.value = !isListingCacheFresh(cacheKey);
  } else {
    listLoading.value = true;
    if (priorKey !== normalizeBrowseKey(trimmed)) {
      listing.value = null;
    }
  }
  if (isGuidePath(trimmed)) {
    // Guide has no async list — apply fully via navigateTo without blocking callers.
    return navigateTo(GUIDE_PATH, { syncOpenState: false, recordHistory: false });
  }
  return navigateTo(trimmed, { syncOpenState: false, recordHistory: false });
}

function revealPathNonBlocking(path: string): void {
  void revealPathAsync(path);
}

/** Fire-and-forget tab/WG reveal timed as `tab_switch` when perf probe is on. */
function revealPathForTabSwitch(path: string): void {
  void perfSpan("tab_switch", () => revealPathAsync(path), () => ({
    path: currentPath.value,
    n: listing.value?.entries.length ?? 0,
  }));
}

/**
 * AIA-238: re-list the active tab folder without blocking UI, history, or visit stats.
 * Used by F5, toolbar Refresh, and window-focus sync after external Explorer changes.
 */
export function refreshActiveListing(): void {
  const path = currentPath.value.trim();
  if (!path) return;
  invalidateListingCache(path);
  void navigateTo(path, {
    syncOpenState: false,
    recordHistory: false,
    recordVisit: false,
    bypassListingCache: true,
  });
}

/**
 * FIL-60：粘贴/删除/改名等变更后强制重列（作废缓存，与 F5 同路径）。
 * 可 await，便于后续选中；调用方按需刷新副窗格。
 */
export async function reloadListingAfterMutation(options?: {
  includeSecondary?: boolean;
}): Promise<void> {
  const path = currentPath.value.trim();
  if (!path) return;
  invalidateListingCache(path);
  await navigateTo(path, {
    syncOpenState: false,
    recordHistory: false,
    recordVisit: false,
    bypassListingCache: true,
  });
  if (!options?.includeSecondary) return;
  const secondary = secondaryPath.value.trim();
  if (!secondary) return;
  invalidateListingCache(secondary);
  await navigateSecondary(secondary);
}

/** 剪切/移动后作废源目录 listing 缓存，避免回访仍显示已移走项。 */
export function invalidateListingParents(paths: string[]): void {
  for (const path of paths) {
    const parent = navigableParentPath(path);
    if (parent) invalidateListingCache(parent);
  }
}

/**
 * 删除成功后先从当前 listing 去掉条目，避免大目录删除阻塞期间界面仍显示旧项。
 * 随后仍应调用 refreshListing / navigateTo 与磁盘对齐。
 */
export function removePathsFromListings(paths: string[]): void {
  const doomed = new Set(
    paths.map((path) => normalizePathKey(path)).filter((key) => key.length > 0),
  );
  if (!doomed.size) return;

  const strip = (current: NavigatorListing | null): NavigatorListing | null => {
    if (!current) return current;
    const entries = current.entries.filter(
      (entry) => !doomed.has(normalizePathKey(entry.path)),
    );
    if (entries.length === current.entries.length) return current;
    let dirCount = 0;
    let fileCount = 0;
    for (const entry of entries) {
      if (entry.kind === "dir") dirCount += 1;
      else fileCount += 1;
    }
    return { ...current, entries, dirCount, fileCount };
  };

  listing.value = strip(listing.value);
  secondaryListing.value = strip(secondaryListing.value);
  if (listing.value) {
    putCachedListing(
      listingCacheKey(listing.value.path, showHiddenItems.value),
      listing.value,
    );
  }
  if (secondaryListing.value) {
    putCachedListing(
      listingCacheKey(secondaryListing.value.path, showHiddenItems.value),
      secondaryListing.value,
    );
  }
}

/** 侧栏目录树在文件变更后递增，展开节点会重新拉取子项。 */
export const folderTreeRevision = ref(0);

export function bumpFolderTreeRevision(): void {
  folderTreeRevision.value += 1;
}

/** 副窗格 listing 导航（不写主 Open State / 历史栈）。 */
export async function navigateSecondary(path: string): Promise<void> {
  const trimmed = path.trim();
  if (isProviderPath(trimmed) && isBlockedSearchProvider(trimmed)) {
    secondaryLoading.value = false;
    secondaryError.value = "PROVIDER_SEARCH_BLOCKED";
    return;
  }
  const previous = secondaryPath.value;
  secondaryLoading.value = true;
  secondaryError.value = null;
  try {
    let entryCount = 0;
    const result = await perfSpan(
      "navigator_list",
      async () => {
        if (isProviderPath(trimmed)) {
          const hydrated = await loadProviderListing(trimmed);
          entryCount = hydrated.entries.length;
          return hydrated;
        }
        const listingResult = await invoke<NavigatorListingWire>("navigator_list", {
          path: trimmed,
          showHidden: showHiddenItems.value,
        });
        const hydrated = hydrateNavigatorListing(listingResult);
        entryCount = hydrated.entries.length;
        return hydrated;
      },
      () => ({ path: trimmed, n: entryCount, pane: "secondary" }),
    );
    secondaryListing.value = result;
    if (!isProviderPath(result.path)) {
      putCachedListing(listingCacheKey(result.path, showHiddenItems.value), result);
    }
    secondaryPath.value = result.path;
    secondaryError.value = result.error?.trim() ? result.error : null;
    if (normalizeBrowseKey(previous) !== normalizeBrowseKey(result.path)) {
      clearNavigatorFilter();
    }
  } catch (err) {
    secondaryListing.value = null;
    secondaryError.value = err instanceof Error ? err.message : String(err);
    secondaryPath.value = trimmed;
  } finally {
    secondaryLoading.value = false;
  }
}

export async function navigateTo(
  path: string,
  options?: {
    syncOpenState?: boolean;
    recordHistory?: boolean;
    /** Default true. Focus/F5 refresh passes false so revisit stats stay clean (AIA-238). */
    recordVisit?: boolean;
    /** F5 / 焦点刷新时跳过新鲜缓存短路。 */
    bypassListingCache?: boolean;
  },
): Promise<void> {
  const trimmed = path.trim();
  const syncOpenState = options?.syncOpenState !== false;
  // Tab switch / bootstrap pass syncOpenState:false — don't pollute back stack.
  const recordHistory = options?.recordHistory ?? syncOpenState;
  const recordVisit = options?.recordVisit !== false;
  const bypassListingCache = options?.bypassListingCache === true;
  if (isProviderPath(trimmed) && isBlockedSearchProvider(trimmed)) {
    listLoading.value = false;
    listError.value = "PROVIDER_SEARCH_BLOCKED";
    return;
  }
  const previous = currentPath.value;
  const pathChanged = normalizeBrowseKey(previous) !== normalizeBrowseKey(trimmed);
  const epoch = ++browseEpoch;

  if (isGuidePath(trimmed)) {
    if (epoch !== browseEpoch) return;
    listing.value = null;
    listError.value = null;
    listLoading.value = false;
    // 已在引导页：跳过 Open State 克隆与写盘，避免路径栏误点根面包屑时卡顿。
    if (!pathChanged && isGuidePath(previous)) {
      currentPath.value = GUIDE_PATH;
      return;
    }
    currentPath.value = GUIDE_PATH;
    if (pathChanged) clearNavigatorFilter();
    if (recordHistory) {
      recordNavHistory(previous, GUIDE_PATH);
    }
    if (syncOpenState) {
      liveOpenState.value = openStateAfterNavigate(liveOpenState.value, GUIDE_PATH);
      const group = liveOpenState.value.tabGroups[liveOpenState.value.currentTabGroupIndex];
      const pane = group?.[liveOpenState.value.currentTabIndex] ?? group?.[0];
      if (pane) {
        pane.name = "Guide";
        pane.type = "guide";
      }
      scheduleOpenStatePersist(false);
    }
    return;
  }

  if (isRecyclePath(trimmed)) {
    // FIL-52：默认用 Explorer/`shell:RecycleBinFolder` 打开，不再在 AFM 内枚举整仓。
    if (epoch !== browseEpoch) return;
    listLoading.value = false;
    try {
      const result = await openRecycleBinInSystem();
      if (epoch !== browseEpoch) return;
      if (!result.success && result.error) {
        listError.value = result.error;
      } else {
        listError.value = null;
      }
    } catch (err) {
      if (epoch !== browseEpoch) return;
      listError.value = err instanceof Error ? err.message : String(err);
    }
    return;
  }

  if (isProviderPath(trimmed)) {
    const target = toProviderUri(trimmed);
    listLoading.value = true;
    listError.value = null;
    try {
      const result = await loadProviderListing(target);
      if (epoch !== browseEpoch) return;
      listing.value = result;
      listError.value = result.error?.trim() ? result.error : null;
      currentPath.value = result.path;
      if (pathChanged) clearNavigatorFilter();
      if (recordVisit) {
        void recordPathVisit(result.path, true);
      }
      if (recordHistory) {
        recordNavHistory(previous, result.path);
      }
      if (syncOpenState) {
        liveOpenState.value = openStateAfterNavigate(liveOpenState.value, result.path);
        const group = liveOpenState.value.tabGroups[liveOpenState.value.currentTabGroupIndex];
        const pane = group?.[liveOpenState.value.currentTabIndex] ?? group?.[0];
        if (pane) {
          pane.type = "provider";
        }
        scheduleOpenStatePersist(false);
      }
    } catch (err) {
      if (epoch !== browseEpoch) return;
      listing.value = null;
      listError.value = err instanceof Error ? err.message : String(err);
      currentPath.value = target;
      if (pathChanged) clearNavigatorFilter();
      if (recordHistory) {
        recordNavHistory(previous, target);
      }
      if (syncOpenState && target) {
        liveOpenState.value = openStateAfterNavigate(liveOpenState.value, target);
        scheduleOpenStatePersist(false);
      }
    } finally {
      if (epoch === browseEpoch) {
        listLoading.value = false;
      }
    }
    return;
  }

  listError.value = null;
  const diskCacheKey = listingCacheKey(trimmed, showHiddenItems.value);
  const warm = !bypassListingCache
    ? getCachedListing<NavigatorListing>(diskCacheKey)
    : null;
  if (warm) {
    listing.value = warm;
    currentPath.value = warm.path;
    if (pathChanged) clearNavigatorFilter();
    if (isListingCacheFresh(diskCacheKey)) {
      // FIL-58：新鲜缓存命中则跳过 IPC，切标签不阻塞。
      listLoading.value = false;
      if (recordVisit) {
        void recordPathVisit(warm.path, true);
      }
      if (recordHistory) {
        recordNavHistory(previous, warm.path);
      }
      if (syncOpenState) {
        liveOpenState.value = openStateAfterNavigate(liveOpenState.value, warm.path);
        scheduleOpenStatePersist(false);
      }
      return;
    }
  }

  listLoading.value = true;
  try {
    let entryCount = 0;
    const result = await perfSpan(
      "navigator_list",
      async () => {
        const listingResult = await invoke<NavigatorListingWire>("navigator_list", {
          path: trimmed,
          showHidden: showHiddenItems.value,
        });
        const hydrated = hydrateNavigatorListing(listingResult);
        entryCount = hydrated.entries.length;
        return hydrated;
      },
      () => ({ path: trimmed, n: entryCount }),
    );
    if (epoch !== browseEpoch) return;
    listing.value = result;
    putCachedListing(listingCacheKey(result.path, showHiddenItems.value), result);
    currentPath.value = result.path;
    if (normalizeBrowseKey(previous) !== normalizeBrowseKey(result.path)) {
      clearNavigatorFilter();
    }
    // AIA-74: count directory landings (tab enter + folder nav share this path).
    if (recordVisit) {
      void recordPathVisit(result.path, true);
    }
    if (recordHistory) {
      recordNavHistory(previous, result.path);
    }
    if (syncOpenState) {
      liveOpenState.value = openStateAfterNavigate(liveOpenState.value, result.path);
      scheduleOpenStatePersist(false);
    }
  } catch (err) {
    if (epoch !== browseEpoch) return;
    if (!listing.value) {
      listing.value = null;
    }
    listError.value = err instanceof Error ? err.message : String(err);
    currentPath.value = trimmed;
    if (pathChanged) clearNavigatorFilter();
    if (recordHistory) {
      recordNavHistory(previous, trimmed);
    }
    if (syncOpenState && trimmed) {
      liveOpenState.value = openStateAfterNavigate(liveOpenState.value, trimmed);
      scheduleOpenStatePersist(false);
    }
  } finally {
    if (epoch === browseEpoch) {
      listLoading.value = false;
    }
  }
}

export async function openPathsAsNavigatorTabs(paths: string[]): Promise<void> {
  const next = openStateFromPaths(paths);
  applyLiveOpenState(next, { persist: true, immediatePersist: false });
  const active = activeStripTab(liveOpenState.value);
  if (active?.path) {
    revealPathNonBlocking(active.path);
  }
}

export async function newNavigatorTab(path?: string): Promise<void> {
  let target: string;
  if (path != null && path.trim()) {
    target = path.trim();
  } else {
    const behavior = await readNewTabBehavior();
    if (behavior === "duplicate") {
      target = currentPath.value.trim();
      if (!target) {
        try {
          target = await invoke<string>("navigator_default_path");
        } catch {
          target = "C:/";
        }
      }
    } else {
      target = GUIDE_PATH;
    }
  }
  liveOpenState.value = openStateWithNewTab(liveOpenState.value, target);
  if (isGuidePath(target)) {
    const group = liveOpenState.value.tabGroups[liveOpenState.value.currentTabGroupIndex];
    const pane = group?.[0];
    if (pane) {
      pane.name = "Guide";
      pane.type = "guide";
    }
  }
  scheduleOpenStatePersist(false);
  revealPathNonBlocking(target);
}

export async function closeNavigatorTab(groupIndex: number): Promise<void> {
  const closing = liveOpenState.value.tabGroups[groupIndex] ?? [];
  forgetPinnedTabs(closing.map((tab) => tab.id));
  const next = openStateAfterCloseGroup(liveOpenState.value, groupIndex);
  liveOpenState.value = next;
  // Closing often activates another group — keep strip responsive (AIA-225).
  scheduleOpenStatePersist(false);
  const active = activeStripTab(next);
  if (active?.path) {
    revealPathNonBlocking(active.path);
  } else {
    void loadDefaultPath();
  }
}

/** 关闭除 keepIndex 以外的全部标签。 */
export async function closeOtherNavigatorTabs(keepIndex: number): Promise<void> {
  const prev = liveOpenState.value;
  if (
    !Number.isInteger(keepIndex) ||
    keepIndex < 0 ||
    keepIndex >= prev.tabGroups.length ||
    prev.tabGroups.length <= 1
  ) {
    return;
  }
  const closingIds = prev.tabGroups.flatMap((group, index) =>
    index === keepIndex ? [] : group.map((tab) => tab.id),
  );
  forgetPinnedTabs(closingIds);
  const next = openStateAfterCloseOtherGroups(prev, keepIndex);
  liveOpenState.value = next;
  scheduleOpenStatePersist(false);
  const active = activeStripTab(next);
  if (active?.path) {
    revealPathNonBlocking(active.path);
  }
}

/** 关闭 groupIndex 右侧的全部标签。 */
export async function closeNavigatorTabsToRight(groupIndex: number): Promise<void> {
  const prev = liveOpenState.value;
  if (
    !Number.isInteger(groupIndex) ||
    groupIndex < 0 ||
    groupIndex >= prev.tabGroups.length - 1
  ) {
    return;
  }
  const closingIds = prev.tabGroups
    .slice(groupIndex + 1)
    .flatMap((group) => group.map((tab) => tab.id));
  forgetPinnedTabs(closingIds);
  const next = openStateAfterCloseGroupsToRight(prev, groupIndex);
  liveOpenState.value = next;
  scheduleOpenStatePersist(false);
  const active = activeStripTab(next);
  if (active?.path) {
    revealPathNonBlocking(active.path);
  }
}

/** 关闭全部标签，并回落到默认路径。 */
export async function closeAllNavigatorTabs(): Promise<void> {
  const prev = liveOpenState.value;
  if (!prev.tabGroups.length) return;
  forgetPinnedTabs(prev.tabGroups.flatMap((group) => group.map((tab) => tab.id)));
  liveOpenState.value = openStateAfterCloseAllGroups(prev);
  scheduleOpenStatePersist(false);
  void loadDefaultPath();
}

export async function switchNavigatorTab(groupIndex: number): Promise<void> {
  // AIA-225: strip + path paint first; listing + OpenState disk write are deferred.
  liveOpenState.value = openStateAfterSwitchGroup(liveOpenState.value, groupIndex);
  scheduleOpenStatePersist(false);
  const active = activeStripTab(liveOpenState.value);
  if (active?.path) {
    revealPathForTabSwitch(active.path);
  }
}

/** Drag-reorder strip tabs (page groups) and persist Open State. */
export async function reorderNavigatorTabs(
  fromIndex: number,
  toIndex: number,
): Promise<void> {
  if (
    !Number.isInteger(fromIndex) ||
    !Number.isInteger(toIndex) ||
    fromIndex === toIndex
  ) {
    return;
  }
  const before = liveOpenState.value.tabGroups.length;
  const next = openStateAfterReorderGroups(
    liveOpenState.value,
    fromIndex,
    toIndex,
  );
  // Reorder must never change group count (guards against insert/duplication bugs).
  if (next.tabGroups.length !== before) return;
  liveOpenState.value = next;
  // Reorder does not change active listing; debounce disk write (AIA-225).
  scheduleOpenStatePersist(false);
}

/** Cycle page groups (Sigma Ctrl+Tab / Ctrl+Shift+Tab → switchToAdjacentTabGroup). */
export async function switchNavigatorTabAdjacent(
  direction: SwitchAdjacentDirection,
): Promise<void> {
  if (liveOpenState.value.tabGroups.length < 2) return;
  // AIA-225: same non-blocking reveal as click-to-switch.
  liveOpenState.value = openStateAfterSwitchAdjacent(liveOpenState.value, direction);
  scheduleOpenStatePersist(false);
  const active = activeStripTab(liveOpenState.value);
  if (active?.path) {
    revealPathForTabSwitch(active.path);
  }
}

/**
 * Select an existing page group for `path`, or open a new one (Sigma openOrFocusTabGroup).
 * Silent Open State persist — no WG nag modal.
 */
export async function openOrFocusNavigatorTab(path: string): Promise<void> {
  const trimmed = path.trim();
  if (!trimmed) return;
  // 侧栏/Places 进回收站：只拉起系统回收站，不新建 AFM 慢路径标签。
  if (isRecyclePath(trimmed)) {
    try {
      await openRecycleBinInSystem();
    } catch {
      /* 错误由调用方 toast 可选承接；此处不阻断 */
    }
    return;
  }
  const result = openStateOpenOrFocusPath(liveOpenState.value, trimmed);
  liveOpenState.value = result.state;
  scheduleOpenStatePersist(false);
  const active = activeStripTab(liveOpenState.value);
  if (active?.path) {
    revealPathNonBlocking(active.path);
  }
}

/**
 * Sigma-style soft sync: ensure each Work Group member path has a tab in the live Open State.
 * Appends missing members only; does not close existing tabs. Returns newly opened paths.
 */
export async function openMembersIntoOpenState(
  memberPaths: string[],
): Promise<string[]> {
  const { state, opened } = openStateEnsureMemberPaths(
    liveOpenState.value,
    memberPaths,
  );
  liveOpenState.value = state;
  scheduleOpenStatePersist(false);
  const active = activeStripTab(liveOpenState.value);
  if (active?.path) {
    revealPathNonBlocking(active.path);
  } else if (opened[0]) {
    revealPathNonBlocking(opened[0]);
  }
  return opened;
}

/** Apply Open State restored by Work Group silent switch / hard-member sync. */
export async function restoreWorkGroupOpenState(
  workGroupId: string,
  state: WorkGroupOpenState | null | undefined,
  options?: {
    /**
     * When false, only sync strip/ids (used to reconcile after an optimistic
     * paint when soft-sync did not change the active path). Default true.
     */
    reveal?: boolean;
  },
): Promise<void> {
  const reveal = options?.reveal !== false;
  const previousPath = currentPath.value;
  const preferredPin = activeStripTab(liveOpenState.value)?.id ?? null;
  const merged = mergePinnedIntoOpenState(
    normalizeRestoredOpenState(state),
    liveOpenState.value,
    pinnedIdSet(preferredPin),
  );
  activeWorkGroupId.value = workGroupId;
  // Strip restores synchronously; directory listing must not block WG switch UI.
  applyLiveOpenState(merged);
  clearSelection();
  const active = activeStripTab(liveOpenState.value);
  if (active?.path) {
    if (
      !reveal &&
      normalizeBrowseKey(active.path) === normalizeBrowseKey(previousPath)
    ) {
      return;
    }
    revealPathForTabSwitch(active.path);
  } else if (!liveOpenState.value.tabGroups.length) {
    listing.value = null;
    listLoading.value = false;
    currentPath.value = "";
  }
}

export type SoftSyncNotice = {
  kind: string;
  workGroupId: string;
  openedMemberPaths: string[];
};

export type SilentSwitchResult = {
  document: UserWorkGroupsDoc;
  openStateToRestore: WorkGroupOpenState | null;
  requireSavePrompt: boolean;
  relativeMemberPathsToOpen: string[];
  notice: SoftSyncNotice | null;
};

/** Apply Project activate → optional WG silent-switch into the shell strip. */
export async function applyProjectSilentSwitch(
  silentSwitch: SilentSwitchResult | null | undefined,
): Promise<SoftSyncNotice | null> {
  if (!silentSwitch) return null;
  if (silentSwitch.requireSavePrompt) {
    throw new Error("PROJECT_SILENT_SWITCH_SAVE_PROMPT");
  }
  await restoreWorkGroupOpenState(
    silentSwitch.document.activeWorkGroupId,
    silentSwitch.openStateToRestore,
  );
  return silentSwitch.notice;
}

async function rollbackSilentSwitchStrip(
  priorDoc: UserWorkGroupsDoc,
  leavingFallback: WorkGroupOpenState,
): Promise<void> {
  const prior = priorDoc.workGroups.find(
    (group) => group.id === priorDoc.activeWorkGroupId,
  );
  await restoreWorkGroupOpenState(
    priorDoc.activeWorkGroupId,
    prior?.openState ?? leavingFallback,
  );
}

/**
 * WG 静默切换会话编排（grill #1）：乐观 strip → IPC → 调和。
 * chrome 只负责 busy/error/notice 与文档展示缓存。
 */
export async function silentSwitchWorkGroup(
  targetId: string,
  priorDoc: UserWorkGroupsDoc | null,
): Promise<{ result: SilentSwitchResult | null; error: string | null }> {
  if (!targetId || targetId === priorDoc?.activeWorkGroupId) {
    return { result: null, error: null };
  }
  cancelOpenStatePersist();
  const leaving = snapshotLeavingOpenState();
  const cachedTarget = priorDoc?.workGroups.find((group) => group.id === targetId);
  let paintedOptimistic = false;
  if (priorDoc && cachedTarget) {
    activeWorkGroupId.value = targetId;
    await restoreWorkGroupOpenState(targetId, cachedTarget.openState);
    paintedOptimistic = true;
  }
  try {
    const result = await invoke<SilentSwitchResult>("work_groups_silent_switch", {
      targetWorkGroupId: targetId,
      liveOpenState: leaving,
    });
    if (result.requireSavePrompt) {
      if (paintedOptimistic && priorDoc) {
        await rollbackSilentSwitchStrip(priorDoc, leaving);
      }
      return { result, error: "REQUIRE_SAVE_PROMPT" };
    }
    rememberHydratedWorkGroups(result.document);
    await restoreWorkGroupOpenState(
      result.document.activeWorkGroupId,
      result.openStateToRestore,
      { reveal: !paintedOptimistic },
    );
    return { result, error: null };
  } catch (err) {
    if (paintedOptimistic && priorDoc) {
      try {
        await rollbackSilentSwitchStrip(priorDoc, leaving);
      } catch {
        /* keep switch error */
      }
    }
    return {
      result: null,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

export async function loadDefaultPath(): Promise<void> {
  listLoading.value = true;
  listError.value = null;
  try {
    const path = await invoke<string>("navigator_default_path");
    await navigateTo(path);
  } catch (err) {
    listLoading.value = false;
    listError.value = err instanceof Error ? err.message : String(err);
  }
}

export async function goUp(): Promise<void> {
  if (!currentPath.value || isGuidePath(currentPath.value)) return;
  if (isRecyclePath(currentPath.value)) {
    await navigateTo(GUIDE_PATH);
    return;
  }
  // Breadcrumb-aligned parent (drive / UNC share root → guide). Prefer pure
  // path logic so Up matches the This PC crumb even when IPC parent is null.
  // Provider `awam://` also resolves here (backend root → Providers root → guide).
  const logical = navigableParentPath(currentPath.value);
  if (logical) {
    await navigateTo(logical);
    return;
  }
  if (isProviderPath(currentPath.value)) {
    await navigateTo(GUIDE_PATH);
    return;
  }
  const parent = await invoke<string | null>("navigator_parent", {
    path: currentPath.value,
  });
  if (parent) {
    await navigateTo(parent);
    return;
  }
  // Last resort: Kernel root with no FS parent → guide landing.
  await navigateTo(GUIDE_PATH);
}

export type FileDoubleClickAction =
  | "systemDefault"
  | "openWithDialog"
  | "reveal"
  | "vscode"
  | "cursor"
  | "trae"
  | "qoder"
  | "none";

/**
 * Activate a Navigator listing entry (double-click / Enter).
 * Directories navigate in the *current* tab (`navigateTo`); external places
 * (Discovery / Favorites / Recent / Home) still call `openOrFocusNavigatorTab`.
 * Files follow Settings `core.fileDoubleClickAction`.
 */
async function openPathWithSystemDefault(path: string): Promise<void> {
  const result = await invoke<{ success: boolean; error: string | null }>(
    "open_path_default",
    { path },
  );
  if (!result.success) {
    throw new Error(result.error ?? "Failed to open path");
  }
}

export async function openEntry(entry: NavigatorEntry): Promise<void> {
  if (entry.kind === "dir") {
    // AIA-150: in-list folder activate stays in the current tab (context Open
    // already did; open-or-focus remains for shell places / Discovery / etc.).
    await navigateTo(entry.path);
    return;
  }
  if (isProviderPath(entry.path) || isProviderPath(currentPath.value)) {
    await openProviderEntry(entry.path);
    return;
  }
  // Non-dir entries (file / symlink-as-file) follow double-click Settings.
  const action = await readFileDoubleClickAction();
  switch (action) {
    case "none":
      return;
    case "openWithDialog": {
      const result = await invoke<{ success: boolean; error: string | null }>(
        "open_native_open_with_dialog",
        { path: entry.path },
      );
      if (!result.success && result.error) {
        throw new Error(result.error);
      }
      return;
    }
    case "reveal":
      await revealItemInDir(entry.path);
      return;
    case "vscode":
    case "cursor":
    case "trae":
    case "qoder": {
      const result = await invoke<{ success: boolean; error: string | null }>(
        "ide_open_path",
        { ideId: action, path: entry.path },
      );
      if (!result.success && result.error) {
        throw new Error(result.error);
      }
      return;
    }
    case "systemDefault":
    default:
      await openPathWithSystemDefault(entry.path);
  }
}

async function readFileDoubleClickAction(): Promise<FileDoubleClickAction> {
  const allowed: FileDoubleClickAction[] = [
    "systemDefault",
    "openWithDialog",
    "reveal",
    "vscode",
    "cursor",
    "trae",
    "qoder",
    "none",
  ];
  try {
    const doc = await hydrateSettings();
    const raw = doc.values["core.fileDoubleClickAction"];
    if (typeof raw === "string" && (allowed as string[]).includes(raw)) {
      return raw as FileDoubleClickAction;
    }
  } catch {
    /* fall through */
  }
  return "systemDefault";
}

