// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const registry = readFileSync(join(root, "src/shortcuts/registry.ts"), "utf8");

assert.ok(view.includes("moveSelectionByArrow"));
assert.ok(view.includes("onListKeyDown"));
assert.ok(shell.includes("chrome.focusCycle"));
assert.ok(shell.includes('data-focus-region="tabstrip"'));
assert.ok(shell.includes('data-focus-region="sidenav"'));
assert.ok(shell.includes('data-focus-region="content"'));
assert.ok(shell.includes('data-focus-region="statusbar"'));
assert.ok(registry.includes('id: "chrome.focusCycle"'));
assert.ok(registry.includes('id: "chrome.focusCycleReverse"'));

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/keyboard-nav-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

console.log("Keyboard nav shell checks OK (AIA-190).");
