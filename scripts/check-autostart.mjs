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
assert.ok(
  libRs.includes("StartupReveal::new(!hide_on_autostart)"),
  "autostart-hidden must gate StartupReveal (AIA-272), not only hide_main_window",
);
assert.ok(tray.includes("afm_startup_should_reveal"), "tray module must expose reveal gate");
assert.ok(tray.includes("StartupReveal"), "tray module must define StartupReveal");
assert.ok(tray.includes("TrayIconBuilder"));
assert.ok(tray.includes("include_bytes!"), "tray must embed AFM icon bytes");
assert.ok(tray.includes("icons/128x128.png"), "tray must use regenerated Tauri icon PNG");
assert.ok(tray.includes("load_app_icon"), "tray must expose load_app_icon");
assert.ok(libRs.includes("set_icon"), "main window must set_icon for taskbar");
assert.ok(tray.includes("destroy_system_tray") || tray.includes("remove_tray_by_id"));
assert.ok(
  tray.includes("try_acquire_single_instance_lock") || libRs.includes("single_instance"),
  "tray/GUI should use a single-instance lock",
);
assert.ok(libRs.includes("destroy_system_tray"), "exit path must destroy tray");
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
