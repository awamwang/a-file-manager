// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-274: Navigator tab strip right-click → copy path (Teleport to body).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(shell.includes('@contextmenu="onTabContextMenu($event, tab.path)"'));
assert.ok(shell.includes("function onTabContextMenu"));
assert.ok(shell.includes("function copyTabPath"));
assert.ok(shell.includes("function closeTabContextMenu"));
assert.ok(shell.includes("tabContextPath"));
assert.ok(shell.includes('id="navigator-tab-context-menu"'));
assert.ok(shell.includes("tab-context-menu"));
assert.ok(shell.includes('<Teleport to="body">'));
assert.ok(shell.includes("contextMenu.copyPath"));
assert.ok(shell.includes("contextMenu.copiedPath"));
assert.ok(shell.includes('zIndex: "1200"') || shell.includes("zIndex: '1200'"));
assert.ok(shell.includes("closeTabContextMenu()"));
assert.equal(en.contextMenu.copyPath, "Copy path");
assert.equal(zh.contextMenu.copyPath, "复制路径");
assert.ok(zh.contextMenu.copiedPath.includes("路径"));

console.log("tab copy path (AIA-274) OK.");
