// SPDX-License-Identifier: GPL-3.0-or-later
/** Lazy shell icon / thumbnail data URLs (AIA-65 · AIA-278 · AIA-279).
 *  Size-aware cache with hard entry + byte budgets and LRU eviction. */

import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type ShellIconResult = {
  path: string;
  dataUrl: string | null;
  kind: string;
};

type QueueItem = { path: string; size: number; preferThumbnail: boolean };

export type RequestShellIconsOptions = {
  /** Push to the front so the current viewport wins over stale scroll leftovers. */
  priority?: boolean;
};

/** Hard caps — extreme folders must not retain every decoded bitmap (AIA-279). */
export const SHELL_ICON_MAX_ENTRIES = 512;
/** Approximate UTF-16 / string weight of cached data URLs. */
export const SHELL_ICON_MAX_BYTES = 48 * 1024 * 1024;

/** `${size}|${path}` → data URL (or blob: URL). */
export const iconByPath = reactive<Record<string, string>>({});

type CacheMeta = {
  bytes: number;
  /** Left viewport / previous folder — prefer for eviction. */
  soft: boolean;
};

/** Insertion order = LRU (oldest = first key). */
const lruOrder = new Map<string, true>();
const cacheMeta = new Map<string, CacheMeta>();
let liveBytes = 0;

const pending = new Set<string>();
let flushTimer: ReturnType<typeof setTimeout> | null = null;
const queue: QueueItem[] = [];
/** Bumped on folder change / full cancel so in-flight IPC results are dropped. */
let generation = 0;
let flushing = false;
let applyTimer: ReturnType<typeof requestAnimationFrame> | null = null;
const pendingApplies: Array<{ key: string; dataUrl: string }> = [];

/** Thumbnail IPC is expensive (Shell handlers); keep batches small so cancel can bite. */
const BATCH_ICON = 40;
const BATCH_THUMB = 8;
const FLUSH_DELAY_MS = 40;

function cacheKey(path: string, size: number): string {
  return `${size}|${path}`;
}

function estimateBytes(dataUrl: string): number {
  return dataUrl.length;
}

function dropEntry(key: string): void {
  const meta = cacheMeta.get(key);
  if (meta) liveBytes = Math.max(0, liveBytes - meta.bytes);
  cacheMeta.delete(key);
  lruOrder.delete(key);
  // Drop decoded / encoded bitmap payload (data URLs today; revoke blob: if introduced).
  const url = iconByPath[key];
  if (url && typeof URL !== "undefined" && url.startsWith("blob:")) {
    try {
      URL.revokeObjectURL(url);
    } catch {
      // ignore revoke races
    }
  }
  delete iconByPath[key];
}

function touchKey(key: string): void {
  if (!lruOrder.has(key)) return;
  lruOrder.delete(key);
  lruOrder.set(key, true);
  const meta = cacheMeta.get(key);
  if (meta) meta.soft = false;
}

function markSoftKey(key: string): void {
  const meta = cacheMeta.get(key);
  if (meta) meta.soft = true;
}

function pickVictim(): string | null {
  for (const [key] of lruOrder) {
    if (cacheMeta.get(key)?.soft) return key;
  }
  const first = lruOrder.keys().next();
  return first.done ? null : first.value;
}

function trimToBudget(): void {
  while (
    lruOrder.size > SHELL_ICON_MAX_ENTRIES ||
    liveBytes > SHELL_ICON_MAX_BYTES
  ) {
    const victim = pickVictim();
    if (!victim) break;
    dropEntry(victim);
  }
}

function putCache(key: string, dataUrl: string): void {
  if (cacheMeta.has(key)) dropEntry(key);
  const bytes = estimateBytes(dataUrl);
  // Keep data URLs (no sync atob on rAF). img decoding="async" stays off the paint path.
  iconByPath[key] = dataUrl;
  cacheMeta.set(key, { bytes, soft: false });
  lruOrder.set(key, true);
  liveBytes += bytes;
  trimToBudget();
}

function queueIndex(key: string): number {
  return queue.findIndex((item) => cacheKey(item.path, item.size) === key);
}

function enqueue(item: QueueItem, priority: boolean): void {
  const key = cacheKey(item.path, item.size);
  const existing = queueIndex(key);
  if (existing >= 0) {
    if (priority && existing > 0) {
      const [row] = queue.splice(existing, 1);
      if (row) queue.unshift(row);
    }
    return;
  }
  if (priority) queue.unshift(item);
  else queue.push(item);
}

function scheduleApply(): void {
  if (applyTimer != null) return;
  applyTimer = requestAnimationFrame(() => {
    applyTimer = null;
    const batch = pendingApplies.splice(0, pendingApplies.length);
    for (const { key, dataUrl } of batch) {
      putCache(key, dataUrl);
    }
  });
}

function applyResult(path: string, size: number, dataUrl: string, gen: number): void {
  if (gen !== generation) return;
  pendingApplies.push({ key: cacheKey(path, size), dataUrl });
  scheduleApply();
}

async function flushQueue(): Promise<void> {
  flushTimer = null;
  if (flushing || !queue.length) return;
  flushing = true;
  const gen = generation;

  try {
    while (queue.length && gen === generation) {
      const preferThumb = queue[0]?.preferThumbnail === true;
      const batchSize = preferThumb ? BATCH_THUMB : BATCH_ICON;
      const batch = queue.splice(0, batchSize);
      for (const item of batch) pending.delete(cacheKey(item.path, item.size));

      // Group by size + preferThumbnail so one IPC round-trip stays coherent.
      const groups = new Map<string, QueueItem[]>();
      for (const item of batch) {
        const gKey = `${item.size}|${item.preferThumbnail ? 1 : 0}`;
        const list = groups.get(gKey) ?? [];
        list.push(item);
        groups.set(gKey, list);
      }

      for (const [, items] of groups) {
        if (gen !== generation) return;
        const size = items[0]!.size;
        const preferThumbnail = items[0]!.preferThumbnail;
        try {
          const results = await invoke<ShellIconResult[]>("shell_icon_get_many", {
            paths: items.map((i) => i.path),
            size,
            preferThumbnail,
          });
          if (gen !== generation) return;
          for (const item of results) {
            if (item.dataUrl) applyResult(item.path, size, item.dataUrl, gen);
          }
        } catch {
          // Fallback glyphs remain visible.
        }
      }
    }
  } finally {
    flushing = false;
    if (queue.length && generation === gen) scheduleFlush();
  }
}

function scheduleFlush() {
  if (flushTimer || flushing) return;
  flushTimer = setTimeout(() => {
    void flushQueue();
  }, FLUSH_DELAY_MS);
}

/** Drop queued / in-flight work (folder change). Cache stays soft for back-navigation. */
export function clearShellIconQueue(): void {
  generation += 1;
  queue.length = 0;
  pending.clear();
  pendingApplies.length = 0;
  if (flushTimer) {
    clearTimeout(flushTimer);
    flushTimer = null;
  }
  if (applyTimer != null) {
    cancelAnimationFrame(applyTimer);
    applyTimer = null;
  }
  // Prefer eviction of previous folder under budget pressure.
  for (const key of cacheMeta.keys()) markSoftKey(key);
}

/** Remove paths that left the viewport so they cannot starve visible work. */
export function cancelShellIconRequests(paths: string[], size?: number): void {
  if (!paths.length) return;
  const pathSet = new Set(paths.map((p) => p.trim()).filter(Boolean));
  if (!pathSet.size) return;

  for (let i = queue.length - 1; i >= 0; i -= 1) {
    const item = queue[i]!;
    if (!pathSet.has(item.path)) continue;
    if (size != null && item.size !== size) continue;
    queue.splice(i, 1);
    pending.delete(cacheKey(item.path, item.size));
  }

  for (const path of pathSet) {
    if (size != null) {
      markSoftKey(cacheKey(path, size));
    } else {
      for (const key of cacheMeta.keys()) {
        if (key.endsWith(`|${path}`) || key.slice(key.indexOf("|") + 1) === path) {
          markSoftKey(key);
        }
      }
    }
  }
}

export function requestShellIcon(
  path: string,
  size = 20,
  preferThumbnail = true,
  options?: RequestShellIconsOptions,
): void {
  const keyPath = path.trim();
  if (!keyPath) return;
  const key = cacheKey(keyPath, size);
  if (iconByPath[key]) {
    touchKey(key);
    return;
  }
  const priority = options?.priority === true;
  if (pending.has(key)) {
    if (priority) enqueue({ path: keyPath, size, preferThumbnail }, true);
    return;
  }
  pending.add(key);
  enqueue({ path: keyPath, size, preferThumbnail }, priority);
  scheduleFlush();
}

export function requestShellIcons(
  paths: string[],
  size = 20,
  preferThumbnail = true,
  options?: RequestShellIconsOptions,
): void {
  for (const path of paths) requestShellIcon(path, size, preferThumbnail, options);
}

export function iconUrlFor(path: string, size = 20): string | null {
  const key = cacheKey(path, size);
  const url = iconByPath[key] || null;
  if (url) touchKey(key);
  return url;
}

/** Test/debug helpers. */
export function shellIconQueueSizeForTests(): number {
  return queue.length;
}

export function shellIconGenerationForTests(): number {
  return generation;
}

export function shellIconCacheStatsForTests(): {
  entries: number;
  bytes: number;
  soft: number;
} {
  let soft = 0;
  for (const meta of cacheMeta.values()) {
    if (meta.soft) soft += 1;
  }
  return { entries: lruOrder.size, bytes: liveBytes, soft };
}

/** Inject a cache entry without IPC (unit tests / debug). */
export function shellIconPutForTests(path: string, size: number, dataUrl: string): void {
  putCache(cacheKey(path, size), dataUrl);
}

/** Mark soft without queue cancel (unit tests). */
export function shellIconMarkSoftForTests(path: string, size: number): void {
  markSoftKey(cacheKey(path, size));
}

/** Drop all cache entries (unit tests). */
export function shellIconClearCacheForTests(): void {
  const keys = [...lruOrder.keys()];
  for (const key of keys) dropEntry(key);
  liveBytes = 0;
}
