// SPDX-License-Identifier: GPL-3.0-or-later
// Guard: the copy/paste name-conflict gate (replace / skip / remember) stays wired and behaves.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const src = readFileSync(join(root, "src/state/nameConflict.ts"), "utf8");

// The paste path (fileClipboard.ts) depends on this decision API.
for (const symbol of [
  "export function readNameConflictDefault",
  "export function writeNameConflictDefault",
  "export function setNameConflictRemember",
  "export function resolveNameConflict",
  "export async function resolveNameConflictChoice",
  '"afm.nameConflictDefault"',
]) {
  assert.ok(src.includes(symbol), `nameConflict.ts missing: ${symbol}`);
}

const unit = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/name-conflict-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (unit.stdout) process.stdout.write(unit.stdout);
if (unit.stderr) process.stderr.write(unit.stderr);
assert.equal(unit.status, 0, "name-conflict behavior checks failed");

console.log("name-conflict checks OK (paste replace/skip/remember contract pinned).");
