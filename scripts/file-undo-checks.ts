// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  baseName,
  moveUndoRecord,
  parentDir,
  popRecord,
  pushRecord,
  renameUndoRecord,
} from "../src/state/fileUndoStack.ts";
import { BUILTIN_SHORTCUTS, resolveDispatchMap } from "../src/shortcuts/registry.ts";
import { chordKey } from "../src/shortcuts/chord.ts";

assert.equal(parentDir("C:\\a\\b.txt"), "C:\\a");
assert.equal(baseName("C:\\a\\b.txt"), "b.txt");

const rename = renameUndoRecord("C:\\a\\old.txt", "C:\\a\\new.txt");
assert.equal(rename.kind, "rename");
if (rename.kind === "rename") {
  assert.equal(rename.previousName, "old.txt");
  assert.equal(rename.currentPath, "C:\\a\\new.txt");
}

const move = moveUndoRecord(
  ["C:\\src\\a.txt", "C:\\src\\b.txt"],
  ["D:\\dst\\a.txt", "D:\\dst\\b.txt"],
);
assert.equal(move.kind, "move");
if (move.kind === "move") {
  assert.equal(move.items[0]?.originalDir, "C:\\src");
  assert.equal(move.items[1]?.currentPath, "D:\\dst\\b.txt");
}

let stack = pushRecord([], rename);
stack = pushRecord(stack, { kind: "create", path: "C:\\a\\x.txt" });
assert.equal(stack.length, 2);
const popped = popRecord(stack);
assert.equal(popped.record?.kind, "create");
assert.equal(popped.stack.length, 1);

const winners = resolveDispatchMap(BUILTIN_SHORTCUTS);
assert.equal(winners.get(chordKey({ ctrl: true, key: "z" }))?.id, "navigator.undo");

console.log("file-undo checks OK (AIA-192).");
