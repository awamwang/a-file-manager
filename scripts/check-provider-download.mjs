// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Provider 虚拟文件下载链路守卫：
 * - 网关侧契约（gatewat download / HTTP 路由）在 AFM 有代理入口；
 * - Rust provider_download 走 spawn_blocking 且要求网关在线；
 * - rclone resolve 的虚拟文件动作与网关统一为 "download"；
 * - 前端统一打开流程对 download 动作先落地再在 Navigator 定位，
 *   Providers 页与 Navigator 共用同一 openProviderEntry。
 * 注释先剥离，避免文档行满足断言。
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

function code(source) {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .map((line) => line.replace(/\/\/.*$/, ""))
    .join("\n");
}

const gateway = code(read("src-tauri/src/providers/gateway.rs"));
const mod = code(read("src-tauri/src/providers/mod.rs"));
const rclone = code(read("src-tauri/src/providers/rclone.rs"));
const lib = code(read("src-tauri/src/lib.rs"));
const browse = code(read("src/state/provider/browse.ts"));
const view = code(read("src/views/ProvidersView.vue"));

// 1. 网关代理：/api/download 的 wire 解析 + camelCase 结果。
for (const needle of [
  'format!("/api/download?uri={}"',
  "struct GatewayDownloadWire",
  "struct GatewayDownloadResult",
  "DOWNLOAD_TIMEOUT",
  "pub fn download(uri: &str)",
]) {
  assert.ok(gateway.includes(needle), `gateway.rs missing ${needle}`);
}

// 2. provider_download 命令：spawn_blocking + 网关在线校验 + 注册。
for (const needle of [
  "pub async fn provider_download(uri: String)",
  "gateway::is_online()",
  "gateway::download(&uri)",
]) {
  assert.ok(mod.includes(needle), `mod.rs missing ${needle}`);
}
assert.ok(lib.includes("providers::provider_download"), "lib.rs missing command registration");

// 3. rclone 虚拟文件 resolve 动作与网关对齐。
assert.ok(
  rclone.includes('action: Some("download".to_string())'),
  'rclone.rs must resolve virtual files with action "download"',
);
assert.ok(!rclone.includes('Some("rclone_spec".to_string())'), "legacy rclone_spec action lingers");

// 4. 前端统一流程：download -> provider_download -> Navigator 定位。
for (const needle of [
  'await invoke<ProviderDownloadResult>("provider_download"',
  'if (resolved.action === "download")',
  "await downloadProviderEntry(uri)",
  "revealItemsInNavigator([localPath])",
]) {
  assert.ok(browse.includes(needle), `browse.ts missing ${needle}`);
}

// 5. Providers 页复用统一入口，不再内联 provider_resolve/open_path_default。
assert.ok(view.includes("openProviderEntry(entry.uri)"), "ProvidersView must reuse openProviderEntry");
assert.ok(
  !view.includes('invoke<ResolveResult>("provider_resolve"'),
  "ProvidersView still inlines provider_resolve",
);

console.log("provider-download checks passed");
