// SPDX-License-Identifier: GPL-3.0-or-later
/** Client-side Navigator list sort (AIA-64). */

import { ref } from "vue";
import type { NavigatorEntry } from "./navigatorBrowse";
import { entryTypeKey } from "./entryType.ts";

export type SortColumn = "name" | "size" | "modified" | "type";
export type SortDirection = "asc" | "desc";

export const sortColumn = ref<SortColumn>("name");
export const sortDirection = ref<SortDirection>("asc");

type SortCache = {
  source: readonly NavigatorEntry[];
  column: SortColumn;
  direction: SortDirection;
  result: NavigatorEntry[];
};

let sortCache: SortCache | null = null;

/** Drop cached sort (tests / forced refresh). */
export function clearSortedEntriesCache(): void {
  sortCache = null;
}

export function toggleSort(column: SortColumn): void {
  if (sortColumn.value === column) {
    sortDirection.value = sortDirection.value === "asc" ? "desc" : "asc";
  } else {
    sortColumn.value = column;
    sortDirection.value = "asc";
  }
  sortCache = null;
}

function compareEntries(
  a: NavigatorEntry,
  b: NavigatorEntry,
  col: SortColumn,
  dir: number,
): number {
  // Folders first, matching Kernel default, then column.
  if (a.kind !== b.kind) {
    if (a.kind === "dir") return -1;
    if (b.kind === "dir") return 1;
  }
  if (col === "size") {
    if (a.size !== b.size) return (a.size - b.size) * dir;
  }
  if (col === "modified") {
    const am = a.modifiedMs ?? 0;
    const bm = b.modifiedMs ?? 0;
    if (am !== bm) return (am - bm) * dir;
  }
  if (col === "type") {
    const at = entryTypeKey(a.kind, a.name);
    const bt = entryTypeKey(b.kind, b.name);
    const typeCmp = at.localeCompare(bt, undefined, { sensitivity: "base" });
    if (typeCmp !== 0) return typeCmp * dir;
  }
  const nameCmp = a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
  return nameCmp * dir;
}

function probeEnabled(): boolean {
  try {
    return globalThis.localStorage?.getItem("afm.perfProbe") === "1";
  } catch {
    return false;
  }
}

export function sortedEntries(entries: NavigatorEntry[]): NavigatorEntry[] {
  const col = sortColumn.value;
  const direction = sortDirection.value;
  if (
    sortCache &&
    sortCache.source === entries &&
    sortCache.column === col &&
    sortCache.direction === direction
  ) {
    return sortCache.result;
  }

  const dir = direction === "asc" ? 1 : -1;
  const t0 = probeEnabled() ? performance.now() : 0;
  const result = [...entries].sort((a, b) => compareEntries(a, b, col, dir));
  if (probeEnabled()) {
    const ms = (performance.now() - t0).toFixed(1);
    console.info(`[AFM_perf] list_sort ${ms}ms n=${entries.length} col=${col}`);
  }
  sortCache = { source: entries, column: col, direction, result };
  return result;
}
