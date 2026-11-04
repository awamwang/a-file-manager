// SPDX-License-Identifier: GPL-3.0-or-later

import { computed, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  activeStripTab,
  createEmptyOpenState,
  normalizeRestoredOpenState,
  openStateAfterCloseGroup,
  openStateAfterNavigate,
  openStateAfterSwitchGroup,
  openStateFromPaths,
  openStateWithNewTab,
  stripTabsFromOpenState,
  type WorkGroupOpenState,
} from "./openStateModel";

export type NavigatorEntry = {
  name: string;
  path: string;
  kind: "dir" | "file" | string;
  size: number;
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
      liveOpenState: liveOpenState.value,
    });
  } catch {
    // Persistence failures should not block browsing; WG UI can still save explicitly.
  }
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

export function bootstrapNavigatorOpenState(): Promise<void> {
  if (!bootstrapPromise) {
    bootstrapPromise = (async () => {
      try {
        const doc = await invoke<UserWorkGroupsDoc>("work_groups_get");
        activeWorkGroupId.value = doc.activeWorkGroupId;
        const active = doc.workGroups.find((g) => g.id === doc.activeWorkGroupId);
        applyLiveOpenState(active?.openState ?? null);
        const activeTab = activeStripTab(liveOpenState.value);
        if (activeTab?.path) {
          await navigateTo(activeTab.path, { syncOpenState: false });
        } else {
          await loadDefaultPath();
        }
      } catch {
        await loadDefaultPath();
      }
    })();
  }
  return bootstrapPromise;
}

export async function navigateTo(
  path: string,
  options?: { syncOpenState?: boolean },
): Promise<void> {
  const trimmed = path.trim();
  const syncOpenState = options?.syncOpenState !== false;
  listLoading.value = true;
  listError.value = null;
  try {
    const result = await invoke<NavigatorListing>("navigator_list", { path: trimmed });
    listing.value = result;
    currentPath.value = result.path;
    if (syncOpenState) {
      liveOpenState.value = openStateAfterNavigate(liveOpenState.value, result.path);
      schedulePersist(false);
    }
  } catch (err) {
    listing.value = null;
    listError.value = err instanceof Error ? err.message : String(err);
    currentPath.value = trimmed;
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
  let target = (path ?? currentPath.value).trim();
  if (!target) {
    try {
      target = await invoke<string>("navigator_default_path");
    } catch {
      target = "C:/";
    }
  }
  liveOpenState.value = openStateWithNewTab(liveOpenState.value, target);
  schedulePersist(true);
  await navigateTo(target, { syncOpenState: false });
}

export async function closeNavigatorTab(groupIndex: number): Promise<void> {
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

/** Apply Open State restored by Work Group silent switch / hard-member sync. */
export async function restoreWorkGroupOpenState(
  workGroupId: string,
  state: WorkGroupOpenState | null | undefined,
): Promise<void> {
  activeWorkGroupId.value = workGroupId;
  applyLiveOpenState(state);
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
  if (!currentPath.value) return;
  const parent = await invoke<string | null>("navigator_parent", {
    path: currentPath.value,
  });
  if (parent) {
    await navigateTo(parent);
  }
}

export async function openEntry(entry: NavigatorEntry): Promise<void> {
  if (entry.kind === "dir") {
    await navigateTo(entry.path);
  }
}

/** Snapshot for Work Groups IPC that need the leaving group's live Open State. */
export function snapshotLiveOpenState(): WorkGroupOpenState {
  return normalizeRestoredOpenState(liveOpenState.value);
}
