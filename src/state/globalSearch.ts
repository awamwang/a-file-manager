// SPDX-License-Identifier: GPL-3.0-or-later

import { ref } from "vue";

/** Bumped when Ctrl+Shift+F requests Discovery search focus. */
export const globalSearchFocusToken = ref(0);
const pendingFocus = ref(false);

export function requestGlobalSearchFocus(): void {
  pendingFocus.value = true;
  globalSearchFocusToken.value += 1;
}

/** Returns true once per request; clears the pending flag. */
export function consumeGlobalSearchFocus(): boolean {
  if (!pendingFocus.value) return false;
  pendingFocus.value = false;
  return true;
}

/** Match Ctrl+Shift+F (global Discovery search). Distinct from Command Panel Ctrl+Shift+P. */
export function isGlobalSearchShortcut(event: KeyboardEvent): boolean {
  if (event.isComposing) return false;
  if (!event.ctrlKey || !event.shiftKey || event.altKey || event.metaKey) return false;
  return event.key === "F" || event.key === "f";
}
