// SPDX-License-Identifier: GPL-3.0-or-later
// Pure-logic checks for paste-target resolution (CLIP-101 / CLIP-102 / CLIP-104).
import assert from "node:assert/strict";
import {
  parentDirectory,
  resolvePasteTarget,
} from "../src/state/pasteTarget.ts";

const CUR = "C:\\Temp\\here";

// CLIP-101: a single selected folder → paste into that folder.
assert.equal(
  resolvePasteTarget([{ path: "C:\\Temp\\here\\sub", isDirectory: true }], CUR),
  "C:\\Temp\\here\\sub",
);

// CLIP-102: a single selected file → paste into the file's parent dir.
assert.equal(
  resolvePasteTarget([{ path: "C:\\Temp\\here\\a.txt", isDirectory: false }], CUR),
  "C:\\Temp\\here",
);

// CLIP-104: marquee over a mixed selection (not a single container) → current dir.
assert.equal(
  resolvePasteTarget(
    [
      { path: "C:\\Temp\\here\\sub", isDirectory: true },
      { path: "C:\\Temp\\here\\a.txt", isDirectory: false },
    ],
    CUR,
  ),
  CUR,
);

// CLIP-103: nothing selected → current dir.
assert.equal(resolvePasteTarget([], CUR), CUR);

// Single file at a drive root falls back to the resolved root parent, never "".
assert.equal(parentDirectory("C:\\x.txt"), "C:");
// A bare name has no navigable parent → resolver keeps the current dir.
assert.equal(
  resolvePasteTarget([{ path: "loose.txt", isDirectory: false }], CUR),
  CUR,
);

// Forward slashes and trailing separators normalize to the same parent.
assert.equal(parentDirectory("C:/Temp/here/a.txt"), "C:\\Temp\\here");

console.log("Paste target checks OK (CLIP-101/102/104).");
