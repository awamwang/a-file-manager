// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { normalizeOpenToolExtensions } from "../src/state/openToolExtensions.ts";

// Empty / whitespace input = unscoped (empty list).
assert.deepEqual(normalizeOpenToolExtensions(""), []);
assert.deepEqual(normalizeOpenToolExtensions("   "), []);
// Comma / Chinese comma / whitespace separated, mixed case.
assert.deepEqual(normalizeOpenToolExtensions("zip, rar"), ["zip", "rar"]);
assert.deepEqual(normalizeOpenToolExtensions("*.ZIP, .rar, 7z"), ["zip", "rar", "7z"]);
assert.deepEqual(normalizeOpenToolExtensions("zip，rar 7z"), ["zip", "rar", "7z"]);
// Leading dots and `*.` prefixes are stripped; a lone `*` (any file) survives.
assert.deepEqual(normalizeOpenToolExtensions("*"), ["*"]);
assert.deepEqual(normalizeOpenToolExtensions("...mp4"), ["mp4"]);
assert.deepEqual(normalizeOpenToolExtensions("*, zip"), ["*", "zip"]);

console.log("Open tool extension normalize checks OK.");
