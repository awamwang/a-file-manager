// SPDX-License-Identifier: GPL-3.0-or-later
/** Session pin for Navigator strip tabs. At most one pin; survives WG switches, not restarts. */

import { ref } from "vue";

export const pinnedTabIds = ref<string[]>([]);

/** Pick one id from a multi-pin list: preferred if present, else first. */
export function pickSinglePinnedId(
  ids: readonly string[],
  preferredId?: string | null,
): string | null {
  const cleaned = ids.filter((id) => Boolean(id));
  if (!cleaned.length) return null;
  if (preferredId && cleaned.includes(preferredId)) return preferredId;
  return cleaned[0] ?? null;
}

/** Collapse session pin state to ≤1 id (AIA-260). */
export function ensureSinglePinnedTab(preferredId?: string | null): void {
  const id = pickSinglePinnedId(pinnedTabIds.value, preferredId);
  pinnedTabIds.value = id ? [id] : [];
}

/** Session pin set for OpenState helpers — always ≤1 id. */
export function pinnedIdSet(preferredId?: string | null): Set<string> {
  ensureSinglePinnedTab(preferredId);
  const id = pinnedTabIds.value[0];
  return id ? new Set([id]) : new Set();
}

export function isTabPinned(id: string): boolean {
  if (!id) return false;
  return pinnedTabIds.value.includes(id);
}

/** Pin replaces any previous pin; pin again on the same tab unpins. */
export function togglePinnedTab(id: string): void {
  if (!id) return;
  pinnedTabIds.value = isTabPinned(id) ? [] : [id];
}

export function forgetPinnedTabs(ids: string[]): void {
  if (!ids.length) return;
  const drop = new Set(ids);
  pinnedTabIds.value = pinnedTabIds.value.filter((id) => id && !drop.has(id));
}
