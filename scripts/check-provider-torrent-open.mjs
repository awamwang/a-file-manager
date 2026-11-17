// SPDX-License-Identifier: GPL-3.0-or-later
/** Prowlarr 结果双击：push_to_downloads 必须交给系统默认程序，而不是硬编码下载器。
 *  Prowlarr 的 downloadUrl / magnetUrl 都是它自己的代理端点（需要 API Key，
 *  失败时回 XML/HTML），所以网关要先解析成真 magnet 或种子字节，AFM 只认这两样。 */
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
  'typeof args.magnetUrl === "string"',
  "args.torrentB64",
  '"open_torrent_target"',
  '"open_torrent_data"',
]) {
  assert.ok(browse.includes(needle), `openProviderEntry must handle: ${needle}`);
}
assert.ok(
  browse.indexOf("if (torrentB64)") < browse.indexOf("downloadUrl, title"),
  "torrent bytes from the gateway must be preferred over the raw download URL",
);
assert.ok(
  browse.includes('magnet.startsWith("magnet:")'),
  "only a real magnet: link may take the protocol-handler path",
);

for (const needle of [
  "pub async fn open_torrent_target",
  "pub async fn open_torrent_data",
  "fn write_torrent_temp",
  "fn write_torrent_from_base64",
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
assert.ok(lib.includes("open_with::open_torrent_data"), "register open_torrent_data IPC");

console.log("provider torrent open checks OK");
