// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readShowHidden } from "../src/state/showHidden/flag.ts";

assert.equal(readShowHidden({ "core.showHidden": true }), true);
assert.equal(readShowHidden({ "core.showHidden": false }), false);
assert.equal(readShowHidden({}), false);
assert.equal(readShowHidden(null), false);

console.log("show-hidden checks OK.");
