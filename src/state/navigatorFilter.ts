// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator folder search — recursive name match under the open directory. */

import { ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { isGuidePath } from "./guidePath";
import type { NavigatorEntry } from "./navigatorBrowse";
import { showHiddenItems } from "./showHidden";

export const navigatorFilterQuery = ref("");

export type FolderSearchResult = {
  path: string;
  query: string;
  entries: NavigatorEntry[];
  truncated: boolean;
};

export const primarySearch = shallowRef<FolderSearchResult | null>(null);
export const secondarySearch = shallowRef<FolderSearchResult | null>(null);
export const primarySearchLoading = ref(false);
export const secondarySearchLoading = ref(false);

const SEARCH_DEBOUNCE_MS = 250;
let primaryTimer: ReturnType<typeof setTimeout> | null = null;
let secondaryTimer: ReturnType<typeof setTimeout> | null = null;
let primarySeq = 0;
let secondarySeq = 0;

const FILTER_INPUT_ID = "navigator-folder-filter";

/** Focus the always-visible toolbar search input (Ctrl+E / F3). */
export function focusNavigatorFilter(seed = ""): void {
  if (seed) navigatorFilterQuery.value = seed;
  requestAnimationFrame(() => {
    document.getElementById(FILTER_INPUT_ID)?.focus();
  });
}

/** Clear the in-folder search query (Escape / clear control / folder change). */
export function clearNavigatorFilter(): void {
  navigatorFilterQuery.value = "";
  primarySearch.value = null;
  secondarySearch.value = null;
  primarySearchLoading.value = false;
  secondarySearchLoading.value = false;
  if (primaryTimer) clearTimeout(primaryTimer);
  if (secondaryTimer) clearTimeout(secondaryTimer);
  primaryTimer = null;
  secondaryTimer = null;
}

/** Debounced recursive search of `path`. Empty query clears that pane's hits. */
export function scheduleFolderSearch(
  pane: "primary" | "secondary",
  path: string,
): void {
  const query = navigatorFilterQuery.value.trim();
  const hits = pane === "primary" ? primarySearch : secondarySearch;
  const loading = pane === "primary" ? primarySearchLoading : secondarySearchLoading;
  const timerSlot = pane === "primary" ? "primary" : "secondary";
  if (!query || !path.trim() || isGuidePath(path)) {
    hits.value = null;
    loading.value = false;
    clearTimer(timerSlot);
    return;
  }
  loading.value = true;
  hits.value = null;
  clearTimer(timerSlot);
  const delay = setTimeout(() => {
    void runFolderSearch(pane, path, query);
  }, SEARCH_DEBOUNCE_MS);
  if (pane === "primary") primaryTimer = delay;
  else secondaryTimer = delay;
}

function clearTimer(pane: "primary" | "secondary") {
  if (pane === "primary") {
    if (primaryTimer) clearTimeout(primaryTimer);
    primaryTimer = null;
  } else {
    if (secondaryTimer) clearTimeout(secondaryTimer);
    secondaryTimer = null;
  }
}

async function runFolderSearch(
  pane: "primary" | "secondary",
  path: string,
  query: string,
) {
  const seq = pane === "primary" ? ++primarySeq : ++secondarySeq;
  const loading = pane === "primary" ? primarySearchLoading : secondarySearchLoading;
  const hits = pane === "primary" ? primarySearch : secondarySearch;
  try {
    const result = await invoke<FolderSearchResult>("navigator_search", {
      path,
      query,
      showHidden: showHiddenItems.value,
    });
    const current = pane === "primary" ? primarySeq : secondarySeq;
    if (seq !== current) return;
    if (navigatorFilterQuery.value.trim() !== query) return;
    hits.value = result;
  } catch {
    const current = pane === "primary" ? primarySeq : secondarySeq;
    if (seq !== current) return;
    hits.value = { path, query, entries: [], truncated: false };
  } finally {
    const current = pane === "primary" ? primarySeq : secondarySeq;
    if (seq === current) loading.value = false;
  }
}

/** Alias used by shell shortcuts — focuses the toolbar input. */
export function openNavigatorFilter(seed = ""): void {
  focusNavigatorFilter(seed);
}

/** Alias used when leaving Navigator — clears the query. */
export function closeNavigatorFilter(): void {
  clearNavigatorFilter();
}
