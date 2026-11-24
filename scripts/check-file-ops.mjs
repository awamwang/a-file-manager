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
assert.ok(lib.includes("file_ops_delete_permanent"));
assert.ok(rust.includes("delete_permanent_paths"));
assert.ok(lib.includes("file_ops::file_ops_create_file"));
assert.ok(rust.includes("create_empty_file"));
// TR-013 / CF-006: type-conflict command registered + FE↔BE name parity.
assert.ok(lib.includes("file_ops::file_ops_name_conflict_info"));
assert.ok(rust.includes("pub fn file_ops_name_conflict_info"));
assert.ok(clip.includes("\"file_ops_name_conflict_info\""));
assert.ok(rust.includes("pub struct NameConflictInfo"));
// LC-005 cancel + UN-008 fingerprint commands registered (parity checked where FE calls them).
assert.ok(lib.includes("file_ops::file_ops_cancel"));
assert.ok(rust.includes("pub fn file_ops_cancel"));
assert.ok(lib.includes("file_ops::file_ops_fingerprints"));
assert.ok(rust.includes("pub fn file_ops_fingerprints"));
assert.ok(rust.includes("pub struct EntryFingerprint"));
assert.ok(rust.includes("delete_to_recycle"));
assert.ok(rust.includes("path_can_use_recycle_bin"));
assert.ok(rust.includes("trash::delete"));
// FIL-69: 删除 junction/symlink 只卸链接，不跟随目标；损坏链接仍可删。
assert.ok(rust.includes("unlink_reparse_point"));
assert.ok(rust.includes("ensure_deletable"));
assert.ok(rust.includes("delete_permanent_one"));
assert.ok(rust.includes("permanent_delete_junction_removes_link_only"));
const listRs = readFileSync(join(root, "src-tauri/src/navigator/list.rs"), "utf8");
assert.ok(
  !listRs.includes("expose the real target as `path`"),
  "listing must not rewrite junction path to the target (delete would hit the real folder)",
);
assert.ok(
  listRs.includes("path` 始终是链接/条目自身路径") ||
    listRs.includes("Never rewritten to the reparse target"),
  "listing path must stay on the link entry",
);
// 大目录删除/复制/移动不得同步阻塞 UI 线程。
for (const name of [
  "file_ops_delete",
  "file_ops_delete_permanent",
  "file_ops_copy",
  "file_ops_move",
  "file_ops_restore",
]) {
  const re = new RegExp(
    String.raw`#\[tauri::command\][\s\S]*?pub async fn ${name}[\s\S]*?spawn_blocking`,
  );
  assert.ok(re.test(rust), `${name} must be async and use spawn_blocking`);
}
const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
assert.ok(browse.includes("removePathsFromListings"));
assert.ok(browse.includes("bumpFolderTreeRevision"));
assert.ok(menu.includes("removePathsFromListings"));
assert.ok(menu.includes("bumpFolderTreeRevision"));
assert.ok(clip.includes("file_ops_copy"));
assert.ok(clip.includes("file_ops_move"));
assert.ok(clip.includes("file_ops_clipboard_set"));
assert.ok(clip.includes("file_ops_clipboard_get"));
assert.ok(rust.includes("mod clipboard") || rust.includes("pub(crate) mod clipboard"));
assert.ok(lib.includes("file_ops_clipboard_set"));
assert.ok(lib.includes("file_ops_clipboard_get"));
assert.ok(lib.includes("file_ops_name_exists"));
assert.ok(rust.includes("NameConflict::Replace"));
assert.ok(rust.includes("NameConflict::Skip"));
// 同目录粘贴：copy 仍走重名弹窗，「保留两者」生成 - Copy 副本（cut 保持无操作）。
assert.ok(clip.includes("selfPaste"), "same-dir paste dialog gate");
assert.ok(
  clip.includes('mode === "copy"'),
  "same-dir copy paste must still prompt for conflict",
);
assert.ok(
  rust.includes("unique_dest_path(parent, file_name)"),
  "same-dir paste keep-both must create a - Copy duplicate",
);
const conflictHost = readFileSync(
  join(root, "src/components/shell/NameConflictDialogHost.vue"),
  "utf8",
);
assert.ok(conflictHost.includes("conflictApplyToAll"));
assert.equal(en.dialog.conflictReplace, "Replace");
assert.equal(en.dialog.conflictKeep, "Keep both");
assert.ok(menu.includes("setClipboard"));
assert.ok(menu.includes("pasteClipboard"));
assert.ok(menu.includes("deleteEntries"));
assert.ok(menu.includes("deletePermanentConfirm"));
assert.ok(menu.includes("confirmPermanentDelete"));
assert.ok(menu.includes("pathsWithoutRecycle"));
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

// FIL-29: cut/pending-cut rows reuse clipboardMode+paths; icon/name opacity ~0.5.
const navigatorView = readFileSync(
  join(root, "src/views/NavigatorView.vue"),
  "utf8",
);
assert.ok(navigatorView.includes("removePathsFromListings"));
assert.ok(navigatorView.includes("bumpFolderTreeRevision"));
assert.ok(clip.includes("isClipboardCutPath"));
assert.ok(clip.includes('clipboardMode.value !== "cut"'));
assert.ok(navigatorView.includes("isClipboardCutPath"));
assert.ok(navigatorView.includes("'row--cut': isClipboardCutPath(entry.path)"));
// DEL-003 / DEL-008: shortcut Delete 走「不可回收 → 永久删除」分流，与右键菜单一致。
// 后端 mod.rs:192 拒绝分支是 FE 流程被绕过时的 defense-in-depth 兜底，不是主路径。
assert.ok(navigatorView.includes("pathsWithoutRecycle"));
assert.ok(navigatorView.includes("confirmPermanentDelete"));
assert.match(
  navigatorView,
  /\.row--cut\s+\.kind[\s\S]*?opacity:\s*0\.5/s,
);
assert.match(
  navigatorView,
  /\.row--cut\s+\.name-block[\s\S]*?opacity:\s*0\.5/s,
);

// DEL-004: Shift+Delete permanent delete — registry chord + global handler wired.
const registry = readFileSync(join(root, "src/shortcuts/registry.ts"), "utf8");
assert.match(
  registry,
  /id: "navigator\.deletePermanent",[\s\S]{0,120}shift: true, key: "delete"/,
  "Shift+Delete chord must stay registered as navigator.deletePermanent",
);
assert.ok(
  navigatorView.includes('registerShortcutHandler("navigator.deletePermanent"'),
  "NavigatorView must handle navigator.deletePermanent",
);

const modelCheck = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/context-menu-model-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (modelCheck.stdout) process.stdout.write(modelCheck.stdout);
if (modelCheck.stderr) process.stderr.write(modelCheck.stderr);
assert.equal(modelCheck.status, 0, "context menu model checks failed");

console.log("File ops checks OK (AIA-62 recycle-first copy/cut/paste/rename/mkdir).");
