// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const defaults = readFileSync(join(root, "src-tauri/src/settings/defaults.rs"), "utf8");
const libRs = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const tray = readFileSync(join(root, "src-tauri/src/system_tray.rs"), "utf8");
const settingsView = readFileSync(join(root, "src/views/SettingsView.vue"), "utf8");
const sync = readFileSync(join(root, "src/utils/autostartSync.ts"), "utf8");
const caps = readFileSync(join(root, "src-tauri/capabilities/default.json"), "utf8");
const cargo = readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8");
const fields = readFileSync(join(root, "src/i18n/settingsFields.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));

assert.ok(defaults.includes('"core.launchAtStartup"'));
assert.ok(defaults.includes('"core.launchAtStartupHidden"'));
assert.ok(fields.includes('"core.launchAtStartup"'));
assert.ok(fields.includes('"core.launchAtStartupHidden"'));
assert.ok(libRs.includes("autostart::autostart_set_enabled"));
assert.ok(libRs.includes("AFM_AUTOSTART_CLI_FLAG") || libRs.includes("autostart::AFM_AUTOSTART_CLI_FLAG"));
assert.ok(libRs.includes("launchAtStartupHidden"));
assert.ok(tray.includes("TrayIconBuilder"));
assert.ok(settingsView.includes("applyLaunchAtStartupPreference"));
assert.ok(sync.includes("autostart_set_enabled"));
assert.ok(caps.includes("core:tray:default"));
assert.ok(cargo.includes("tray-icon"));
assert.ok(!cargo.includes("tauri-plugin-autostart"));
const autostartRs = readFileSync(join(root, "src-tauri/src/autostart.rs"), "utf8");
assert.ok(autostartRs.includes("CurrentVersion\\\\Run") || autostartRs.includes(String.raw`CurrentVersion\Run`));
assert.equal(en.settings.fields.core.launchAtStartup.title, "Launch at startup");
assert.equal(zh.settings.fields.core.launchAtStartup.title, "开机自启动");
assert.equal(zh.settings.fields.core.launchAtStartupHidden.title, "启动后隐藏到托盘");

console.log("Autostart / tray checks OK (AIA-60).");
