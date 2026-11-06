// SPDX-License-Identifier: GPL-3.0-or-later

import { computed, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  activeStripTab,
  createEmptyOpenState,
  normalizeRestoredOpenState,
  openStateAfterCloseGroup,
  openStateAfterNavigate,
  openStateAfterReorderGroups,
  openStateAfterSwitchAdjacent,
  openStateAfterSwitchGroup,
  openStateEnsureMemberPaths,
  openStateFromPaths,
  openStateOpenOrFocusPath,
  openStateWithNewTab,
  stripTabsFromOpenState,
  mergePinnedIntoOpenState,
  openStateWithoutPinned,
  type SwitchAdjacentDirection,
  type WorkGroupOpenState,
} from "./openStateModel";
import { navigableParentPath, recordNavHistory } from "./navHistory";
import { GUIDE_PATH, isGuidePath } from "./navigatorGuide";
import { isRecyclePath, RECYCLE_PATH } from "./recyclePath";
import { loadRecycleListing } from "./recycleBin";
import { readNewTabBehavior } from "./newTabBehavior";
import { hydrateSettings } from "./settingsHydrate";
import { recordPathVisit } from "./visitStats";
import { clearNavigatorFilter } from "./navigatorFilter";
import { showHiddenItems } from "./showHidden";
import { forgetPinnedTabs, pinnedTabIds } from "./tabPins";

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
};

export type UserWorkGroupsDoc = {
  workGroups: Array<{
    id: string;
    name: string;
    isDefault: boolean;
    memberPaths: string[];
    openState: WorkGroupOpenState | null;
  }>;
  activeWorkGroupId: string;
};

export const currentPath = ref("");
export const listing = shallowRef<NavigatorListing | null>(null);
export const listError = ref<string | null>(null);
export const listLoading = ref(false);

/** Runtime Open State for the active Work Group (Navigator tab strip). */
export const liveOpenState = ref<WorkGroupOpenState>(createEmptyOpenState());
export const activeWorkGroupId = ref("");

export const stripTabs = computed(() => stripTabsFromOpenState(liveOpenState.value));

/** @deprecated Prefer `stripTabs` — kept for callers that only need paths. */
export const navigatorTabs = computed(() => stripTabs.value.map((tab) => tab.path));

let persistTimer: ReturnType<typeof setTimeout> | null = null;
let bootstrapPromise: Promise<void> | null = null;
/** Single-flight `work_groups_get` so Shell chrome + OpenState bootstrap share one IPC. */
let workGroupsHydrate: Promise<UserWorkGroupsDoc> | null = null;

function schedulePersist(immediate = false) {
  if (!activeWorkGroupId.value) return;
  if (persistTimer) {
    clearTimeout(persistTimer);
    persistTimer = null;
  }
  const run = () => {
    persistTimer = null;
    void persistOpenStateNow();
  };
  if (immediate) {
    run();
  } else {
    persistTimer = setTimeout(run, 280);
  }
}

export async function persistOpenStateNow(): Promise<void> {
  if (!activeWorkGroupId.value) return;
  try {
    await invoke<UserWorkGroupsDoc>("work_groups_save_open_state", {
      workGroupId: activeWorkGroupId.value,
      liveOpenState: snapshotLiveOpenState(),
    });
  } catch {
    // Persistence failures should not block browsing; WG UI can still save explicitly.
  }
}

/** Flush debounced Open State to disk (close / tray / quit / hide). */
export async function flushOpenStatePersist(): Promise<void> {
  if (persistTimer) {
    clearTimeout(persistTimer);
    persistTimer = null;
  }
  await persistOpenStateNow();
}

export function applyLiveOpenState(
  state: WorkGroupOpenState | null | undefined,
  options?: { persist?: boolean; immediatePersist?: boolean },
): void {
  liveOpenState.value = normalizeRestoredOpenState(state);
  if (options?.persist) {
    schedulePersist(options.immediatePersist ?? true);
  }
}

/**
 * Cold-start / concurrent `work_groups_get` bus (AIA-181 P0).
 * Pass `{ force: true }` when the menu needs a fresh disk read.
 */
export function hydrateWorkGroups(options?: {
  force?: boolean;
}): Promise<UserWorkGroupsDoc> {
  if (!options?.force && workGroupsHydrate) return workGroupsHydrate;
  const run = (async () => {
    const doc = await invoke<UserWorkGroupsDoc>("work_groups_get");
    activeWorkGroupId.value = doc.activeWorkGroupId;
    return doc;
  })();
  workGroupsHydrate = run.catch((err) => {
    if (workGroupsHydrate === run) workGroupsHydrate = null;
    throw err;
  });
  return workGroupsHydrate;
}

/** Keep hydrate cache aligned after silent switch / member save. */
export function rememberHydratedWorkGroups(doc: UserWorkGroupsDoc): void {
  activeWorkGroupId.value = doc.activeWorkGroupId;
  workGroupsHydrate = Promise.resolve(doc);
}

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
  return path.trim().replace(/\//g, "\\").replace(/\\+$/, "").toLowerCase();
}

export async function navigateTo(
  path: string,
  options?: { syncOpenState?: boolean; recordHistory?: boolean },
): Promise<void> {
  const trimmed = path.trim();
  const syncOpenState = options?.syncOpenState !== false;
  // Tab switch / bootstrap pass syncOpenState:false — don't pollute back stack.
  const recordHistory = options?.recordHistory ?? syncOpenState;
  const previous = currentPath.value;
  const pathChanged = normalizeBrowseKey(previous) !== normalizeBrowseKey(trimmed);

  if (isGuidePath(trimmed)) {
    listing.value = null;
    listError.value = null;
    listLoading.value = false;
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
      schedulePersist(false);
    }
    return;
  }

  if (isRecyclePath(trimmed)) {
    listLoading.value = true;
    listError.value = null;
    try {
      listing.value = await loadRecycleListing();
      currentPath.value = RECYCLE_PATH;
      if (pathChanged) clearNavigatorFilter();
      if (recordHistory) {
        recordNavHistory(previous, RECYCLE_PATH);
      }
      if (syncOpenState) {
        liveOpenState.value = openStateAfterNavigate(liveOpenState.value, RECYCLE_PATH);
        const group = liveOpenState.value.tabGroups[liveOpenState.value.currentTabGroupIndex];
        const pane = group?.[liveOpenState.value.currentTabIndex] ?? group?.[0];
        if (pane) {
          pane.name = "Recycle Bin";
          pane.type = "recycle";
        }
        schedulePersist(false);
      }
    } catch (err) {
      listing.value = null;
      listError.value = err instanceof Error ? err.message : String(err);
      currentPath.value = RECYCLE_PATH;
      if (pathChanged) clearNavigatorFilter();
    } finally {
      listLoading.value = false;
    }
    return;
  }

  listLoading.value = true;
  listError.value = null;
  try {
    const result = await invoke<NavigatorListing>("navigator_list", {
      path: trimmed,
      showHidden: showHiddenItems.value,
    });
    listing.value = result;
    currentPath.value = result.path;
    if (normalizeBrowseKey(previous) !== normalizeBrowseKey(result.path)) {
      clearNavigatorFilter();
    }
    // AIA-74: count directory landings (tab enter + folder nav share this path).
    void recordPathVisit(result.path, true);
    if (recordHistory) {
      recordNavHistory(previous, result.path);
    }
    if (syncOpenState) {
      liveOpenState.value = openStateAfterNavigate(liveOpenState.value, result.path);
      schedulePersist(false);
    }
  } catch (err) {
    listing.value = null;
    listError.value = err instanceof Error ? err.message : String(err);
    currentPath.value = trimmed;
    if (pathChanged) clearNavigatorFilter();
    if (recordHistory) {
      recordNavHistory(previous, trimmed);
    }
    if (syncOpenState && trimmed) {
      liveOpenState.value = openStateAfterNavigate(liveOpenState.value, trimmed);
      schedulePersist(false);
    }
  } finally {
    listLoading.value = false;
  }
}

export async function openPathsAsNavigatorTabs(paths: string[]): Promise<void> {
  const next = openStateFromPaths(paths);
  applyLiveOpenState(next, { persist: true, immediatePersist: true });
  const active = activeStripTab(liveOpenState.value);
  if (active?.path) {
    await navigateTo(active.path, { syncOpenState: false });
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
  schedulePersist(true);
  await navigateTo(target, { syncOpenState: false });
}

export async function closeNavigatorTab(groupIndex: number): Promise<void> {
  const closing = liveOpenState.value.tabGroups[groupIndex] ?? [];
  forgetPinnedTabs(closing.map((tab) => tab.id));
  const next = openStateAfterCloseGroup(liveOpenState.value, groupIndex);
  liveOpenState.value = next;
  schedulePersist(true);
  const active = activeStripTab(next);
  if (active?.path) {
    await navigateTo(active.path, { syncOpenState: false });
  } else {
    await loadDefaultPath();
  }
}

export async function switchNavigatorTab(groupIndex: number): Promise<void> {
  liveOpenState.value = openStateAfterSwitchGroup(liveOpenState.value, groupIndex);
  schedulePersist(true);
  const active = activeStripTab(liveOpenState.value);
  if (active?.path) {
    await navigateTo(active.path, { syncOpenState: false });
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
  schedulePersist(true);
}

/** Cycle page groups (Sigma Ctrl+Tab / Ctrl+Shift+Tab → switchToAdjacentTabGroup). */
export async function switchNavigatorTabAdjacent(
  direction: SwitchAdjacentDirection,
): Promise<void> {
  if (liveOpenState.value.tabGroups.length < 2) return;
  liveOpenState.value = openStateAfterSwitchAdjacent(liveOpenState.value, direction);
  schedulePersist(true);
  const active = activeStripTab(liveOpenState.value);
  if (active?.path) {
    await navigateTo(active.path, { syncOpenState: false });
  }
}

/**
 * Select an existing page group for `path`, or open a new one (Sigma openOrFocusTabGroup).
 * Silent Open State persist — no WG nag modal.
 */
export async function openOrFocusNavigatorTab(path: string): Promise<void> {
  const trimmed = path.trim();
  if (!trimmed) return;
  const result = openStateOpenOrFocusPath(liveOpenState.value, trimmed);
  liveOpenState.value = result.state;
  schedulePersist(true);
  const active = activeStripTab(liveOpenState.value);
  if (active?.path) {
    await navigateTo(active.path, { syncOpenState: false });
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
  schedulePersist(true);
  const active = activeStripTab(liveOpenState.value);
  if (active?.path) {
    await navigateTo(active.path, { syncOpenState: false });
  } else if (opened[0]) {
    await navigateTo(opened[0], { syncOpenState: false });
  }
  return opened;
}

/** Apply Open State restored by Work Group silent switch / hard-member sync. */
export async function restoreWorkGroupOpenState(
  workGroupId: string,
  state: WorkGroupOpenState | null | undefined,
): Promise<void> {
  const merged = mergePinnedIntoOpenState(
    normalizeRestoredOpenState(state),
    liveOpenState.value,
    new Set(pinnedTabIds.value),
  );
  activeWorkGroupId.value = workGroupId;
  applyLiveOpenState(merged);
  const active = activeStripTab(liveOpenState.value);
  if (active?.path) {
    await navigateTo(active.path, { syncOpenState: false });
  } else if (!liveOpenState.value.tabGroups.length) {
    listing.value = null;
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
  const logical = navigableParentPath(currentPath.value);
  if (logical) {
    await navigateTo(logical);
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

/** Snapshot for Work Groups IPC that need the leaving group's live Open State. */
export function snapshotLiveOpenState(): WorkGroupOpenState {
  return normalizeRestoredOpenState(liveOpenState.value);
}

/** Same snapshot without pinned tabs, so a switch does not store them on the leaving group. */
export function snapshotLeavingOpenState(): WorkGroupOpenState {
  return openStateWithoutPinned(snapshotLiveOpenState(), new Set(pinnedTabIds.value));
}
