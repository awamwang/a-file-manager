// SPDX-License-Identifier: GPL-3.0-or-later
/** Prowlarr 结果双击：push_to_downloads 必须交给系统默认程序，而不是硬编码下载器。 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (relative) => readFileSync(join(root, relative), "utf8");

const browse = read("src/state/provider/browse.ts");
const openWith = read("src-tauri/src/open_with/mod.rs");
const lib = read("src-tauri/src/lib.rs");

for (const needle of [
  'resolved.action === "push_to_downloads"',
  "resolved.actionArgs?.magnetUrl",
  "resolved.actionArgs?.downloadUrl",
  '"open_torrent_target"',
]) {
  assert.ok(browse.includes(needle), `openProviderEntry must handle: ${needle}`);
}
assert.ok(
  browse.indexOf("magnetUrl") < browse.indexOf('"open_torrent_target"'),
  "magnet should be preferred over the .torrent download URL",
);

for (const needle of [
  "pub async fn open_torrent_target",
  "spawn_blocking",
  "classify_torrent_target",
  "TorrentTargetKind::Magnet",
  "TorrentTargetKind::Http",
  ".limit(MAX_TORRENT_BYTES)",
  ".torrent",
]) {
  assert.ok(openWith.includes(needle), `open_with must include: ${needle}`);
}
assert.ok(!openWith.includes("qBittorrent"), "do not hardcode a specific downloader");
assert.ok(lib.includes("open_with::open_torrent_target"), "register open_torrent_target IPC");

console.log("provider torrent open checks OK");
