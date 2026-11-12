// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  baseName,
  dirsAffectedByUndo,
  moveUndoRecord,
  parentDir,
  popRecord,
  pushRecord,
  renameUndoRecord,
} from "../src/state/fileUndo/stack.ts";
import { BUILTIN_SHORTCUTS, resolveDispatchMap } from "../src/shortcuts/registry.ts";
import { chordKey } from "../src/shortcuts/chord.ts";

assert.equal(parentDir("C:\\Temp\\afm-fixture\\a\\b.txt"), "C:\\Temp\\afm-fixture\\a");
assert.equal(baseName("C:\\Temp\\afm-fixture\\a\\b.txt"), "b.txt");

const rename = renameUndoRecord("C:\\Temp\\afm-fixture\\a\\old.txt", "C:\\Temp\\afm-fixture\\a\\new.txt");
assert.equal(rename.kind, "rename");
if (rename.kind === "rename") {
  assert.equal(rename.previousName, "old.txt");
  assert.equal(rename.currentPath, "C:\\Temp\\afm-fixture\\a\\new.txt");
}

const move = moveUndoRecord(
  ["C:\\Temp\\afm-fixture\\src\\a.txt", "C:\\Temp\\afm-fixture\\src\\b.txt"],
  ["D:\\Temp\\afm-fixture\\dst\\a.txt", "D:\\Temp\\afm-fixture\\dst\\b.txt"],
);
assert.equal(move.kind, "move");
if (move.kind === "move") {
  assert.equal(move.items[0]?.originalDir, "C:\\Temp\\afm-fixture\\src");
  assert.equal(move.items[1]?.currentPath, "D:\\Temp\\afm-fixture\\dst\\b.txt");
}

assert.deepEqual(dirsAffectedByUndo(rename), ["C:\\Temp\\afm-fixture\\a"]);
assert.deepEqual(dirsAffectedByUndo(move), [
  "D:\\Temp\\afm-fixture\\dst",
  "C:\\Temp\\afm-fixture\\src",
]);
assert.deepEqual(
  dirsAffectedByUndo({
    kind: "copy",
    createdPaths: ["C:\\Temp\\afm-fixture\\a\\copy.txt"],
  }),
  ["C:\\Temp\\afm-fixture\\a"],
);
assert.deepEqual(
  dirsAffectedByUndo({ kind: "create", path: "C:\\Temp\\afm-fixture\\a\\x.txt" }),
  ["C:\\Temp\\afm-fixture\\a"],
);
assert.deepEqual(
  dirsAffectedByUndo({
    kind: "delete",
    originalPaths: ["C:\\Temp\\afm-fixture\\a\\gone.txt"],
  }),
  ["C:\\Temp\\afm-fixture\\a"],
);

let stack = pushRecord([], rename);
stack = pushRecord(stack, { kind: "create", path: "C:\\Temp\\afm-fixture\\a\\x.txt" });
assert.equal(stack.length, 2);
const popped = popRecord(stack);
assert.equal(popped.record?.kind, "create");
assert.equal(popped.stack.length, 1);

const winners = resolveDispatchMap(BUILTIN_SHORTCUTS);
assert.equal(winners.get(chordKey({ ctrl: true, key: "z" }))?.id, "navigator.undo");

console.log("file-undo checks OK (AIA-192 / FIL-61).");
