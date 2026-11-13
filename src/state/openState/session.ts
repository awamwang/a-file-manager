// SPDX-License-Identifier: GPL-3.0-or-later
/** Open State 运行时会话：live 引用 / hydrate / 防抖持久化 / 离开快照。 */

import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  activeStripTab,
  createEmptyOpenState,
  normalizeRestoredOpenState,
  openStateWithoutPinned,
  stripTabsFromOpenState,
  type WorkGroupOpenState,
} from "./model.ts";
import { pinnedIdSet } from "../tabPins.ts";

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

/** Runtime Open State for the active Work Group (Navigator tab strip). */
export const liveOpenState = ref<WorkGroupOpenState>(createEmptyOpenState());
export const activeWorkGroupId = ref("");

export const stripTabs = computed(() => stripTabsFromOpenState(liveOpenState.value));

let persistTimer: ReturnType<typeof setTimeout> | null = null;
/** Single-flight `work_groups_get` so Shell chrome + OpenState bootstrap share one IPC. */
let workGroupsHydrate: Promise<UserWorkGroupsDoc> | null = null;

/** 防抖写盘；browse 在原地改 liveOpenState 后调用。 */
export function scheduleOpenStatePersist(immediate = false): void {
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

/** Drop a pending debounced persist (silent switch owns the leaving-group write). */
export function cancelOpenStatePersist(): void {
  if (persistTimer) {
    clearTimeout(persistTimer);
    persistTimer = null;
  }
}

export async function persistOpenStateNow(): Promise<void> {
  if (!activeWorkGroupId.value) return;
  try {
    // AIA-245: never persist session pins onto the active WG (same strip as silent switch).
    await invoke<UserWorkGroupsDoc>("work_groups_save_open_state", {
      workGroupId: activeWorkGroupId.value,
      liveOpenState: snapshotLeavingOpenState(),
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
    scheduleOpenStatePersist(options.immediatePersist ?? true);
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

/** Snapshot for Work Groups IPC that need the leaving group's live Open State. */
export function snapshotLiveOpenState(): WorkGroupOpenState {
  return normalizeRestoredOpenState(liveOpenState.value);
}

/** Same snapshot without pinned tabs, so a switch does not store them on the leaving group. */
export function snapshotLeavingOpenState(): WorkGroupOpenState {
  const preferredPin = activeStripTab(liveOpenState.value)?.id ?? null;
  return openStateWithoutPinned(snapshotLiveOpenState(), pinnedIdSet(preferredPin));
}
