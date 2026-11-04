// SPDX-License-Identifier: GPL-3.0-or-later

import { ref } from "vue";
import { eventMatchesChord } from "../shortcuts/chord.ts";
import { BUILTIN_SHORTCUTS } from "../shortcuts/registry.ts";

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

const searchChord = BUILTIN_SHORTCUTS.find((s) => s.id === "discovery.globalSearch")!.chord;

/** Match Ctrl+Shift+F via central shortcut registry. Distinct from Command Panel. */
export function isGlobalSearchShortcut(event: KeyboardEvent): boolean {
  return eventMatchesChord(event, searchChord);
}
