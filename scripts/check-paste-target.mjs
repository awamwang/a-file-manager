// SPDX-License-Identifier: GPL-3.0-or-later
// Paste-target resolution: folder/file/mixed/none selection decides where Ctrl+V lands.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const module = readFileSync(join(root, "src/state/pasteTarget.ts"), "utf8");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const pkg = readFileSync(join(root, "package.json"), "utf8");

assert.ok(module.includes("export function resolvePasteTarget("));
assert.ok(module.includes("export function parentDirectory("));
assert.match(
  module,
  /if \(selection\.length !== 1\) return currentDir;[\s\S]*if \(only\.isDirectory\) return only\.path;/,
);

// NavigatorView keyboard paste resolves the target from the selection, not currentPath only.
assert.ok(view.includes('from "../state/pasteTarget"'));
assert.ok(view.includes("const pasteTarget = resolvePasteTarget("));
assert.ok(view.includes("await pasteClipboard(pasteTarget)"));
assert.ok(view.includes("selectedEntries(visibleEntries.value)"));

assert.ok(pkg.includes('"test:paste-target"'));

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/paste-target-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0, "paste-target-checks.ts failed");

console.log("paste-target OK (CLIP-101/102/104).");
