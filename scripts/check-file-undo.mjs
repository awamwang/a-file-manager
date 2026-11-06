// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const registry = readFileSync(join(root, "src/shortcuts/registry.ts"), "utf8");
const ops = readFileSync(join(root, "src-tauri/src/file_ops/mod.rs"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(view.includes("navigator.undo"));
assert.ok(view.includes("undoLast"));
assert.ok(view.includes("recordRenameUndo"));
assert.ok(registry.includes('id: "navigator.undo"'));
assert.ok(ops.includes("file_ops_restore"));
assert.equal(en.shortcuts.items.navigatorUndo, "Undo last file action");
assert.equal(en.contextMenu.undone, "Undone");

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/file-undo-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

const rust = spawnSync(
  "cargo",
  ["test", "--manifest-path", join(root, "src-tauri/Cargo.toml"), "recycle::", "--", "--nocapture"],
  { cwd: root, encoding: "utf8" },
);
if (rust.stdout) process.stdout.write(rust.stdout);
if (rust.stderr) process.stderr.write(rust.stderr);
assert.equal(rust.status, 0);

console.log("File undo shell checks OK (AIA-192).");
