// SPDX-License-Identifier: GPL-3.0-or-later
/** Lazy shell icon / thumbnail data URLs (AIA-65). Size-aware cache for icon views. */

import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type ShellIconResult = {
  path: string;
  dataUrl: string | null;
  kind: string;
};

type QueueItem = { path: string; size: number; preferThumbnail: boolean };

/** `${size}|${path}` → data URL */
export const iconByPath = reactive<Record<string, string>>({});

const pending = new Set<string>();
let flushTimer: ReturnType<typeof setTimeout> | null = null;
const queue: QueueItem[] = [];

function cacheKey(path: string, size: number): string {
  return `${size}|${path}`;
}

async function flushQueue(): Promise<void> {
  flushTimer = null;
  if (!queue.length) return;
  const batch = queue.splice(0, 40);
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
    const size = items[0]!.size;
    const preferThumbnail = items[0]!.preferThumbnail;
    try {
      const results = await invoke<ShellIconResult[]>("shell_icon_get_many", {
        paths: items.map((i) => i.path),
        size,
        preferThumbnail,
      });
      for (const item of results) {
        if (item.dataUrl) {
          iconByPath[cacheKey(item.path, size)] = item.dataUrl;
        }
      }
    } catch {
      // Fallback glyphs remain visible.
    }
  }
  if (queue.length) scheduleFlush();
}

function scheduleFlush() {
  if (flushTimer) return;
  flushTimer = setTimeout(() => {
    void flushQueue();
  }, 40);
}

export function requestShellIcon(
  path: string,
  size = 20,
  preferThumbnail = true,
): void {
  const keyPath = path.trim();
  if (!keyPath) return;
  const key = cacheKey(keyPath, size);
  if (iconByPath[key] || pending.has(key)) return;
  pending.add(key);
  queue.push({ path: keyPath, size, preferThumbnail });
  scheduleFlush();
}

export function requestShellIcons(
  paths: string[],
  size = 20,
  preferThumbnail = true,
): void {
  for (const path of paths) requestShellIcon(path, size, preferThumbnail);
}

export function iconUrlFor(path: string, size = 20): string | null {
  return iconByPath[cacheKey(path, size)] || null;
}
