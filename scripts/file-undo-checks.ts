// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  baseName,
  dirsAffectedByUndo,
  isContaminated,
  moveUndoRecord,
  parentDir,
  popRecord,
  pushRecord,
  renameUndoRecord,
  type Fingerprint,
  type UndoRecord,
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

// UN-006 (已拍板 2026-11-23): consecutive Ctrl+Z must unwind in strict reverse
// (LIFO) order. The stack is the real one production drives (pushRecord appends,
// popRecord returns the last). Each pop must reveal the immediately-earlier
// record so the *next* Ctrl+Z keeps unwinding until the stack is empty — i.e. an
// undo of op N never blocks the undo of op N-1. This pins copy→remove, move→back,
// rename→original, create→delete, recycle-delete→restore happening last-to-first.
let seq: UndoRecord[] = [];
seq = pushRecord(seq, { kind: "copy", createdPaths: ["C:\\Temp\\afm-fixture\\a\\note - Copy.txt"] });
seq = pushRecord(seq, { kind: "create", path: "C:\\Temp\\afm-fixture\\a\\New folder" });
seq = pushRecord(
  seq,
  renameUndoRecord("C:\\Temp\\afm-fixture\\a\\old.txt", "C:\\Temp\\afm-fixture\\a\\new.txt"),
);
seq = pushRecord(
  seq,
  moveUndoRecord(["C:\\Temp\\afm-fixture\\src\\x.bin"], ["D:\\Temp\\afm-fixture\\dst\\x.bin"]),
);
seq = pushRecord(seq, { kind: "delete", originalPaths: ["C:\\Temp\\afm-fixture\\a\\gone.txt"] });

const unwind: Array<UndoRecord["kind"]> = [];
for (;;) {
  const step = popRecord(seq);
  if (!step.record) break;
  unwind.push(step.record.kind);
  seq = step.stack;
}
assert.deepEqual(
  unwind,
  ["delete", "move", "rename", "create", "copy"],
  "Ctrl+Z ×n must undo last→first, each action reversible in turn",
);
assert.equal(seq.length, 0, "fully drained — no record is stranded by an earlier undo");

const winners = resolveDispatchMap(BUILTIN_SHORTCUTS);
assert.equal(winners.get(chordKey({ ctrl: true, key: "z" }))?.id, "navigator.undo");

// UN-008 (已拍板 2026-11-22): an undo whose target was rewritten by a foreign
// program must be refused. Only a real change (still-present path, different
// size/mtime) counts — no baseline, a vanished path, or an unchanged file must
// never block the undo.
const fp = (size: number, modifiedMs: number): Fingerprint => ({ size, modifiedMs });
assert.equal(isContaminated(undefined, [fp(1, 1)]), false, "no baseline captured → undo proceeds");
assert.equal(isContaminated([], [fp(1, 1)]), false, "empty baseline → undo proceeds");
assert.equal(isContaminated([null], [fp(1, 1)]), false, "unreadable baseline → undo proceeds");
assert.equal(isContaminated([fp(1, 1)], [fp(1, 1)]), false, "identical fingerprints → not contaminated");
assert.equal(isContaminated([fp(1, 1)], [null]), false, "path vanished externally → delete path handles it");
assert.equal(isContaminated([fp(1, 1)], [fp(2, 1)]), true, "size drift → contaminated");
assert.equal(isContaminated([fp(1, 1)], [fp(1, 2)]), true, "mtime drift → contaminated");
assert.equal(
  isContaminated([fp(1, 1), null], [fp(1, 1), fp(9, 9)]),
  false,
  "per-path: only the captured baselines are compared",
);
assert.equal(
  isContaminated([fp(1, 1), fp(2, 2)], [fp(1, 1), fp(3, 2)]),
  true,
  "one drifted path is enough to refuse",
);

console.log("file-undo checks OK (AIA-192 / FIL-61 / UN-006 reverse-order multi-undo / UN-008 contamination).");
