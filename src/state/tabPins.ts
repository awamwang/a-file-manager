// SPDX-License-Identifier: GPL-3.0-or-later
/** 标签固定：会话内跨工作组保留；按路径持久化以便重启后恢复，并在新工作组中打开同一固定标签。 */

import { ref } from "vue";
import {
  clampOpenStateIndices,
  createDirectoryTab,
  normalizePathKey,
  toNativeFsPath,
  type WorkGroupOpenState,
  type WorkGroupOpenStateTab,
} from "./openState/model.ts";

export const pinnedTabIds = ref<string[]>([]);

/** 固定标签的可持久化身份（tab id 会随会话重建，不能跨重启）。 */
export type PinnedTabSnapshot = {
  path: string;
  type: string;
  name: string;
  stripIndex: number;
};

const STORAGE_KEY = "afm.pinnedTabs.v1";

/** Pick one id from a multi-pin list: preferred if present, else first. */
export function pickSinglePinnedId(
  ids: readonly string[],
  preferredId?: string | null,
): string | null {
  const cleaned = ids.filter((id) => Boolean(id));
  if (!cleaned.length) return null;
  if (preferredId && cleaned.includes(preferredId)) return preferredId;
  return cleaned[0] ?? null;
}

/** Collapse session pin state to ≤1 id (AIA-260). */
export function ensureSinglePinnedTab(preferredId?: string | null): void {
  const id = pickSinglePinnedId(pinnedTabIds.value, preferredId);
  pinnedTabIds.value = id ? [id] : [];
}

/** Session pin set for Open State helpers — always ≤1 id. */
export function pinnedIdSet(preferredId?: string | null): Set<string> {
  ensureSinglePinnedTab(preferredId);
  const id = pinnedTabIds.value[0];
  return id ? new Set([id]) : new Set();
}

export function isTabPinned(id: string): boolean {
  if (!id) return false;
  return pinnedTabIds.value.includes(id);
}

/** Pin replaces any previous pin; pin again on the same tab unpins. */
export function togglePinnedTab(id: string): void {
  if (!id) return;
  pinnedTabIds.value = isTabPinned(id) ? [] : [id];
  // 取消固定时立即清快照；固定时由调用方带 Open State 调用 commitPinnedPersistence。
  if (!pinnedTabIds.value.length) writePinnedSnapshots([]);
}

export function forgetPinnedTabs(ids: string[]): void {
  if (!ids.length) return;
  const drop = new Set(ids);
  pinnedTabIds.value = pinnedTabIds.value.filter((id) => id && !drop.has(id));
  if (!pinnedTabIds.value.length) writePinnedSnapshots([]);
}

function clampStripIndex(index: number, length: number): number {
  if (!Number.isFinite(index) || length <= 0) return 0;
  return Math.max(0, Math.min(Math.trunc(index), length));
}

function snapshotFromTab(
  tab: WorkGroupOpenStateTab,
  stripIndex: number,
): PinnedTabSnapshot {
  return {
    path: toNativeFsPath(tab.path),
    type: tab.type || "directory",
    name: tab.name || tab.path,
    stripIndex: Math.max(0, Math.trunc(stripIndex) || 0),
  };
}

/** 读取持久化固定标签快照（同步 localStorage，不阻塞 UI）。 */
export function readPinnedSnapshots(): PinnedTabSnapshot[] {
  try {
    const raw = globalThis.localStorage?.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    const out: PinnedTabSnapshot[] = [];
    for (const item of parsed) {
      if (!item || typeof item !== "object") continue;
      const row = item as Record<string, unknown>;
      const path = typeof row.path === "string" ? toNativeFsPath(row.path) : "";
      if (!path) continue;
      out.push({
        path,
        type: typeof row.type === "string" && row.type ? row.type : "directory",
        name: typeof row.name === "string" && row.name ? row.name : path,
        stripIndex:
          typeof row.stripIndex === "number" && Number.isFinite(row.stripIndex)
            ? Math.max(0, Math.trunc(row.stripIndex))
            : 0,
      });
      // AIA-260：最多保留一条
      break;
    }
    return out;
  } catch {
    return [];
  }
}

/** 写入或清除固定标签快照。 */
export function writePinnedSnapshots(snapshots: readonly PinnedTabSnapshot[]): void {
  try {
    const single = snapshots[0];
    if (!single?.path) {
      globalThis.localStorage?.removeItem(STORAGE_KEY);
      return;
    }
    globalThis.localStorage?.setItem(
      STORAGE_KEY,
      JSON.stringify([
        {
          path: toNativeFsPath(single.path),
          type: single.type || "directory",
          name: single.name || single.path,
          stripIndex: Math.max(0, Math.trunc(single.stripIndex) || 0),
        },
      ]),
    );
  } catch {
    /* quota / private mode */
  }
}

/**
 * 根据当前 strip 与 pinnedTabIds 同步持久化内容。
 * 在固定/取消固定、关闭固定标签、以及 Open State 落盘时调用。
 */
export function commitPinnedPersistence(state: WorkGroupOpenState): void {
  ensureSinglePinnedTab();
  const id = pinnedTabIds.value[0];
  if (!id) {
    writePinnedSnapshots([]);
    return;
  }
  for (let groupIndex = 0; groupIndex < state.tabGroups.length; groupIndex += 1) {
    const group = state.tabGroups[groupIndex];
    const tab = group?.find((item) => item.id === id) ?? group?.[0];
    if (group?.some((item) => item.id === id) && tab) {
      writePinnedSnapshots([snapshotFromTab(tab, groupIndex)]);
      return;
    }
  }
  writePinnedSnapshots([]);
}

/**
 * 冷启动：把持久化固定标签合并进 Open State，并恢复 pinnedTabIds。
 * 已存在同路径标签则复用；否则按上次 strip 位置插入。不抢占非固定的当前标签焦点。
 */
export function hydratePersistedPins(state: WorkGroupOpenState): {
  state: WorkGroupOpenState;
  pinnedIds: string[];
} {
  const snap = readPinnedSnapshots()[0];
  if (!snap?.path) {
    pinnedTabIds.value = [];
    return { state, pinnedIds: [] };
  }
  const key = normalizePathKey(snap.path);
  let pinnedId: string | null = null;
  let nextGroups = state.tabGroups.map((group) => group.map((tab) => ({ ...tab })));
  let currentTabGroupIndex = state.currentTabGroupIndex;
  let currentTabIndex = state.currentTabIndex;

  for (const group of nextGroups) {
    const hit = group.find((tab) => normalizePathKey(tab.path) === key);
    if (hit) {
      pinnedId = hit.id;
      break;
    }
  }

  if (!pinnedId) {
    const tab = createDirectoryTab(snap.path);
    if (snap.type) tab.type = snap.type;
    if (snap.name) tab.name = snap.name;
    const insertAt = clampStripIndex(snap.stripIndex, nextGroups.length);
    nextGroups = [
      ...nextGroups.slice(0, insertAt),
      [tab],
      ...nextGroups.slice(insertAt),
    ];
    if (insertAt <= currentTabGroupIndex && state.tabGroups.length > 0) {
      currentTabGroupIndex += 1;
    }
    pinnedId = tab.id;
  }

  pinnedTabIds.value = pinnedId ? [pinnedId] : [];
  return {
    state: clampOpenStateIndices({
      tabGroups: nextGroups,
      currentTabGroupIndex,
      currentTabIndex,
    }),
    pinnedIds: pinnedId ? [pinnedId] : [],
  };
}
