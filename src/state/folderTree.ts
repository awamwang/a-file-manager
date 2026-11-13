// SPDX-License-Identifier: GPL-3.0-or-later
/** Pure helpers for the navigator folder tree (dirs only, path highlight). */

import { normalizeTreePathKey } from "./pathKey.ts";

export type FolderTreeChild = {
  name: string;
  path: string;
};

export function directoryChildren(
  entries: Array<{ name: string; path: string; kind: string }>,
): FolderTreeChild[] {
  return entries
    .filter((entry) => entry.kind === "dir" && entry.name && entry.path)
    .map((entry) => ({ name: entry.name, path: entry.path }))
    .sort((left, right) =>
      left.name.localeCompare(right.name, undefined, { sensitivity: "base" }),
    );
}

/** Compare Windows paths without trailing slashes. `C:\` 与 `C:` 同键。 */
export function normalizeTreePath(path: string): string {
  return normalizeTreePathKey(path);
}

/** True when `current` is the node or a descendant. Drive roots match `C:\…`. */
export function treePathActive(current: string, node: string): boolean {
  const here = normalizeTreePath(current);
  const root = normalizeTreePath(node);
  if (!here || !root) return false;
  if (here === root) return true;
  return here.startsWith(`${root}\\`);
}

/** Exact path match only (AIA-230 entry points — no sticky selected / descendant glow). */
export function treePathExact(current: string, node: string): boolean {
  const here = normalizeTreePath(current);
  const root = normalizeTreePath(node);
  if (!here || !root) return false;
  return here === root;
}

/**
 * AIA-236: expand a node when `current` is a strict descendant (ancestors +
 * siblings along the path). The active folder itself need not be expanded.
 */
export function treeShouldAutoExpand(current: string, node: string): boolean {
  const here = normalizeTreePath(current);
  const root = normalizeTreePath(node);
  if (!here || !root) return false;
  if (here === root) return false;
  return here.startsWith(`${root}\\`);
}

/**
 * How a sidebar row should highlight.
 * - `tree` (AIA-236 Explorer): exact current folder → selected
 * - `entry` (AIA-230 legacy): exact current only as soft current cue
 */
export type TreeHighlightMode = "tree" | "entry";

export type TreeHighlight = "none" | "selected" | "current";

export function treeRowHighlight(
  current: string,
  node: string,
  mode: TreeHighlightMode,
): TreeHighlight {
  if (mode === "entry") {
    return treePathExact(current, node) ? "current" : "none";
  }
  return treePathExact(current, node) ? "selected" : "none";
}

/** True for real OS / UNC paths that belong under This PC drive roots. */
export function isOsHierarchyPath(path: string | null | undefined): boolean {
  if (!path) return false;
  const trimmed = path.trim();
  if (!trimmed) return false;
  if (/^[a-zA-Z]:[\\/]/.test(trimmed) || /^[a-zA-Z]:$/.test(trimmed)) return true;
  if (trimmed.startsWith("\\\\")) return true;
  return false;
}
