// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const registry = readFileSync(join(root, "src/shortcuts/registry.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(view.includes("toggleSort"));
assert.ok(view.includes("onListMouseDown"));
assert.ok(view.includes("marquee"));
assert.ok(view.includes("application/x-afm-paths"));
assert.ok(view.includes("transferEntries") || view.includes("file_ops_move"));
assert.ok(view.includes("selectRange"));
assert.ok(view.includes("navigator.selectAll"));
assert.ok(registry.includes('id: "navigator.selectAll"'));
assert.equal(en.navigator.colName, "Name");
assert.equal(en.navigator.colSize, "Size");

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/list-select-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

console.log("List select shell checks OK (AIA-64).");
