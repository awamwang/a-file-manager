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
const stackMod = readFileSync(join(root, "src/state/fileUndo/stack.ts"), "utf8");
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
  recycle.includes("New-Item -ItemType Directory -Path $origDir -Force"),
  "DEL-007: batch restore must recreate a vanished original parent chain (Explorer parity)",
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

// UN-007 (已拍板 2026-11-22): the undo stack is in-memory only and must never
// survive an app restart. If someone wires it to localStorage / sessionStorage /
// the settings store, Ctrl+Z would offer stale cross-session undos Explorer does
// not have — this gate must go red. The stack is a module-scoped Vue `ref`; the
// record/stack modules must contain no persistence calls.
for (const text of [fileUndo, stackMod]) {
  assert.ok(!text.includes("localStorage"), "undo stack must not read/write localStorage");
  assert.ok(!text.includes("sessionStorage"), "undo stack must not use sessionStorage");
}

// UN-009 (已拍板 2026-11-22): a cancel/rollback triggered by a transfer is the
// transfer's own teardown and must NOT enter the undo stack — Ctrl+Z after an
// aborted batch undoes the *previous complete* operation. Both transfer entry
// points record undo only after the `!result.ok` early return, never on a failed
// /cancelled result. Pin that ordering in the drag path and the paste path.
const dropSurface = readFileSync(join(root, "src/state/dnd/surface.ts"), "utf8");
assert.ok(
  /if \(!result\.ok\)\s*\{[\s\S]{0,160}?return[\s\S]{0,200}?recordCopyUndo/.test(dropSurface),
  "drop path must return on failure before recordCopyUndo (cancel not recorded) — UN-009",
);
const pasteBlock =
  /const result = await pasteClipboard[\s\S]*?if \(!result\.ok\)\s*\{[\s\S]*?return[\s\S]*?recordMoveUndo\(sources, result\.paths\)[\s\S]*?recordCopyUndo\(result\.paths\)/.test(
    view,
  );
assert.ok(
  pasteBlock,
  "paste path must early-return on failure before recordMoveUndo/recordCopyUndo — UN-009",
);

// UN-008 (已拍板 2026-11-22): an undo whose copy/create target was modified by a
// foreign program between the operation and Ctrl+Z must be REFUSED with a
// localized toast — never executed (would clobber the foreign edits), never
// silent. copy/create records capture fingerprints at record time and re-check
// at undo time via file_ops_fingerprints; the refused record is popped so the
// LIFO stack keeps unwinding (UN-006).
assert.ok(
  stackMod.includes("export function isContaminated"),
  "pure contamination comparison must live in stack.ts (UN-008)",
);
assert.ok(
  fileUndo.includes('"file_ops_fingerprints"'),
  "undo record/refusal must round-trip file_ops_fingerprints (UN-008)",
);
assert.ok(
  fileUndo.includes('"fileOps.undoContaminated"'),
  "the UN-008 refusal must toast the fileOps.undoContaminated i18n key",
);
assert.ok(
  /case "copy":[\s\S]*?contaminationCheck\(record, record\.createdPaths\)[\s\S]*?case "create":[\s\S]*?contaminationCheck\(record, \[record\.path\]\)/.test(
    fileUndo,
  ),
  "both destructive undos (copy/create) must run the UN-008 contamination check",
);
assert.ok(
  typeof en.fileOps.undoContaminated === "string" && en.fileOps.undoContaminated.length,
  "en.json fileOps.undoContaminated missing",
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
