// SPDX-License-Identifier: GPL-3.0-or-later
// Transfer-source dedup: one transfer never double-processes a folder and its descendant.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const module = readFileSync(join(root, "src/state/transferDedup.ts"), "utf8");
const clip = readFileSync(join(root, "src/state/fileClipboard.ts"), "utf8");
const pkg = readFileSync(join(root, "package.json"), "utf8");

assert.ok(module.includes("export function dedupeDescendantSources("));
assert.ok(module.includes("function isStrictDescendant("));
// Segment-boundary containment (a separator is appended), not a raw substring test.
assert.ok(module.includes("nodeKey.startsWith("));

// transferEntries runs every copy/move/paste batch through the dedup reducer.
assert.ok(clip.includes('from "./transferDedup.ts"'));
assert.match(
  clip,
  /export async function transferEntries\([\s\S]*?sources = dedupeDescendantSources\(sources\);/,
);

assert.ok(pkg.includes('"test:transfer-dedup"'));

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/transfer-dedup-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0, "transfer-dedup-checks.ts failed");

console.log("transfer-dedup OK (SEL-007/011).");
