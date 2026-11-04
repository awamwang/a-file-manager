// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator selection set (single-select Day-1; multi-select expands in AIA-64). */

import { computed, ref } from "vue";
import type { NavigatorEntry } from "./navigatorBrowse";

export const selectedPaths = ref<string[]>([]);

export const primarySelectedPath = computed(() => selectedPaths.value[0] ?? null);

export function selectOnly(path: string): void {
  selectedPaths.value = [path];
}

export function clearSelection(): void {
  selectedPaths.value = [];
}

export function isSelected(path: string): boolean {
  return selectedPaths.value.includes(path);
}

export function selectedEntries(
  entries: NavigatorEntry[] | null | undefined,
): NavigatorEntry[] {
  if (!entries?.length || !selectedPaths.value.length) return [];
  const set = new Set(selectedPaths.value);
  return entries.filter((e) => set.has(e.path));
}
