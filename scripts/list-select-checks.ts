// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  clearSelection,
  selectOnly,
  selectRange,
  selectedPaths,
  setSelection,
  toggleInSelection,
} from "../src/state/navigatorSelection.ts";
import {
  sortedEntries,
  sortColumn,
  sortDirection,
  toggleSort,
} from "../src/state/navigatorSort.ts";

clearSelection();
selectOnly("a");
assert.deepEqual(selectedPaths.value, ["a"]);
toggleInSelection("b");
assert.deepEqual(selectedPaths.value, ["a", "b"]);
toggleInSelection("a");
assert.deepEqual(selectedPaths.value, ["b"]);

const ordered = ["p1", "p2", "p3", "p4"];
selectOnly("p2");
selectRange(ordered, "p2", "p4");
assert.deepEqual(selectedPaths.value, ["p2", "p3", "p4"]);
setSelection(["p1", "p3"], "p1");
assert.deepEqual(selectedPaths.value, ["p1", "p3"]);

const entries = [
  { name: "b.txt", path: "C:\\b.txt", kind: "file" as const, size: 20 },
  { name: "a-dir", path: "C:\\a-dir", kind: "dir" as const, size: 0 },
  { name: "a.txt", path: "C:\\a.txt", kind: "file" as const, size: 10 },
];
sortColumn.value = "name";
sortDirection.value = "asc";
const byName = sortedEntries(entries);
assert.equal(byName[0].name, "a-dir");
assert.equal(byName[1].name, "a.txt");
assert.equal(byName[2].name, "b.txt");

toggleSort("size");
assert.equal(sortColumn.value, "size");
const bySize = sortedEntries(entries);
assert.equal(bySize[0].kind, "dir");
assert.equal(bySize[1].name, "a.txt");
assert.equal(bySize[2].name, "b.txt");

console.log("List select / sort checks OK (AIA-64).");
