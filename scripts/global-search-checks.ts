// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  consumeGlobalSearchFocus,
  isGlobalSearchShortcut,
  requestGlobalSearchFocus,
  globalSearchFocusToken,
} from "../src/state/globalSearch.ts";
import { isToggleCommandPanelShortcut } from "../src/state/commandPanel.ts";

function keyEvent(partial: Partial<KeyboardEvent>): KeyboardEvent {
  return {
    key: "F",
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    isComposing: false,
    ...partial,
  } as KeyboardEvent;
}

assert.equal(
  isGlobalSearchShortcut(keyEvent({ ctrlKey: true, shiftKey: true, key: "F" })),
  true,
);
assert.equal(
  isGlobalSearchShortcut(keyEvent({ ctrlKey: true, shiftKey: true, key: "f" })),
  true,
);
assert.equal(isGlobalSearchShortcut(keyEvent({ ctrlKey: true, key: "f" })), false);
assert.equal(
  isGlobalSearchShortcut(
    keyEvent({ ctrlKey: true, shiftKey: true, key: "F", isComposing: true }),
  ),
  false,
);

// Must not collide with Command Panel shortcut.
assert.equal(
  isToggleCommandPanelShortcut(
    keyEvent({ ctrlKey: true, shiftKey: true, key: "F" }),
  ),
  false,
);
assert.equal(
  isGlobalSearchShortcut(keyEvent({ ctrlKey: true, shiftKey: true, key: "P" })),
  false,
);

const before = globalSearchFocusToken.value;
assert.equal(consumeGlobalSearchFocus(), false);
requestGlobalSearchFocus();
assert.equal(globalSearchFocusToken.value, before + 1);
assert.equal(consumeGlobalSearchFocus(), true);
assert.equal(consumeGlobalSearchFocus(), false);

console.log("Global search shortcut checks OK (Ctrl+Shift+F ≠ Ctrl+Shift+P).");
