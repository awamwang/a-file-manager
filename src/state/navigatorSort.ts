// SPDX-License-Identifier: GPL-3.0-or-later
/** Client-side Navigator list sort (AIA-64) + persisted sort field/direction (schema v17). */

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { NavigatorEntry } from "./navigatorBrowse.ts";
import { entryTypeKey } from "./entryType.ts";
import { rememberSettingsDocument } from "./settingsHydrate.ts";

export type SortColumn = "name" | "size" | "modified" | "type";
export type SortDirection = "asc" | "desc";

export const sortColumn = ref<SortColumn>("name");
export const sortDirection = ref<SortDirection>("asc");

const SORT_COLUMNS: readonly SortColumn[] = ["name", "size", "modified", "type"];

/** 排序菜单字段（Win11 顺序：名称/修改日期/类型/大小）。 */
export const NAVIGATOR_SORT_COLUMNS: readonly SortColumn[] = [
  "name",
  "modified",
  "type",
  "size",
];

export function sortColumnLabelKey(column: SortColumn): string {
  switch (column) {
    case "name":
      return "chrome.sortName";
    case "size":
      return "chrome.sortSize";
    case "modified":
      return "chrome.sortModified";
    case "type":
      return "chrome.sortType";
  }
}

function coerceColumn(value: unknown): SortColumn {
  return typeof value === "string" && (SORT_COLUMNS as readonly string[]).includes(value)
    ? (value as SortColumn)
    : "name";
}

function coerceDirection(value: unknown): SortDirection {
  return value === "desc" ? "desc" : "asc";
}

/** Sync apply from already-hydrated settings (cold-start critical path). */
export function applyNavigatorSortFromValues(values: Record<string, unknown>): void {
  sortColumn.value = coerceColumn(values["core.navigatorSortColumn"]);
  sortDirection.value = coerceDirection(values["core.navigatorSortDirection"]);
}

function persistSortPatch(
  patch: Partial<
    Record<"core.navigatorSortColumn" | "core.navigatorSortDirection", string>
  >,
): void {
  void invoke<{ version: number; values: Record<string, unknown> }>("settings_set", {
    patch,
  })
    .then((doc) => rememberSettingsDocument(doc))
    .catch(() => {
      // Persist 失败保留本地值；下次启动以后端文档为准。
    });
}

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

/** Windows 式显式选列（下拉/右键菜单）：保持当前方向不变。 */
export function setSortColumn(column: SortColumn): void {
  if (sortColumn.value === column) return;
  sortColumn.value = column;
  sortCache = null;
  persistSortPatch({ "core.navigatorSortColumn": column });
}

/** Windows 式方向单选（递增/递减）。 */
export function setSortDirection(direction: SortDirection): void {
  if (sortDirection.value === direction) return;
  sortDirection.value = direction;
  sortCache = null;
  persistSortPatch({ "core.navigatorSortDirection": direction });
}

export function toggleSort(column: SortColumn): void {
  if (sortColumn.value === column) {
    sortDirection.value = sortDirection.value === "asc" ? "desc" : "asc";
  } else {
    sortColumn.value = column;
    sortDirection.value = "asc";
  }
  sortCache = null;
  persistSortPatch({
    "core.navigatorSortColumn": sortColumn.value,
    "core.navigatorSortDirection": sortDirection.value,
  });
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
  const nameCmp = sortNameKey(a).localeCompare(sortNameKey(b), undefined, {
    sensitivity: "base",
  });
  return nameCmp * dir;
}

/**
 * ADR 0031: drill-through rows sort by their root-relative location
 * (`sub1\file.txt`), so the flattened list keeps its hierarchy. For ordinary
 * listings the relative path IS the file name — the comparison is unchanged.
 */
function sortNameKey(entry: NavigatorEntry): string {
  return entry.relativePath || entry.name;
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
