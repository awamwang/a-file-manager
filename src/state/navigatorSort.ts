// SPDX-License-Identifier: GPL-3.0-or-later
/** Client-side Navigator list sort (AIA-64). */

import { ref } from "vue";
import type { NavigatorEntry } from "./navigatorBrowse";

export type SortColumn = "name" | "size";
export type SortDirection = "asc" | "desc";

export const sortColumn = ref<SortColumn>("name");
export const sortDirection = ref<SortDirection>("asc");

export function toggleSort(column: SortColumn): void {
  if (sortColumn.value === column) {
    sortDirection.value = sortDirection.value === "asc" ? "desc" : "asc";
  } else {
    sortColumn.value = column;
    sortDirection.value = "asc";
  }
}

export function sortedEntries(entries: NavigatorEntry[]): NavigatorEntry[] {
  const col = sortColumn.value;
  const dir = sortDirection.value === "asc" ? 1 : -1;
  return [...entries].sort((a, b) => {
    // Folders first, matching Kernel default, then column.
    if (a.kind !== b.kind) {
      if (a.kind === "dir") return -1;
      if (b.kind === "dir") return 1;
    }
    if (col === "size") {
      if (a.size !== b.size) return (a.size - b.size) * dir;
    }
    const nameCmp = a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
    return nameCmp * dir;
  });
}
