// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const menu = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);
const model = readFileSync(
  join(root, "src/components/navigator/contextMenuModel.ts"),
  "utf8",
);
const nav = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const rust = readFileSync(join(root, "src-tauri/src/file_ops/new_templates.rs"), "utf8");
const lib = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(model.includes("buildNewFileMenuItem"));
assert.ok(model.includes("newFileTemplate:"));
assert.ok(menu.includes("newFileTemplate:"));
assert.ok(menu.includes("loadNewFileTemplates"));
assert.ok(nav.includes("planMultiRename"));
assert.ok(rust.includes("ShellNew"));
assert.ok(lib.includes("file_ops_new_templates"));
assert.equal(typeof en.contextMenu.renamedMulti, "string");
assert.equal(typeof en.contextMenu.newFileCustom, "string");

const logic = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/new-file-rename-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (logic.stdout) process.stdout.write(logic.stdout);
if (logic.stderr) process.stderr.write(logic.stderr);
assert.equal(logic.status, 0);

const cargo = spawnSync(
  "cargo",
  [
    "test",
    "--manifest-path",
    join(root, "src-tauri/Cargo.toml"),
    "new_templates::",
    "--",
    "--nocapture",
  ],
  { cwd: root, encoding: "utf8" },
);
if (cargo.stdout) process.stdout.write(cargo.stdout);
if (cargo.stderr) process.stderr.write(cargo.stderr);
assert.equal(cargo.status, 0);

console.log("New-file / multi-rename shell checks OK (AIA-197).");
