// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const mainTs = readFileSync(join(root, "src/main.ts"), "utf8");
const nav = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const menu = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);
const model = readFileSync(
  join(root, "src/components/navigator/contextMenuModel.ts"),
  "utf8",
);
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));

// AIA-130: global native WebView context menu suppressed (capture-phase preventDefault).
assert.ok(
  mainTs.includes('addEventListener(') &&
    mainTs.includes('"contextmenu"') &&
    mainTs.includes("preventDefault") &&
    /contextmenu[\s\S]{0,200}preventDefault[\s\S]{0,80},\s*true\s*,/.test(mainTs),
  "main.ts must suppress native contextmenu globally (AIA-130)",
);
assert.ok(nav.includes("NavigatorContextMenu"));
assert.ok(nav.includes("@contextmenu"));
assert.ok(menu.includes("copyPath"));
assert.ok(menu.includes("revealItemInDir"));
assert.ok(menu.includes("newNavigatorTab"));
assert.ok(menu.includes("canOpenInNewTab"));
assert.ok(menu.includes("shell_show_context_menu"));
assert.ok(menu.includes("open_in_terminal"));
assert.ok(menu.includes("terminal_list_apps"));
assert.ok(menu.includes("open_native_open_with_dialog"));
assert.ok(
  menu.includes("open_path_default") ||
    readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8").includes(
      "open_path_default",
    ),
);
assert.ok(menu.includes("z-index: 1200") || menu.includes("z-index:1200"));
assert.ok((menu.match(/Teleport to="body"/g) || []).length >= 2);
assert.ok(menu.includes("onItemEnter"), "submenu must open on hover");
assert.ok(menu.includes("onSubmenuEnter"), "flyout must keep open while hovered");
assert.ok(menu.includes("scheduleSubmenuClose"), "flyout needs leave delay");
assert.ok(menu.includes("preloadLaunchersIfNeeded"), "launcher submenu should preload");
assert.ok(menu.includes("toggleFavorite"));
assert.ok(menu.includes("setClipboard"));
assert.ok(menu.includes("pasteClipboard"));
assert.ok(menu.includes("deleteEntries"));
assert.ok(!menu.includes("deleteStub"));
assert.ok(menu.includes("buildContextMenuItems"));
assert.ok(model.includes('id: "openInNewTab"'));
assert.ok(model.includes('id: "openWith"'));
assert.ok(model.includes('id: "openInTerminal"'));
assert.ok(model.includes('id: "toggleFavorite"'));
assert.ok(model.includes('id: "systemMenu"'));
assert.ok(model.includes('id: "copy"'));
assert.ok(model.includes('id: "cut"'));
assert.ok(model.includes('id: "paste"'));
assert.ok(model.includes('id: "rename"'));
assert.ok(model.includes('id: "newFolder"'));
assert.ok(model.includes('id: "newFile"'));
// Open in new tab is nested under Open with (folders only); no top-level Open.
assert.ok(
  /openWithChildren[\s\S]*id:\s*"openInNewTab"/.test(model) ||
    /if \(isDir\)[\s\S]*id:\s*"openInNewTab"/.test(model),
  "openInNewTab must live under Open with for directories",
);
assert.ok(
  !/return \[\s*\{\s*id:\s*"open"/.test(model),
  "top-level Open (current tab) must be removed",
);
assert.ok(menu.includes("createFile") || menu.includes("runNewFile"));
assert.ok(nav.includes("navigator.refresh") || nav.includes("navigator.newFile"));
assert.ok(
  !/openInNewTab[\s\S]{0,120}stub:\s*true/.test(model),
  "openInNewTab must not be stubbed",
);
assert.ok(
  !/openInTerminal[\s\S]{0,120}stub:\s*true/.test(model),
  "openInTerminal must not be stubbed",
);
assert.ok(
  !/openWith[\s\S]{0,160}stub:\s*true/.test(model),
  "openWith must not be stubbed",
);
assert.ok(
  !/toggleFavorite[\s\S]{0,160}stub:\s*true/.test(model),
  "toggleFavorite must not be stubbed",
);
assert.ok(
  !/id:\s*"delete"[\s\S]{0,80}stub:\s*true/.test(model),
  "delete must not be stubbed",
);
assert.equal(en.contextMenu.copyPath, "Copy path");
assert.equal(en.contextMenu.copy, "Copy");
assert.equal(en.contextMenu.cut, "Cut");
assert.equal(en.contextMenu.paste, "Paste");
assert.equal(en.contextMenu.rename, "Rename");
assert.equal(en.contextMenu.newFolder, "New folder");
assert.equal(en.contextMenu.openInNewTab, "Open in new tab");
assert.equal(en.contextMenu.openInTerminal, "Open in terminal");
assert.equal(en.contextMenu.openWith, "Open with…");
assert.equal(en.contextMenu.addFavorite, "Add to favorites");
assert.equal(en.contextMenu.removeFavorite, "Remove from favorites");
assert.equal(en.contextMenu.systemMenu, "System menu");
assert.equal(en.favorites.label, "Favorites");
assert.equal(zh.contextMenu.openInNewTab, "在新标签页打开");
assert.equal(zh.contextMenu.openInTerminal, "在终端打开");
assert.equal(zh.contextMenu.openWith, "打开方式…");
assert.equal(zh.contextMenu.addFavorite, "添加到收藏");
assert.equal(zh.contextMenu.systemMenu, "系统右键菜单");
assert.equal(zh.contextMenu.copy, "复制");
assert.equal(zh.favorites.label, "收藏");
assert.ok(
  menu.includes("newNavigatorTab"),
  "Open in new tab handler must remain (under Open with)",
);

for (const id of [
  "openInNewTab",
  "openWith",
  "openInTerminal",
  "toggleFavorite",
  "systemMenu",
  "reveal",
  "copy",
  "cut",
  "paste",
  "rename",
  "newFolder",
  "copyPath",
  "delete",
]) {
  assert.ok(
    model.includes(`"${id}"`) || model.includes(`'${id}'`),
    `missing action ${id}`,
  );
}

const modelCheck = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/context-menu-model-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (modelCheck.stdout) process.stdout.write(modelCheck.stdout);
if (modelCheck.stderr) process.stderr.write(modelCheck.stderr);
assert.equal(modelCheck.status, 0, "context menu model checks failed");

console.log(
  "Context menu checks OK (Open in new tab is real for directories; Copy path Day-1 set).",
);
