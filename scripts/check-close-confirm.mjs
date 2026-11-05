// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-86: close → tray / quit / ask, persisted in settings.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const defaults = readFileSync(join(root, "src-tauri/src/settings/defaults.rs"), "utf8");
const libRs = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const state = readFileSync(join(root, "src/state/closeBehavior.ts"), "utf8");
const fields = readFileSync(join(root, "src/i18n/settingsFields.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));

assert.ok(defaults.includes('"core.closeBehavior"'));
assert.ok(defaults.includes('"ask"'));
assert.ok(defaults.includes('"tray"'));
assert.ok(defaults.includes('"quit"'));
assert.ok(fields.includes('"core.closeBehavior"'));
assert.ok(fields.includes('"ask"') && fields.includes('"tray"') && fields.includes('"quit"'));
assert.ok(libRs.includes("fn app_quit") || libRs.includes("app_quit"));
assert.ok(state.includes("hideMainWindowToTray"));
assert.ok(state.includes("quitApplication"));
assert.ok(state.includes("persistCloseBehavior"));
assert.ok(shell.includes("onCloseRequested"));
assert.ok(shell.includes("close-confirm-dialog"));
assert.ok(shell.includes("closeConfirmRemember"));
assert.ok(shell.includes("hideMainWindowToTray") || shell.includes("applyCloseAction"));
assert.equal(en.settings.fields.core.closeBehavior.options.ask, "Ask every time");
assert.equal(zh.settings.fields.core.closeBehavior.options.ask, "每次询问");
assert.equal(zh.chrome.closeConfirmTray, "隐藏到托盘");

console.log("close-confirm checks OK (AIA-86).");
