// SPDX-License-Identifier: GPL-3.0-or-later
// Behavioral unit for the name-conflict "replace / skip / remember" gate. Pinned by scripts/check-name-conflict.mjs.
// nameConflict.ts imports only vue + a localStorage access that is called at runtime, so a tiny
// in-memory shim makes the whole remember/short-circuit contract testable without a browser.
import assert from "node:assert/strict";

// localStorage shim MUST exist before the storage-touching functions run (import itself is safe).
const store = new Map<string, string>();
(globalThis as unknown as { localStorage: unknown }).localStorage = {
  getItem: (k: string) => (store.has(k) ? store.get(k)! : null),
  setItem: (k: string, v: string) => void store.set(k, String(v)),
  removeItem: (k: string) => void store.delete(k),
  clear: () => store.clear(),
};

const {
  readNameConflictDefault,
  writeNameConflictDefault,
  setNameConflictRemember,
  openNameConflictDialog,
  resolveNameConflict,
  resolveNameConflictChoice,
  nameConflictRememberReplace,
  nameConflictRememberSkip,
} = await import("../src/state/nameConflict.ts");

// Unset / unknown persisted value falls back to "ask" (never silently replaces or skips).
assert.equal(readNameConflictDefault(), "ask", "unset defaults to ask");
store.set("afm.nameConflictDefault", "garbage");
assert.equal(readNameConflictDefault(), "ask", "unknown value falls back to ask");
store.clear();

// Round-trip the remembered default through storage.
writeNameConflictDefault("replace");
assert.equal(readNameConflictDefault(), "replace");
writeNameConflictDefault("skip");
assert.equal(readNameConflictDefault(), "skip");
writeNameConflictDefault("ask"); // "ask" means forget
assert.equal(store.has("afm.nameConflictDefault"), false, "ask clears the stored key");

// Remembering replace and skip are mutually exclusive (last one wins, the other clears).
setNameConflictRemember("replace", true);
assert.equal(nameConflictRememberReplace.value, true);
assert.equal(nameConflictRememberSkip.value, false);
setNameConflictRemember("skip", true);
assert.equal(nameConflictRememberSkip.value, true);
assert.equal(nameConflictRememberReplace.value, false, "choosing skip un-checks replace");

// Confirming with "remember replace" persists it as the default for later pastes.
void openNameConflictDialog("a.txt");
setNameConflictRemember("replace", true);
resolveNameConflict("replace");
assert.equal(readNameConflictDefault(), "replace", "remembered replace persisted through resolve");

// A remembered default short-circuits the dialog entirely (no prompt shown to the user).
assert.equal(await resolveNameConflictChoice("b.txt"), "replace", "short-circuits to remembered replace");
writeNameConflictDefault("skip");
assert.equal(await resolveNameConflictChoice("c.txt"), "skip", "short-circuits to remembered skip");

console.log(
  "name-conflict-checks OK (ask fallback, storage round-trip, mutually-exclusive remember, confirm persists, remembered default short-circuits).",
);
