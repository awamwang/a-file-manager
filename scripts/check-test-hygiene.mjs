// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Test-suite hygiene meta-gate (ADR 0025 baseline maintenance).
 *
 * A gate that is defined but never wired protects nothing. This suite has twice accumulated
 * "orphan" gates that sat outside the aggregated `npm test` chain (5 found by H8), and the
 * same drift could hit check-*.mjs scripts without a `test:*` key or behavior units no .mjs
 * spawns. This gate pins the three structural invariants so the suite cannot silently rot:
 *   1. every `test:*` script key is referenced by the aggregated `test` chain;
 *   2. every `scripts/check-*.mjs` is wired through some `test:*` key;
 *   3. every `scripts/*-checks.ts` behavior unit is spawned by some `check-*.mjs`.
 * Pure reads of package.json + scripts/ — no execution, no network.
 */
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const scriptsDir = join(root, "scripts");
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const chain = pkg.scripts.test ?? "";
const files = readdirSync(scriptsDir);

// 1) no `test:*` key outside the aggregated chain
const testKeys = Object.keys(pkg.scripts).filter((k) => k.startsWith("test:") && k !== "test");
const orphanKeys = testKeys.filter((k) => !chain.includes(`npm run ${k}`));
assert.deepEqual(orphanKeys, [], `test:* keys defined but absent from the aggregated "test" chain (never runs):\n  ${orphanKeys.join("\n  ")}`);

// 2) every check-*.mjs on disk is wired by some test:* key
const wiredMjs = new Set(
  Object.entries(pkg.scripts)
    .filter(([k, v]) => k.startsWith("test:") && typeof v === "string")
    .map(([, v]) => v.match(/check-[a-z0-9-]+\.mjs/)?.[0])
    .filter(Boolean),
);
const checkMjs = files.filter((f) => /^check-.*\.mjs$/.test(f));
const orphanMjs = checkMjs.filter((f) => !wiredMjs.has(f));
assert.deepEqual(orphanMjs, [], `check-*.mjs scripts on disk with no test:* wiring (never runs):\n  ${orphanMjs.join("\n  ")}`);

// 3) every *-checks.ts behavior unit is spawned by some check-*.mjs
const mjsText = checkMjs.map((f) => readFileSync(join(scriptsDir, f), "utf8")).join("\n");
const checksTs = files.filter((f) => /-checks\.ts$/.test(f));
const orphanTs = checksTs.filter((f) => !mjsText.includes(f));
assert.deepEqual(orphanTs, [], `*-checks.ts behavior units never spawned by any check-*.mjs (dead coverage):\n  ${orphanTs.join("\n  ")}`);

console.log(
  `test-hygiene checks OK (${testKeys.length} test:* all chained; ${checkMjs.length} check-*.mjs all wired; ${checksTs.length} *-checks.ts all spawned).`,
);
