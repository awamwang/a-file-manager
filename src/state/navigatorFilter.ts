// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator folder filter bar (AIA-88) — local name/path match only. */

import { ref } from "vue";

export const navigatorFilterQuery = ref("");
export const navigatorFilterOpen = ref(false);

export function openNavigatorFilter(seed = ""): void {
  navigatorFilterOpen.value = true;
  if (seed) navigatorFilterQuery.value = seed;
}

export function closeNavigatorFilter(): void {
  navigatorFilterOpen.value = false;
  navigatorFilterQuery.value = "";
}

export function toggleNavigatorFilter(): void {
  if (navigatorFilterOpen.value) {
    closeNavigatorFilter();
  } else {
    openNavigatorFilter();
  }
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
