// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { expandEnvPath } from "../src/state/expandEnvPath.ts";
import { BUILTIN_SHORTCUTS, resolveDispatchMap } from "../src/shortcuts/registry.ts";
import { chordKey } from "../src/shortcuts/chord.ts";

assert.equal(
  expandEnvPath("%USERPROFILE%\\Docs", { USERPROFILE: "C:\\Users\\me" }),
  "C:\\Users\\me\\Docs",
);
assert.equal(expandEnvPath("C:\\Temp", {}), "C:\\Temp");
assert.equal(expandEnvPath("%MISSING%\\x", {}), "%MISSING%\\x");

const winners = resolveDispatchMap(BUILTIN_SHORTCUTS);
assert.equal(winners.get(chordKey({ alt: true, key: "d" }))?.id, "chrome.focusAddressAltD");
assert.equal(winners.get(chordKey({ key: "f4" }))?.id, "chrome.focusAddressF4");

console.log("address-env checks OK (AIA-196).");
