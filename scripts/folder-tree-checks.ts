// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { directoryChildren, treePathActive } from "../src/state/folderTree.ts";

assert.deepEqual(
  directoryChildren([
    { name: "b.txt", path: "C:\\b.txt", kind: "file" },
    { name: "zeta", path: "C:\\zeta", kind: "dir" },
    { name: "Alpha", path: "C:\\Alpha", kind: "dir" },
  ]).map((item) => item.name),
  ["Alpha", "zeta"],
);

assert.equal(treePathActive("C:\\Users\\a", "C:\\Users"), true);
assert.equal(treePathActive("C:\\Users", "C:\\Users"), true);
assert.equal(treePathActive("C:\\Users", "D:\\Users"), false);
assert.equal(treePathActive("C:\\Users2", "C:\\Users"), false);
assert.equal(treePathActive("C:\\work\\file.txt", "C:\\"), true);

console.log("folder-tree checks OK.");
