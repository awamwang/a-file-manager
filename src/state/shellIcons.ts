// SPDX-License-Identifier: GPL-3.0-or-later
/** Lazy shell icon / thumbnail data URLs (AIA-65 · AIA-278 · AIA-279).
 *  Size-aware cache with hard entry + byte budgets and LRU eviction. */

import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { perfSpan } from "./perfProbe";

export type ShellIconResult = {
  path: string;
  dataUrl: string | null;
  kind: string;
};

type QueueItem = {
  path: string;
  size: number;
  preferThumbnail: boolean;
  /** Monotonic attempt id; stale in-flight results are dropped after leave/reenter. */
  attempt: number;
};

export type RequestShellIconsOptions = {
  /** Push to the front so the current viewport wins over stale scroll leftovers. */
  priority?: boolean;
};

/** 单条壳图标请求；可按条目关闭缩略图（目录/公共壳图标走快通道）。 */
export type ShellIconRequest = {
  path: string;
  preferThumbnail?: boolean;
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

/** Queued or in-flight keys. Cleared on settle, cancel, or full clear. */
const pending = new Set<string>();
/** Latest attempt id per cache key — leave/reenter bumps this so old IPC cannot stick. */
const attemptByKey = new Map<string, number>();
let nextAttempt = 1;
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
    const row = queue[existing]!;
    row.attempt = item.attempt;
    row.preferThumbnail = item.preferThumbnail;
    // 已在队列中：不因 priority 反复 unshift，避免视口同步把顺序打成扫描推进感。
    return;
  }
  if (priority) queue.unshift(item);
  else queue.push(item);
}

/**
 * 将当前视口请求整块放到队头，并保持传入顺序（上→下）。
 * 避免逐条 unshift 把第一屏倒成最后加载。
 */
function promotePriorityWindow(items: QueueItem[]): void {
  if (!items.length) return;
  const keys = new Set(items.map((item) => cacheKey(item.path, item.size)));
  for (let i = queue.length - 1; i >= 0; i -= 1) {
    const row = queue[i]!;
    if (keys.has(cacheKey(row.path, row.size))) queue.splice(i, 1);
  }
  queue.unshift(...items);
}

/** 优先抽出非缩略图（壳图标）项，让同屏目录先稳定展示。 */
function takeNextBatch(): QueueItem[] {
  const iconIndices: number[] = [];
  for (let i = 0; i < queue.length; i += 1) {
    if (queue[i]!.preferThumbnail !== true) iconIndices.push(i);
  }
  if (iconIndices.length) {
    const take = Math.min(BATCH_ICON, iconIndices.length);
    const batch: QueueItem[] = [];
    for (let i = take - 1; i >= 0; i -= 1) {
      batch.unshift(queue.splice(iconIndices[i]!, 1)[0]!);
    }
    return batch;
  }
  return queue.splice(0, BATCH_THUMB);
}

function settleKey(key: string): void {
  pending.delete(key);
  attemptByKey.delete(key);
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

function applyResult(
  path: string,
  size: number,
  dataUrl: string,
  gen: number,
  attempt: number,
): void {
  if (gen !== generation) return;
  const key = cacheKey(path, size);
  if (attemptByKey.get(key) !== attempt) return;
  pendingApplies.push({ key, dataUrl });
  scheduleApply();
}

async function flushQueue(): Promise<void> {
  flushTimer = null;
  if (flushing || !queue.length) return;
  flushing = true;
  const gen = generation;

  try {
    while (queue.length && gen === generation) {
      const batch = takeNextBatch();

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
        // 离开视口或再次进入会 bump attempt：过期 batch 项直接跳过，且不得 settle 新 attempt。
        const activeItems = items.filter(
          (item) => attemptByKey.get(cacheKey(item.path, item.size)) === item.attempt,
        );
        if (!activeItems.length) continue;

        try {
          const results = await perfSpan(
            "shell_icon_batch",
            () =>
              invoke<ShellIconResult[]>("shell_icon_get_many", {
                paths: activeItems.map((i) => i.path),
                size,
                preferThumbnail,
              }),
            () => ({
              n: activeItems.length,
              size,
              thumb: preferThumbnail ? 1 : 0,
              queue: queue.length,
            }),
          );
          if (gen !== generation) return;
          const byPath = new Map(results.map((item) => [item.path, item] as const));
          for (const item of activeItems) {
            const key = cacheKey(item.path, item.size);
            if (attemptByKey.get(key) !== item.attempt) continue;
            const result = byPath.get(item.path);
            if (result?.dataUrl) {
              applyResult(item.path, size, result.dataUrl, gen, item.attempt);
            }
            // 成功写入或空结果均结束本轮 attempt，允许离开后再进入时重新排队。
            if (attemptByKey.get(key) === item.attempt) settleKey(key);
          }
        } catch {
          // Fallback glyphs remain visible; clear attempts so viewport reenter can retry.
          for (const item of activeItems) {
            const key = cacheKey(item.path, item.size);
            if (attemptByKey.get(key) === item.attempt) settleKey(key);
          }
        }
      }
    }
  } finally {
    flushing = false;
    // 文件夹切换会 bump generation；若期间已有新的排队，仍要继续刷。
    if (queue.length) scheduleFlush();
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
  attemptByKey.clear();
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

function forgetPendingKey(key: string): void {
  pending.delete(key);
  attemptByKey.delete(key);
}

/**
 * 甩动滚动时挂起未开始的内容缩略图：从队列摘掉 preferThumbnail 项并清 pending，
 * 便于停稳后按当前视口重新排队。进行中的 IPC 批仍会结束，但结果可被新 attempt 取代。
 */
export function suspendQueuedContentThumbnails(): void {
  for (let i = queue.length - 1; i >= 0; i -= 1) {
    const item = queue[i]!;
    if (item.preferThumbnail !== true) continue;
    queue.splice(i, 1);
    forgetPendingKey(cacheKey(item.path, item.size));
  }
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
    forgetPendingKey(cacheKey(item.path, item.size));
  }

  // 窗口化/observer 重绑可能丢掉 leave：即使已在 in-flight，也要清掉 pending，
  // 否则回滚视口时会被当成「已请求」而永久跳过。
  for (const path of pathSet) {
    if (size != null) {
      forgetPendingKey(cacheKey(path, size));
      markSoftKey(cacheKey(path, size));
    } else {
      for (const key of [...pending]) {
        if (key.endsWith(`|${path}`) || key.slice(key.indexOf("|") + 1) === path) {
          forgetPendingKey(key);
        }
      }
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
    // 仍在排队/in-flight：保留 attempt。视口几何同步会重复请求同一窗，
    // 若每次 bump，慢速外部缩略图器的整批结果都会被当成过期丢弃（FIL-50）。
    // 离开窗口由 cancel 清 pending；再次进入走下方新 attempt 分支（FIL-33）。
    const attempt = attemptByKey.get(key) ?? nextAttempt++;
    attemptByKey.set(key, attempt);
    // 单条重复请求：已在队列则保持位置；仅 in-flight（不在队列）时按 priority 补排。
    enqueue({ path: keyPath, size, preferThumbnail, attempt }, priority);
    scheduleFlush();
    return;
  }
  const attempt = nextAttempt++;
  attemptByKey.set(key, attempt);
  pending.add(key);
  enqueue({ path: keyPath, size, preferThumbnail, attempt }, priority);
  scheduleFlush();
}

/**
 * 批量请求壳图标。priority 时整窗按传入顺序置于队头（上→下），
 * 且可按条目设置 preferThumbnail（无内容缩略图/公共壳图标应关闭）。
 */
export function requestShellIcons(
  paths: string[] | ShellIconRequest[],
  size = 20,
  preferThumbnail = true,
  options?: RequestShellIconsOptions,
): void {
  const priority = options?.priority === true;
  const requests: ShellIconRequest[] = paths.map((item) =>
    typeof item === "string" ? { path: item, preferThumbnail } : item,
  );
  if (!priority) {
    for (const req of requests) {
      requestShellIcon(
        req.path,
        size,
        req.preferThumbnail ?? preferThumbnail,
        options,
      );
    }
    return;
  }

  const windowItems: QueueItem[] = [];
  for (const req of requests) {
    const keyPath = req.path.trim();
    if (!keyPath) continue;
    const wantThumb = req.preferThumbnail ?? preferThumbnail;
    const key = cacheKey(keyPath, size);
    if (iconByPath[key]) {
      touchKey(key);
      continue;
    }
    let attempt: number;
    if (pending.has(key)) {
      attempt = attemptByKey.get(key) ?? nextAttempt++;
      attemptByKey.set(key, attempt);
    } else {
      attempt = nextAttempt++;
      attemptByKey.set(key, attempt);
      pending.add(key);
    }
    windowItems.push({
      path: keyPath,
      size,
      preferThumbnail: wantThumb,
      attempt,
    });
  }
  promotePriorityWindow(windowItems);
  if (windowItems.length) scheduleFlush();
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

export function shellIconPendingHasForTests(path: string, size: number): boolean {
  return pending.has(cacheKey(path, size));
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
