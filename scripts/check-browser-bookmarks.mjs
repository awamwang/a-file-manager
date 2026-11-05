// SPDX-License-Identifier: GPL-3.0-or-later
// Shared Browser provider: Edge + Chrome favorites under awam://browser (no history).
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
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));

assert.ok(providersMod.includes("mod browser"), "providers mod must include browser");
assert.ok(factory.includes("BrowserProvider"), "factory must register BrowserProvider");
assert.ok(factory.includes('register(Arc::new(BrowserProvider::system_default()))'));
assert.ok(browser.includes('"browser"') && browser.includes("fn backend_id"), "single browser backend id");
assert.ok(browser.includes("Microsoft Edge") && browser.includes("Google Chrome"));
assert.ok(browser.includes("Bookmarks"), "must read Chromium Bookmarks JSON");
assert.ok(
  !/\bHistory\b/.test(browser) && !browser.includes("Visited Links"),
  "favorites only — no history file reader",
);
assert.ok(browser.includes("open_url"), "resolve must expose open_url action");
assert.ok(view.includes("provider_resolve"), "Providers UI resolves bookmark entries");
assert.ok(view.includes("openUrl"), "Providers UI opens bookmark URLs");
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

console.log("Browser bookmarks (shared category) checks OK.");
