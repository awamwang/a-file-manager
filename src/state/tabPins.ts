// SPDX-License-Identifier: GPL-3.0-or-later
/** 标签固定：会话内跨工作组保留；按路径持久化到 work-groups.json 顶层（与工作组打开标签同存储级别），重启后恢复并带入新工作组。 */

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
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

/** work-groups.json 顶层 `pinnedTab` 的 wire 形状（camelCase）。 */
export type PinnedTabWire = {
  path: string;
  name: string;
  type: string;
  stripIndex: number;
} | null;

/** 旧存储键（localStorage）：仅用于一次性迁移到 work-groups.json。 */
const LEGACY_STORAGE_KEY = "afm.pinnedTabs.v1";

/** 内存中的持久化快照：启动时由 work-groups.json 种入，写入时同步更新。 */
let persistedPin: PinnedTabSnapshot | null = null;
/** 种子完成前禁止清空持久化 pin，避免 bootstrap 失败回退路径误抹已有固定标签。 */
let pinSeedReady = false;
/** 串行化后端写请求，保证 pin 写与 openState 写的落盘顺序。 */
let pinWriteChain: Promise<unknown> = Promise.resolve();
/** 上一次已排队的 wire 值，跳过无变化写入。 */
let lastQueuedWire: PinnedTabWire = null;

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

function normalizeWirePinnedTab(raw: unknown): PinnedTabSnapshot | null {
  if (!raw || typeof raw !== "object") return null;
  const row = raw as Record<string, unknown>;
  const path = typeof row.path === "string" ? toNativeFsPath(row.path) : "";
  if (!path) return null;
  return {
    path,
    type: typeof row.type === "string" && row.type ? row.type : "directory",
    name: typeof row.name === "string" && row.name ? row.name : path,
    stripIndex:
      typeof row.stripIndex === "number" && Number.isFinite(row.stripIndex)
        ? Math.max(0, Math.trunc(row.stripIndex))
        : 0,
  };
}

function wireFromSnapshot(snap: PinnedTabSnapshot | null): PinnedTabWire {
  if (!snap?.path) return null;
  return {
    path: toNativeFsPath(snap.path),
    type: snap.type || "directory",
    name: snap.name || snap.path,
    stripIndex: Math.max(0, Math.trunc(snap.stripIndex) || 0),
  };
}

function wireEquals(a: PinnedTabWire, b: PinnedTabWire): boolean {
  if (a === b) return true;
  if (!a || !b) return false;
  return (
    a.path === b.path &&
    a.name === b.name &&
    a.type === b.type &&
    a.stripIndex === b.stripIndex
  );
}

/** 一次性迁移：localStorage 旧固定标签 → work-groups.json。 */
function migrateLegacySnapshot(): PinnedTabSnapshot | null {
  try {
    const raw = globalThis.localStorage?.getItem(LEGACY_STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed) || !parsed.length) return null;
    return normalizeWirePinnedTab(parsed[0]);
  } catch {
    return null;
  }
}

function clearLegacySnapshot(): void {
  try {
    globalThis.localStorage?.removeItem(LEGACY_STORAGE_KEY);
  } catch {
    /* quota / private mode */
  }
}

/** 把 wire 值串行写入后端 work-groups.json（跳过无变化写入）。 */
function queuePinWireWrite(wire: PinnedTabWire): void {
  if (wireEquals(wire, lastQueuedWire)) return;
  lastQueuedWire = wire;
  pinWriteChain = pinWriteChain
    .then(() => invoke("work_groups_set_pinned_tab", { pinned: wire }))
    .catch(() => undefined);
}

/** 等待已排队的 pin 写全部落盘（quit/hide flush 与 openState 落盘前调用）。 */
export function flushPinnedPersistence(): Promise<unknown> {
  return pinWriteChain;
}

/**
 * 冷启动种子：把 work-groups.json 顶层 `pinnedTab` 读入内存。
 * 后端没有且 localStorage 有旧数据时执行一次性迁移。
 */
export function seedPersistedPinFromDoc(pinnedTab: unknown): void {
  pinSeedReady = true;
  const fromDoc = normalizeWirePinnedTab(pinnedTab);
  if (fromDoc) {
    persistedPin = fromDoc;
    lastQueuedWire = wireFromSnapshot(fromDoc);
    clearLegacySnapshot();
    return;
  }
  const legacy = migrateLegacySnapshot();
  if (legacy) {
    persistedPin = legacy;
    queuePinWireWrite(wireFromSnapshot(legacy));
    void flushPinnedPersistence().then(clearLegacySnapshot);
    return;
  }
  persistedPin = null;
  lastQueuedWire = null;
}

/** 读取持久化固定标签快照（内存缓存，启动时由 work-groups.json 种入）。 */
export function readPinnedSnapshots(): PinnedTabSnapshot[] {
  return persistedPin ? [persistedPin] : [];
}

/** 写入或清除固定标签快照：同步更新内存，异步串行写穿 work-groups.json。 */
export function writePinnedSnapshots(snapshots: readonly PinnedTabSnapshot[]): void {
  const single = snapshots[0];
  persistedPin = single?.path
    ? { ...single, path: toNativeFsPath(single.path) }
    : null;
  const wire = wireFromSnapshot(persistedPin);
  if (!wire) {
    // 种子完成前不允许清空：bootstrap 失败回退不应抹掉已有固定标签。
    if (!pinSeedReady) return;
    clearLegacySnapshot();
  }
  queuePinWireWrite(wire);
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
  // pin id 暂时不在 strip（瞬态替换）：保留现有快照，避免固定标签被误清。
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
