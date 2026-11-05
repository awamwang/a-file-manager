// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator folder filter (AIA-88 / AIA-147) — local name/path match only. */

import { ref } from "vue";

export const navigatorFilterQuery = ref("");

const FILTER_INPUT_ID = "navigator-folder-filter";

/** Focus the always-visible toolbar search input (Ctrl+Shift+F on Navigator). */
export function focusNavigatorFilter(seed = ""): void {
  if (seed) navigatorFilterQuery.value = seed;
  requestAnimationFrame(() => {
    document.getElementById(FILTER_INPUT_ID)?.focus();
  });
}

/** Clear the in-folder filter query (Escape / clear control). */
export function clearNavigatorFilter(): void {
  navigatorFilterQuery.value = "";
}

/** Alias used by shell shortcuts — focuses the toolbar input. */
export function openNavigatorFilter(seed = ""): void {
  focusNavigatorFilter(seed);
}

/** Alias used when leaving Navigator — clears the query. */
export function closeNavigatorFilter(): void {
  clearNavigatorFilter();
}

export function entryMatchesFilter(
  name: string,
  path: string,
  query: string,
): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return (
    name.toLowerCase().includes(q) || path.toLowerCase().replace(/\//g, "\\").includes(q)
  );
}
