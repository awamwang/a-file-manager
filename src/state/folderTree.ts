// SPDX-License-Identifier: GPL-3.0-or-later
/** Pure helpers for the navigator folder tree (dirs only, path highlight). */

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

/** Compare Windows paths without trailing slashes. `C:\` stays distinguishable from `C:`. */
export function normalizeTreePath(path: string): string {
  return path.replace(/\//g, "\\").replace(/\\+$/, "").toLowerCase();
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
 * AIA-230: how a sidebar row should highlight.
 * - `tree`: hierarchy selection (node or descendant) → selected semantics
 * - `entry`: favorite / 常用文件夹 roots → exact current only, never selected
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
  return treePathActive(current, node) ? "selected" : "none";
}
