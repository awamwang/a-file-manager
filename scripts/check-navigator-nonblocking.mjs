// SPDX-License-Identifier: GPL-3.0-or-later
/** FIL-54: 非磁盘 / 慢路径 Navigator IPC 不得同步阻塞 UI；须 async + spawn_blocking + 加载提示可取消。 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const navMod = readFileSync(join(root, "src-tauri/src/navigator/mod.rs"), "utf8");
const places = readFileSync(join(root, "src-tauri/src/navigator/places.rs"), "utf8");
const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const openWith = readFileSync(join(root, "src-tauri/src/open_with/mod.rs"), "utf8");

for (const name of [
  "navigator_list",
  "navigator_search",
  "navigator_stat",
  "navigator_get_attributes",
  "navigator_set_attributes",
  "navigator_list_drives",
  "navigator_user_folders",
  "navigator_shell_places",
  "navigator_disk_space",
]) {
  const re = new RegExp(
    String.raw`#\[tauri::command\][\s\S]*?pub async fn ${name}[\s\S]*?spawn_blocking`,
  );
  assert.ok(re.test(navMod), `${name} must be async and use spawn_blocking`);
}

assert.ok(places.includes("GetLogicalDrives"), "list_drives uses GetLogicalDrives bitmask");
assert.ok(
  places.includes("DriveKind::Remote") && places.includes("volume_label_for_root"),
  "remote drives skip volume-label hang path",
);

assert.ok(browse.includes("listLoading.value = true"), "navigate sets listLoading");
assert.ok(browse.includes("browseEpoch"), "stale list results superseded on navigate away");
assert.ok(
  browse.includes("openRecycleBinInSystem"),
  "FIL-52 recycle fast path must remain (no AFM enumerate)",
);
assert.ok(openWith.includes("open_recycle_bin_system"), "recycle system open command present");

assert.ok(
  shell.includes("breadcrumbs--loading"),
  "path bar shows non-blocking loading hint while listLoading",
);
assert.ok(/aria-busy=/.test(shell) && shell.includes("listLoading"), "path bar aria-busy while loading");

console.log("navigator non-blocking checks OK (FIL-54)");
