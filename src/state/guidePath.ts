// SPDX-License-Identifier: GPL-3.0-or-later
/** Guide / This PC virtual path helpers (no Vue — usable from Node checks). */

/** App-internal guide landing — never sent to `navigator_list`. */
export const GUIDE_PATH = "afm://guide";

export function normalizeGuideKey(path: string): string {
  return path.trim().replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

export function isGuidePath(path: string | null | undefined): boolean {
  if (!path) return false;
  return normalizeGuideKey(path) === normalizeGuideKey(GUIDE_PATH);
}
