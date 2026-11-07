// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { BUILTIN_SHORTCUTS, resolveDispatchMap } from "../src/shortcuts/registry.ts";
import { chordKey } from "../src/shortcuts/chord.ts";

/**
 * Mirror of former `src/state/expandEnvPath` (AIA-270 removed — address bar
 * expands via Rust `navigator_expand_path`). Keep a tiny pure check so the
 * %VAR% contract stays documented for scripts.
 */
function expandEnvPath(
  input: string,
  env: Record<string, string | undefined>,
): string {
  return input.replace(/%([^%]+)%/g, (match, name: string) => {
    const key = name.trim();
    if (!key) return match;
    const direct = env[key];
    if (direct != null && direct !== "") return direct;
    const upper = env[key.toUpperCase()];
    if (upper != null && upper !== "") return upper;
    return match;
  });
}

assert.equal(
  expandEnvPath("%USERPROFILE%\\Docs", { USERPROFILE: "C:\\Temp\\afm-fixture\\home" }),
  "C:\\Temp\\afm-fixture\\home\\Docs",
);
assert.equal(expandEnvPath("C:\\Temp", {}), "C:\\Temp");
assert.equal(expandEnvPath("%MISSING%\\x", {}), "%MISSING%\\x");

const winners = resolveDispatchMap(BUILTIN_SHORTCUTS);
assert.equal(winners.get(chordKey({ alt: true, key: "d" }))?.id, "chrome.focusAddressAltD");
assert.equal(winners.get(chordKey({ key: "f4" }))?.id, "chrome.focusAddressF4");

console.log("address-env checks OK (AIA-196).");
