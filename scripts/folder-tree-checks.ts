// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  directoryChildren,
  treePathActive,
  treePathExact,
  treeRowHighlight,
} from "../src/state/folderTree.ts";

assert.deepEqual(
  directoryChildren([
    { name: "b.txt", path: "C:\\Temp\\afm-fixture\\b.txt", kind: "file" },
    { name: "zeta", path: "C:\\Temp\\afm-fixture\\zeta", kind: "dir" },
    { name: "Alpha", path: "C:\\Temp\\afm-fixture\\Alpha", kind: "dir" },
  ]).map((item) => item.name),
  ["Alpha", "zeta"],
);

assert.equal(treePathActive("C:\\Temp\\afm-fixture\\a", "C:\\Temp\\afm-fixture"), true);
assert.equal(treePathActive("C:\\Temp\\afm-fixture", "C:\\Temp\\afm-fixture"), true);
assert.equal(treePathActive("C:\\Temp\\afm-fixture", "D:\\Temp\\afm-fixture"), false);
assert.equal(treePathActive("C:\\Temp\\afm-fixture2", "C:\\Temp\\afm-fixture"), false);
assert.equal(treePathActive("C:\\Temp\\afm-fixture\\work\\file.txt", "C:\\"), true);

// AIA-230: entry roots never sticky-select on descendant / partial hover match.
assert.equal(treePathExact("C:\\Users\\me\\Documents\\work", "C:\\Users\\me\\Documents"), false);
assert.equal(treePathExact("C:\\Users\\me\\Documents", "C:\\Users\\me\\Documents"), true);
assert.equal(treePathExact("C:\\Users\\me\\Documents", "C:\\Users\\me\\documents"), true);
assert.equal(
  treeRowHighlight("C:\\Users\\me\\Documents\\a", "C:\\Users\\me\\Documents", "entry"),
  "none",
);
assert.equal(
  treeRowHighlight("C:\\Users\\me\\Documents", "C:\\Users\\me\\Documents", "entry"),
  "current",
);
assert.equal(
  treeRowHighlight("C:\\Users\\me\\Documents\\a", "C:\\Users\\me\\Documents", "tree"),
  "selected",
);
assert.equal(
  treeRowHighlight("C:\\Users\\me\\Documents", "C:\\Users\\me\\Documents", "tree"),
  "selected",
);

console.log("folder-tree checks OK.");

await import("./folder-tree-section-checks.ts");
