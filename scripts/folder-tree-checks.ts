// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { directoryChildren, treePathActive } from "../src/state/folderTree.ts";

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

console.log("folder-tree checks OK.");
