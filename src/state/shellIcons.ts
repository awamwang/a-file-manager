// SPDX-License-Identifier: GPL-3.0-or-later
/** Lazy shell icon / thumbnail data URLs (AIA-65 · AIA-278). Size-aware cache for icon views. */

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

/** `${size}|${path}` → data URL */
export const iconByPath = reactive<Record<string, string>>({});

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
      iconByPath[key] = dataUrl;
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

/** Drop queued / in-flight work (folder change). Cache stays for back-navigation. */
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
  if (iconByPath[key]) return;
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
  return iconByPath[cacheKey(path, size)] || null;
}

/** Test/debug helpers (queue pressure). */
export function shellIconQueueSizeForTests(): number {
  return queue.length;
}

export function shellIconGenerationForTests(): number {
  return generation;
}
