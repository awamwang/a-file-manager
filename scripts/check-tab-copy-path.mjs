// SPDX-License-Identifier: GPL-3.0-or-later
// FIL-24: Navigator tab strip right-click → tab-only ops (duplicate + close variants).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
const model = readFileSync(join(root, "src/state/openState/model.ts"), "utf8");
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(shell.includes('@contextmenu="onTabContextMenu($event, tab.groupIndex)"'));
assert.ok(shell.includes("function onTabContextMenu"));
assert.ok(shell.includes("function closeTabContextMenu"));
assert.ok(shell.includes("tabContextGroupIndex"));
assert.ok(shell.includes('id="navigator-tab-context-menu"'));
assert.ok(shell.includes("tab-context-menu"));
assert.ok(shell.includes('<Teleport to="body">'));
assert.ok(shell.includes("chrome.duplicateTab"));
assert.ok(shell.includes("chrome.tabClose"));
assert.ok(shell.includes("chrome.closeOtherTabs"));
assert.ok(shell.includes("chrome.closeRightTabs"));
assert.ok(shell.includes("chrome.closeAllTabs"));
assert.ok(shell.includes("onDuplicateTabFromContext"));
assert.ok(shell.includes("onCloseTabFromContext"));
assert.ok(shell.includes("onCloseOtherTabsFromContext"));
assert.ok(shell.includes("onCloseRightTabsFromContext"));
assert.ok(shell.includes("onCloseAllTabsFromContext"));
assert.ok(shell.includes('zIndex: "1200"') || shell.includes("zIndex: '1200'"));
assert.ok(shell.includes("closeTabContextMenu()"));
assert.ok(!shell.includes("function copyTabPath"));
assert.ok(!/navigator-tab-context-menu[\s\S]*contextMenu\.copyPath/.test(shell));

assert.ok(browse.includes("closeOtherNavigatorTabs"));
assert.ok(browse.includes("closeNavigatorTabsToRight"));
assert.ok(browse.includes("closeAllNavigatorTabs"));
assert.ok(browse.includes("newNavigatorTab"));
assert.ok(model.includes("openStateAfterCloseOtherGroups"));
assert.ok(model.includes("openStateAfterCloseGroupsToRight"));
assert.ok(model.includes("openStateAfterCloseAllGroups"));

assert.equal(en.chrome.duplicateTab, "Duplicate tab");
assert.equal(zh.chrome.duplicateTab, "复制标签");
assert.equal(en.chrome.tabClose, "Close");
assert.equal(zh.chrome.tabClose, "关闭");
assert.equal(en.chrome.closeOtherTabs, "Close other tabs");
assert.equal(zh.chrome.closeOtherTabs, "关闭其他标签");
assert.equal(en.chrome.closeRightTabs, "Close tabs to the right");
assert.equal(zh.chrome.closeRightTabs, "关闭右侧标签");
assert.equal(en.chrome.closeAllTabs, "Close all tabs");
assert.equal(zh.chrome.closeAllTabs, "关闭全部标签");
// Path copy remains available outside the tab menu.
assert.equal(en.contextMenu.copyPath, "Copy path");
assert.equal(zh.contextMenu.copyPath, "复制路径");

console.log("tab context menu (FIL-24) OK.");
