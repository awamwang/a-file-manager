// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Pure reducer for the navigator "Add to Work Group" action.
 *
 * Extracted from NavigatorContextMenu.vue so the multi-select → folder-path
 * reduction can be unit-tested without a running app (no reactivity, no invoke).
 * Behavior must stay identical to the inlined version it replaced.
 */
import type { NavigatorEntry } from "../../state/navigatorBrowse.ts";

/**
 * Which folder paths an "Add to Work Group" action should target.
 * - A non-directory selection contributes nothing.
 * - A plain (single, or not part of a >1 multi-select) dir → just that dir.
 * - A dir that is part of a >1 multi-select → every selected dir (files skipped);
 *   falls back to the entry itself when the listing no longer contains the selection.
 */
export function folderPathsForWorkGroup(
  entry: Pick<NavigatorEntry, "kind" | "path">,
  selectedPaths: string[],
  entries: readonly NavigatorEntry[] | null | undefined,
): string[] {
  if (entry.kind !== "dir") return [];
  if (selectedPaths.includes(entry.path) && selectedPaths.length > 1) {
    const set = new Set(selectedPaths);
    const dirs = (entries ?? [])
      .filter((row) => set.has(row.path) && row.kind === "dir")
      .map((row) => row.path);
    return dirs.length ? dirs : [entry.path];
  }
  return [entry.path];
}
