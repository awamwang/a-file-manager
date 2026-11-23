// SPDX-License-Identifier: GPL-3.0-or-later
// Run via: node --experimental-strip-types scripts/action-tip-checks.ts
import assert from "node:assert/strict";
import { createActionTipInteractionGate } from "../src/shortcuts/actionTip.ts";

// Cold start on macOS: WKWebView may focus the first focusable control (the
// Command Panel button) while revealing the window, with no user input at all.
const cold = createActionTipInteractionGate();
assert.equal(
  cold.allows("focus"),
  false,
  "startup focus must not pop the floating shortcut hint",
);
assert.equal(
  cold.allows("hover"),
  true,
  "hover is a direct pointer signal and must still show the hint",
);

cold.markInteraction();
assert.equal(cold.allows("focus"), true, "focus after real input shows the hint");
assert.equal(cold.allows("hover"), true, "hover still works after input");

cold.markInteraction();
assert.equal(cold.allows("focus"), true, "repeated input marks idempotently");

const otherMount = createActionTipInteractionGate();
assert.equal(
  otherMount.allows("focus"),
  false,
  "a fresh mount must not inherit another gate's interaction state",
);
otherMount.markInteraction();
assert.equal(cold.allows("focus"), true, "gates must not leak state backwards");

console.log("action-tip interaction gate checks OK.");
