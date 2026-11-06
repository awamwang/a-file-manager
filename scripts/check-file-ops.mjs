// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const rust = readFileSync(join(root, "src-tauri/src/file_ops/mod.rs"), "utf8");
const lib = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const cargo = readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8");
const menu = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);
const model = readFileSync(
  join(root, "src/components/navigator/contextMenuModel.ts"),
  "utf8",
);
const clip = readFileSync(join(root, "src/state/fileClipboard.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(cargo.includes('trash = "5'));
assert.ok(lib.includes("file_ops::file_ops_copy"));
assert.ok(lib.includes("file_ops::file_ops_delete"));
assert.ok(lib.includes("file_ops::file_ops_create_file"));
assert.ok(rust.includes("create_empty_file"));
assert.ok(rust.includes("delete_to_recycle"));
assert.ok(rust.includes("path_can_use_recycle_bin"));
assert.ok(rust.includes("trash::delete"));
assert.ok(clip.includes("file_ops_copy"));
assert.ok(clip.includes("file_ops_move"));
assert.ok(clip.includes("file_ops_clipboard_set"));
assert.ok(clip.includes("file_ops_clipboard_get"));
assert.ok(rust.includes("mod clipboard") || rust.includes("pub(crate) mod clipboard"));
assert.ok(lib.includes("file_ops_clipboard_set"));
assert.ok(lib.includes("file_ops_clipboard_get"));
assert.ok(menu.includes("setClipboard"));
assert.ok(menu.includes("pasteClipboard"));
assert.ok(menu.includes("deleteEntries"));
assert.ok(menu.includes("deleteNoRecycle"));
assert.ok(menu.includes("confirmDeleteIfNeeded"));
assert.ok(!menu.includes("window.confirm"));
assert.ok(!menu.includes("window.prompt"));
assert.ok(menu.includes("openPromptDialog"));
assert.ok(!menu.includes("deleteStub"));
assert.ok(model.includes('id: "copy"'));
assert.ok(model.includes('id: "cut"'));
assert.ok(model.includes('id: "paste"'));
assert.ok(model.includes('id: "rename"'));
assert.ok(model.includes('id: "newFolder"'));
assert.ok(model.includes('id: "newFile"'));
assert.ok(clip.includes("file_ops_create_file"));
assert.ok(menu.includes("runNewFile") || menu.includes("createFile"));
assert.ok(!/id:\s*"delete"[\s\S]{0,80}stub:\s*true/.test(model));
assert.equal(en.contextMenu.copy, "Copy");
assert.equal(en.contextMenu.newFile, "New file");
assert.equal(en.contextMenu.deleteConfirm.includes("Recycle"), true);

const modelCheck = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/context-menu-model-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (modelCheck.stdout) process.stdout.write(modelCheck.stdout);
if (modelCheck.stderr) process.stderr.write(modelCheck.stderr);
assert.equal(modelCheck.status, 0, "context menu model checks failed");

console.log("File ops checks OK (AIA-62 recycle-first copy/cut/paste/rename/mkdir).");
