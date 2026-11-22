// SPDX-License-Identifier: GPL-3.0-or-later
// Guard: the copy/paste name-conflict gate. CF-003 / TR-014 — "Apply to all"
// is per-batch only, never persisted across pastes or sessions.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const src = readFileSync(join(root, "src/state/nameConflict.ts"), "utf8");
const clip = readFileSync(join(root, "src/state/fileClipboard.ts"), "utf8");

// Decision API: open / resolve / apply-to-all setter / batch reset / short-circuit reader.
for (const symbol of [
  "export function resetNameConflictBatch",
  "export function setNameConflictApplyToAll",
  "export function resolveNameConflict",
  "export async function resolveNameConflictChoice",
]) {
  assert.ok(src.includes(symbol), `nameConflict.ts missing: ${symbol}`);
}

// CF-003 / TR-014: apply-to-all must NEVER persist. If someone re-adds the
// old localStorage-backed "next time" default, this gate must fail. Match the
// actual API calls, not the word — the docstring legitimately mentions it.
assert.ok(!src.includes("localStorage.getItem"), "nameConflict.ts must not read localStorage");
assert.ok(!src.includes("localStorage.setItem"), "nameConflict.ts must not write localStorage");
assert.ok(!src.includes("afm.nameConflictDefault"), "old persisted-default key must stay removed");
assert.ok(
  !src.includes("readNameConflictDefault") && !src.includes("writeNameConflictDefault"),
  "old persisted-default API must stay removed",
);

// The paste path is the batch boundary. Without this reset, a remembered
// choice leaks from one paste into the next — the exact bug CF-003 forbids.
assert.ok(clip.includes("resetNameConflictBatch()"), "transferEntries must reset the batch");

const unit = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/name-conflict-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (unit.stdout) process.stdout.write(unit.stdout);
if (unit.stderr) process.stderr.write(unit.stderr);
assert.equal(unit.status, 0, "name-conflict behavior checks failed");

console.log("name-conflict checks OK (per-batch apply-to-all, no persistence, pinned).");
