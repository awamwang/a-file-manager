// SPDX-License-Identifier: GPL-3.0-or-later
/** Lazy shell icon / thumbnail data URLs (AIA-65). */

import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type ShellIconResult = {
  path: string;
  dataUrl: string | null;
  kind: string;
};

/** path → data URL (or "" when missing). */
export const iconByPath = reactive<Record<string, string>>({});

const pending = new Set<string>();
let flushTimer: ReturnType<typeof setTimeout> | null = null;
const queue: string[] = [];

async function flushQueue(): Promise<void> {
  flushTimer = null;
  if (!queue.length) return;
  const batch = queue.splice(0, 40);
  for (const p of batch) pending.delete(p);
  try {
    const results = await invoke<ShellIconResult[]>("shell_icon_get_many", {
      paths: batch,
      size: 20,
      preferThumbnail: true,
    });
    for (const item of results) {
      if (item.dataUrl) {
        iconByPath[item.path] = item.dataUrl;
      }
    }
  } catch {
    // Fallback glyphs remain visible.
  }
  if (queue.length) scheduleFlush();
}

function scheduleFlush() {
  if (flushTimer) return;
  flushTimer = setTimeout(() => {
    void flushQueue();
  }, 40);
}

export function requestShellIcon(path: string): void {
  const key = path.trim();
  if (!key || iconByPath[key] || pending.has(key)) return;
  pending.add(key);
  queue.push(key);
  scheduleFlush();
}

export function requestShellIcons(paths: string[]): void {
  for (const path of paths) requestShellIcon(path);
}

export function iconUrlFor(path: string): string | null {
  return iconByPath[path] || null;
}
