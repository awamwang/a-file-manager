// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const types = readFileSync(join(root, "src-tauri/src/discovery/types.rs"), "utf8");
const service = readFileSync(join(root, "src-tauri/src/discovery/service.rs"), "utf8");
const blacklist = readFileSync(join(root, "src-tauri/src/discovery/blacklist.rs"), "utf8");
const modRs = readFileSync(join(root, "src-tauri/src/discovery/mod.rs"), "utf8");
const defaults = readFileSync(join(root, "src-tauri/src/settings/defaults.rs"), "utf8");
const view = readFileSync(join(root, "src/views/DiscoveryView.vue"), "utf8");
const settingsView = readFileSync(join(root, "src/views/SettingsView.vue"), "utf8");
const fields = readFileSync(join(root, "src/i18n/settingsFields.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));

assert.ok(types.includes("DiscoveryIndexState"));
assert.ok(types.includes("estimated_item_count"));
assert.ok(types.includes("blacklist"));
assert.ok(service.includes("set_blacklist"));
assert.ok(service.includes("estimated_item_count"));
assert.ok(blacklist.includes("path_is_blacklisted"));
assert.ok(modRs.includes("discovery_set_blacklist"));
assert.ok(defaults.includes('"core.discoveryBlacklist"'));
assert.ok(fields.includes('"core.discoveryBlacklist"'));
assert.ok(view.includes("indexState"));
assert.ok(view.includes("discovery_set_blacklist"));
assert.ok(view.includes("blacklistDraft"));
assert.ok(settingsView.includes("stringArray"));
assert.equal(en.discovery.indexState, "Index status");
assert.equal(zh.discovery.indexState, "索引状态");
assert.equal(zh.discovery.blacklistTitle, "目录黑名单");
assert.equal(en.settings.fields.core.discoveryBlacklist.title, "Discovery directory blacklist");

console.log("Discovery index/blacklist checks OK (AIA-61).");
