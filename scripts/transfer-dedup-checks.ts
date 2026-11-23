// SPDX-License-Identifier: GPL-3.0-or-later
// Pure-logic checks for transfer-source dedup (SEL-007 / SEL-011).
import assert from "node:assert/strict";
import { dedupeDescendantSources } from "../src/state/transferDedup.ts";

// A folder and one of its descendants → only the folder is carried.
assert.deepEqual(
  dedupeDescendantSources(["C:\\A", "C:\\A\\sub"]),
  ["C:\\A"],
);
// Order-independent: descendant listed first still collapses onto the ancestor.
assert.deepEqual(
  dedupeDescendantSources(["C:\\A\\sub", "C:\\A"]),
  ["C:\\A"],
);
// Deep chains inside an ancestor are all dropped.
assert.deepEqual(
  dedupeDescendantSources(["C:\\A", "C:\\A\\sub", "C:\\A\\sub\\deep"]),
  ["C:\\A"],
);
// Siblings are unrelated and both kept.
assert.deepEqual(
  dedupeDescendantSources(["C:\\A", "C:\\B"]),
  ["C:\\A", "C:\\B"],
);
// Case-insensitive FS: descendant matches the ancestor regardless of casing.
assert.deepEqual(
  dedupeDescendantSources(["C:\\A", "c:\\a\\Sub"]),
  ["C:\\A"],
);
// Prefix must respect the segment boundary: "C:\A2" is NOT inside "C:\A".
assert.deepEqual(
  dedupeDescendantSources(["C:\\A", "C:\\A2"]),
  ["C:\\A", "C:\\A2"],
);
// Exact duplicates collapse (keep first occurrence).
assert.deepEqual(
  dedupeDescendantSources(["C:\\A", "C:\\A"]),
  ["C:\\A"],
);
// A volume root as a source absorbs everything beneath it.
assert.deepEqual(
  dedupeDescendantSources(["C:\\", "C:\\Temp\\x"]),
  ["C:\\"],
);

console.log("Transfer dedup checks OK (SEL-007/011).");
