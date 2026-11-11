// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  keyboardNavDirection,
  mouseButtonNavDirection,
} from "../src/state/browserNavGestures.ts";

assert.equal(mouseButtonNavDirection(0), null);
assert.equal(mouseButtonNavDirection(1), null);
assert.equal(mouseButtonNavDirection(2), null);
assert.equal(mouseButtonNavDirection(3), "back");
assert.equal(mouseButtonNavDirection(4), "forward");
assert.equal(mouseButtonNavDirection(5), null);

assert.equal(keyboardNavDirection("BrowserBack"), "back");
assert.equal(keyboardNavDirection("browserback"), "back");
assert.equal(keyboardNavDirection("BrowserForward"), "forward");
assert.equal(keyboardNavDirection("ArrowLeft"), null);
assert.equal(keyboardNavDirection(""), null);

console.log("Browser nav gesture checks OK (FIL-51).");
