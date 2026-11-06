// SPDX-License-Identifier: GPL-3.0-or-later
/** Session pin set for Navigator strip tabs. Survives Work Group switches, not restarts. */

import { ref } from "vue";

export const pinnedTabIds = ref<string[]>([]);

export function isTabPinned(id: string): boolean {
  return pinnedTabIds.value.includes(id);
}

export function togglePinnedTab(id: string): void {
  if (!id) return;
  pinnedTabIds.value = isTabPinned(id)
    ? pinnedTabIds.value.filter((item) => item !== id)
    : [...pinnedTabIds.value, id];
}

export function forgetPinnedTabs(ids: string[]): void {
  if (!ids.length) return;
  const drop = new Set(ids);
  pinnedTabIds.value = pinnedTabIds.value.filter((id) => !drop.has(id));
}
