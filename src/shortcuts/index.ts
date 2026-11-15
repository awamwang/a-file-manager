// SPDX-License-Identifier: GPL-3.0-or-later

export {
  chordFromEvent,
  chordKey,
  chordToAria,
  eventMatchesChord,
  formatChordLabel,
  hasChordModifier,
  hasDomTextSelection,
  isEditableTarget,
  isModifierKey,
  isReservedBareKey,
  normalizeEventKey,
} from "./chord.ts";
export {
  SHORTCUT_ACTIONS,
  FIXED_CHORD_KEYS,
  BUILTIN_SHORTCUTS,
  actionIdFor,
  ariaKeyshortcutsForId,
  defaultChordsForAction,
  effectiveChordLabelsForId,
  effectiveChordsForId,
  effectiveShortcutDefinitions,
  findShortcutAction,
  findShortcutConflicts,
  listShortcutActionsForDisplay,
  listShortcutsForDisplay,
  resolveDispatchMap,
  validateNewChord,
} from "./registry.ts";
export {
  MAX_CHORDS_PER_ACTION,
  SHORTCUT_OVERRIDES_STORAGE_KEY,
  getShortcutOverrides,
  loadShortcutOverrides,
  normalizeShortcutOverrides,
  resetActionChords,
  resetAllShortcutOverrides,
  setActionChords,
  shortcutOverridesVersion,
} from "./config.ts";
export {
  chordKeyForId,
  dispatchShortcutEvent,
  ownerForChordKey,
  registerShortcutHandler,
  unregisterShortcutHandler,
} from "./runtime.ts";
export type {
  ShortcutAction,
  ShortcutActionBinding,
  ShortcutActionRow,
  ShortcutChord,
  ShortcutConflict,
  ShortcutDefinition,
  ShortcutHandler,
  ShortcutOverrides,
  ShortcutScope,
  ShortcutValidationReason,
  ShortcutValidationResult,
} from "./types.ts";
