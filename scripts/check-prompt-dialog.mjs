// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-186: Fluent prompt dialog replaces window.prompt for new folder/file.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const host = readFileSync(
  join(root, "src/components/shell/PromptDialogHost.vue"),
  "utf8",
);
const dialogState = readFileSync(join(root, "src/state/promptDialog.ts"), "utf8");
const nav = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const menu = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));

assert.ok(dialogState.includes("openPromptDialog"));
assert.ok(dialogState.includes("promptDialogValue"));
assert.ok(host.includes("prompt-dialog-backdrop"));
assert.ok(host.includes("afm-prompt-dialog-input"));
assert.ok(host.includes("align-items: center"));
assert.ok(host.includes("justify-content: center"));
assert.ok(shell.includes("PromptDialogHost"));
assert.ok(nav.includes("openPromptDialog"));
assert.ok(menu.includes("openPromptDialog"));
assert.ok(!nav.includes("window.prompt"));
assert.ok(!menu.includes("window.prompt"));
assert.ok(!nav.includes("window.alert"));
assert.ok(!menu.includes("window.alert"));
assert.ok(!nav.includes("window.confirm"));
assert.ok(!menu.includes("window.confirm"));
assert.equal(zh.dialog.promptTitle, "输入名称");
assert.equal(en.dialog.promptTitle, "Enter a name");
assert.equal(zh.dialog.promptLabel, "名称");
assert.equal(en.dialog.promptLabel, "Name");

console.log("prompt-dialog checks OK (AIA-186).");
