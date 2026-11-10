// SPDX-License-Identifier: GPL-3.0-or-later
/** Provider IPC 不得同步阻塞 UI：命令须 async + spawn_blocking；rclone 须有超时/杀树。 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const mod = readFileSync(join(root, "src-tauri/src/providers/mod.rs"), "utf8");
const rclone = readFileSync(join(root, "src-tauri/src/providers/rclone.rs"), "utf8");
const win = readFileSync(join(root, "src-tauri/src/win_process.rs"), "utf8");

for (const name of [
  "provider_gateway_status",
  "provider_backends",
  "provider_list",
  "provider_stat",
  "provider_resolve",
  "provider_preload",
  "provider_cache_invalidate",
]) {
  const re = new RegExp(
    String.raw`#\[tauri::command\][\s\S]*?pub async fn ${name}[\s\S]*?spawn_blocking`,
  );
  assert.ok(re.test(mod), `${name} must be async and use spawn_blocking`);
}

assert.ok(rclone.includes("LIST_REMOTE_TIMEOUT"), "rclone remote list wall timeout");
assert.ok(rclone.includes("--contimeout"), "rclone must pass connect timeout");
assert.ok(rclone.includes("--max-depth"), "rclone lsjson must limit depth");
assert.ok(rclone.includes("RCLONE_ASK_PASSWORD"), "disable interactive password hang");
assert.ok(rclone.includes("kill_process_tree"), "timeout must kill rclone+ssh tree");
assert.ok(rclone.includes("join_bytes_with_timeout"), "pipe join must not hang forever");
assert.ok(win.includes("fn kill_process_tree"), "win_process exposes kill_process_tree");

console.log("provider non-blocking checks OK");
