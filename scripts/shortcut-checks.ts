// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { chordKey, eventMatchesChord } from "../src/shortcuts/chord.ts";
import {
  BUILTIN_SHORTCUTS,
  findShortcutConflicts,
  listShortcutsForDisplay,
  resolveDispatchMap,
} from "../src/shortcuts/registry.ts";
import { isToggleCommandPanelShortcut } from "../src/state/commandPanel.ts";
import { isGlobalSearchShortcut } from "../src/state/globalSearch.ts";

assert.ok(BUILTIN_SHORTCUTS.length >= 6);
assert.equal(findShortcutConflicts().length, 0, "builtins must not conflict");

const winners = resolveDispatchMap();
assert.equal(
  winners.get(chordKey({ ctrl: true, shift: true, key: "p" }))?.id,
  "commandPanel.toggle",
);

const conflicted = findShortcutConflicts([
  ...BUILTIN_SHORTCUTS,
  {
    id: "dup.panel",
    labelKey: "x",
    chord: { ctrl: true, shift: true, key: "p" },
    scope: "global",
  },
]);
assert.equal(conflicted.length, 1);
assert.deepEqual(conflicted[0].ids.sort(), ["commandPanel.toggle", "dup.panel"]);

const display = listShortcutsForDisplay([
  ...BUILTIN_SHORTCUTS,
  {
    id: "dup.panel",
    labelKey: "x",
    chord: { ctrl: true, shift: true, key: "p" },
    scope: "global",
  },
]);
assert.equal(display.find((r) => r.id === "dup.panel")?.blockedBy, "commandPanel.toggle");

function keyEvent(partial: Partial<KeyboardEvent>): KeyboardEvent {
  return {
    key: "P",
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    isComposing: false,
    ...partial,
  } as KeyboardEvent;
}

assert.equal(
  isToggleCommandPanelShortcut(
    keyEvent({ ctrlKey: true, shiftKey: true, key: "P" }),
  ),
  true,
);
assert.equal(
  isGlobalSearchShortcut(keyEvent({ ctrlKey: true, shiftKey: true, key: "f" })),
  true,
);
assert.equal(
  eventMatchesChord(keyEvent({ ctrlKey: true, key: "l" }), {
    ctrl: true,
    key: "l",
  }),
  true,
);

console.log(
  `Shortcut registry checks OK (${BUILTIN_SHORTCUTS.length} builtins, conflict + match).`,
);
