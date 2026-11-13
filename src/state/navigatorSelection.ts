// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator selection set with multi-select helpers (AIA-64). */

import { computed, ref } from "vue";
import type { NavigatorEntry } from "./navigatorBrowse.ts";

export const selectedPaths = ref<string[]>([]);
/** Anchor path for Shift+click ranges. */
export const selectionAnchor = ref<string | null>(null);

export const primarySelectedPath = computed(() => selectedPaths.value[0] ?? null);

export function selectOnly(path: string): void {
  selectedPaths.value = [path];
  selectionAnchor.value = path;
}

export function clearSelection(): void {
  selectedPaths.value = [];
  selectionAnchor.value = null;
}

export function isSelected(path: string): boolean {
  return selectedPaths.value.includes(path);
}

export function toggleInSelection(path: string): void {
  if (selectedPaths.value.includes(path)) {
    selectedPaths.value = selectedPaths.value.filter((p) => p !== path);
  } else {
    selectedPaths.value = [...selectedPaths.value, path];
  }
  selectionAnchor.value = path;
}

export function selectRange(
  orderedPaths: string[],
  fromPath: string | null,
  toPath: string,
): void {
  const toIndex = orderedPaths.indexOf(toPath);
  if (toIndex < 0) {
    selectOnly(toPath);
    return;
  }
  const fromIndex = fromPath ? orderedPaths.indexOf(fromPath) : toIndex;
  if (fromIndex < 0) {
    selectOnly(toPath);
    return;
  }
  const start = Math.min(fromIndex, toIndex);
  const end = Math.max(fromIndex, toIndex);
  selectedPaths.value = orderedPaths.slice(start, end + 1);
  // Keep the range origin unless none was set yet.
  if (!selectionAnchor.value || !orderedPaths.includes(selectionAnchor.value)) {
    selectionAnchor.value = fromPath && orderedPaths.includes(fromPath) ? fromPath : toPath;
  }
}

export function setSelection(paths: string[], anchor?: string | null): void {
  selectedPaths.value = [...paths];
  selectionAnchor.value = anchor ?? paths[0] ?? null;
}

export function selectedEntries(
  entries: NavigatorEntry[] | null | undefined,
): NavigatorEntry[] {
  if (!entries?.length || !selectedPaths.value.length) return [];
  const set = new Set(selectedPaths.value);
  return entries.filter((e) => set.has(e.path));
}
