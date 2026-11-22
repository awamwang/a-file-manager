// SPDX-License-Identifier: GPL-3.0-or-later
// Behavioral unit for the name-conflict "replace / skip / keep" gate and the
// per-batch "Apply to all" contract (CF-003 / TR-014). Pinned by scripts/check-name-conflict.mjs.
// nameConflict.ts only depends on `vue`'s reactivity `ref`, which resolves
// headless here, so no localStorage shim is needed (and must NOT exist —
// apply-to-all is never persisted).
import assert from "node:assert/strict";

const {
  resetNameConflictBatch,
  currentRememberedChoice,
  setNameConflictApplyToAll,
  openNameConflictDialog,
  resolveNameConflict,
  resolveNameConflictChoice,
  nameConflictApplyToAll,
  nameConflictOpen,
} = await import("../src/state/nameConflict.ts");

// Fresh batch starts empty — no remembered choice leaks in.
resetNameConflictBatch();
assert.equal(currentRememberedChoice(), null, "fresh batch has no remembered choice");

// Without "Apply to all", resolving only returns the choice — it does NOT
// remember it for subsequent conflicts in the same batch.
void openNameConflictDialog("a.txt");
setNameConflictApplyToAll(false);
resolveNameConflict("replace");
assert.equal(currentRememberedChoice(), null, "un-checked apply-to-all does not remember");

// With "Apply to all", the choice short-circuits later conflicts in the batch.
void openNameConflictDialog("b.txt");
setNameConflictApplyToAll(true);
resolveNameConflict("skip");
assert.equal(currentRememberedChoice(), "skip", "checked apply-to-all remembers the choice");

// The remembered choice is returned without opening a dialog (resolve returns
// synchronously from the remembered branch; no pending dialog exists).
assert.equal(nameConflictApplyToAll.value, false, "apply-to-all flag cleared on settle");

// A remembered choice short-circuits resolveNameConflictChoice — no second
// prompt is shown for the next conflicting file in the same paste.
assert.equal(await resolveNameConflictChoice("b2.txt"), "skip", "remembered skip short-circuits");
assert.equal(nameConflictOpen.value, false, "short-circuit does not reopen the dialog");

// Cancelling never remembers, even with the checkbox ticked.
void openNameConflictDialog("c.txt");
setNameConflictApplyToAll(true);
resolveNameConflict("cancel");
assert.equal(currentRememberedChoice(), "skip", "cancel does not overwrite the remembered choice");

// Batch boundary: resetNameConflictBatch (what transferEntries calls per paste)
// forgets everything — the next paste asks again.
resetNameConflictBatch();
assert.equal(currentRememberedChoice(), null, "reset clears the remembered choice — next paste re-asks");

// Keep-both is a valid remembered choice too (not just replace/skip).
void openNameConflictDialog("d.txt");
setNameConflictApplyToAll(true);
resolveNameConflict("keep");
assert.equal(currentRememberedChoice(), "keep", "keep-both is rememberable");

console.log(
  "name-conflict-checks OK (fresh batch empty, unchecked no-remember, checked remembers, cancel-safe, batch reset, keep rememberable).",
);
