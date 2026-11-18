// SPDX-License-Identifier: GPL-3.0-or-later
// Browser bookmarks: AFM is a thin client; awam-file owns Chromium Bookmarks parsing.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const browser = readFileSync(join(root, "src-tauri/src/providers/browser.rs"), "utf8");
const factory = readFileSync(join(root, "src-tauri/src/providers/factory.rs"), "utf8");
const providersMod = readFileSync(join(root, "src-tauri/src/providers/mod.rs"), "utf8");
const view = readFileSync(join(root, "src/views/ProvidersView.vue"), "utf8");
const browse = readFileSync(join(root, "src/state/provider/browse.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));

assert.ok(providersMod.includes("mod browser"), "providers mod must include browser");
assert.ok(factory.includes("BrowserProvider"), "factory must register offline browser stub");
assert.ok(factory.includes('register(Arc::new(BrowserProvider::system_default()))'));
assert.ok(
  factory.includes("离线占位") || factory.includes("offline stub") || browser.includes("薄客户端"),
  "factory/browser must document gateway ownership",
);
assert.ok(browser.includes('"browser"') && browser.includes("fn backend_id"), "single browser backend id");
assert.ok(browser.includes("awam-file"), "offline stub must mention awam-file gateway");
assert.ok(
  !browser.includes("read_bookmarks_json") && !browser.includes("discover_profiles"),
  "AFM must not parse Chromium Bookmarks JSON",
);
assert.ok(
  !/\bHistory\b/.test(browser) && !browser.includes("Visited Links"),
  "favorites only — no history file reader",
);
// 打开逻辑已抽到 state/provider/browse.ts；Providers 页只调用 openProviderEntry。
assert.ok(view.includes("openProviderEntry"), "Providers UI opens entries via shared helper");
assert.ok(browse.includes("provider_resolve"), "provider browse resolves bookmark entries");
assert.ok(browse.includes('open_url'), "provider browse handles open_url from gateway");
assert.ok(browse.includes("openUrl"), "provider browse opens bookmark URLs");
assert.ok(
  String(en.providers.gatewayOffline).includes("rclone")
    && !String(en.providers.gatewayOffline).includes("rclone / browser"),
  "en gatewayOffline: rclone builtin only, not builtin browser bookmarks",
);
assert.ok(
  String(zh.providers.gatewayOffline).includes("rclone")
    && !String(zh.providers.gatewayOffline).includes("rclone / 浏览器书签"),
  "zh gatewayOffline: rclone builtin only; bookmarks need gateway",
);
assert.ok(
  typeof pkg.scripts["test:browser-bookmarks"] === "string",
  "package.json must wire test:browser-bookmarks",
);

const cargo = spawnSync(
  "cargo",
  ["test", "-p", "a-file-manager", "--lib", "providers::browser", "--", "--nocapture"],
  { cwd: join(root, "src-tauri"), encoding: "utf8" },
);
if (cargo.stdout) process.stdout.write(cargo.stdout);
if (cargo.stderr) process.stderr.write(cargo.stderr);
assert.equal(cargo.status, 0, "browser provider rust tests failed");

console.log("Browser bookmarks (awam-file thin client) checks OK.");
