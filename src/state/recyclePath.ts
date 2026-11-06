// SPDX-License-Identifier: GPL-3.0-or-later
/** Virtual Recycle Bin path (never sent to navigator_list). */

export const RECYCLE_PATH = "afm://recycle-bin";

export function normalizeRecycleKey(path: string): string {
  return path.trim().replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

export function isRecyclePath(path: string | null | undefined): boolean {
  if (!path) return false;
  return normalizeRecycleKey(path) === normalizeRecycleKey(RECYCLE_PATH);
}
