// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Paste-target resolution (CLIP-101 / CLIP-102 / CLIP-104) — pure, headless-testable.
 * Windows Explorer rule: where Ctrl+V lands depends on the current selection, not just
 * the open directory. Extracted from `NavigatorView.vue` so the behavior is pinned by
 * `paste-target-checks.ts` instead of only by a GUI click.
 */

import { toNativeFsPath } from "./pathKey.ts";

export type PasteTargetEntry = { path: string; isDirectory: boolean };

/** Parent directory of a native path; "" for a drive root or a single segment. */
export function parentDirectory(path: string): string {
  const norm = toNativeFsPath(path).replace(/\\+$/, "");
  const idx = norm.lastIndexOf("\\");
  if (idx <= 0) return "";
  return norm.slice(0, idx);
}

/**
 * Resolve where a paste lands from the current selection:
 * - single selected **folder** → into that folder (CLIP-101)
 * - single selected **file**   → into the file's parent dir (CLIP-102)
 * - marquee over mixed / no single container / nothing selected → current dir (CLIP-103/104)
 */
export function resolvePasteTarget(
  selection: PasteTargetEntry[],
  currentDir: string,
): string {
  if (selection.length !== 1) return currentDir;
  const only = selection[0];
  if (!only || !only.path) return currentDir;
  if (only.isDirectory) return only.path;
  return parentDirectory(only.path) || currentDir;
}
