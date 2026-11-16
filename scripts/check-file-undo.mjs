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
assert.ok(view.includes("afterUndoMutation"), "FIL-61 undo must refresh via afterUndoMutation");
assert.ok(view.includes("afterFileMutation"), "FIL-61 mutations must use afterFileMutation");
assert.ok(registry.includes('id: "navigator.undo"'));
assert.ok(ops.includes("file_ops_restore"));
assert.equal(en.shortcuts.items.navigatorUndo, "Undo last file action");
assert.equal(en.contextMenu.undone, "Undone");

// AIA-192: Ctrl+Z with nothing undoable must stay silent.
const fileUndo = readFileSync(join(root, "src/state/fileUndo/index.ts"), "utf8");
assert.ok(
  view.includes('"Nothing to undo"'),
  "undo must silently ignore 'Nothing to undo'",
);
assert.ok(
  fileUndo.includes("Nothing to undo") && fileUndo.includes("popRecord"),
  "stale delete records (Recycle Bin items gone) must be popped silently",
);
const recycle = readFileSync(join(root, "src-tauri/src/file_ops/recycle.rs"), "utf8");
assert.ok(
  !recycle.includes("No matching items found in Recycle Bin"),
  "restore no-match must be a silent no-op, not an error",
);
assert.ok(
  recycle.includes("afm-restore-wanted-") &&
    recycle.includes("DoIt()") &&
    recycle.includes("还原"),
  "FIL-70: batch restore must DoIt() Restore/还原 in one PowerShell pass (InvokeVerb is a no-op)",
);
assert.ok(
  !recycle.includes("fn restore_recycle_item"),
  "FIL-70: per-item PowerShell restore helper must be removed (UI freeze on many files)",
);
assert.ok(
  fileUndo.includes("restoreRecyclePaths"),
  "FIL-70: undo-delete must use restoreRecyclePaths (task center + async restore)",
);
const restoreBridge = readFileSync(join(root, "src/state/recycle/bin.ts"), "utf8");
assert.ok(
  restoreBridge.includes('beginLongTask("restore"') &&
    restoreBridge.includes("file_ops_restore"),
  "FIL-70: recycle restore must enter the long-task queue",
);
assert.ok(
  /#\[tauri::command\][\s\S]*?pub async fn file_ops_restore[\s\S]*?spawn_blocking/.test(ops),
  "FIL-70: file_ops_restore must be async + spawn_blocking",
);

const mutation = readFileSync(join(root, "src/state/fileMutation.ts"), "utf8");
assert.ok(mutation.includes("afterFileMutation"));
assert.ok(mutation.includes("afterUndoMutation"));
assert.ok(mutation.includes("dirsAffectedByUndo"));
assert.ok(
  mutation.includes("reloadListingAfterMutation"),
  "mutation hook must call reloadListingAfterMutation",
);
assert.ok(
  !/setInterval|setTimeout\s*\(\s*[^,]+,\s*\d{2,}/.test(mutation),
  "FIL-61 must not poll directories as the primary refresh",
);

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
