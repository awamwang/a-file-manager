// SPDX-License-Identifier: GPL-3.0-or-later

export {
  chordKey,
  eventMatchesChord,
  formatChordLabel,
  hasDomTextSelection,
  isEditableTarget,
  normalizeEventKey,
} from "./chord.ts";
export {
  BUILTIN_SHORTCUTS,
  findShortcutConflicts,
  listShortcutsForDisplay,
  resolveDispatchMap,
} from "./registry.ts";
export {
  chordKeyForId,
  dispatchShortcutEvent,
  ownerForChordKey,
  registerShortcutHandler,
  unregisterShortcutHandler,
} from "./runtime.ts";
export type {
  ShortcutChord,
  ShortcutConflict,
  ShortcutDefinition,
  ShortcutHandler,
  ShortcutScope,
} from "./types.ts";
