// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
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

assert.ok(nav.includes("NavigatorContextMenu"));
assert.ok(nav.includes("@contextmenu"));
assert.ok(menu.includes("copyPath"));
assert.ok(menu.includes("revealItemInDir"));
assert.ok(menu.includes("newNavigatorTab"));
assert.ok(menu.includes("canOpenInNewTab"));
assert.ok(menu.includes("shell_show_context_menu"));
assert.ok(menu.includes("deleteStub") || menu.includes("contextMenu.deleteStub"));
assert.ok(menu.includes("buildContextMenuItems"));
assert.ok(model.includes('id: "openInNewTab"'));
assert.ok(model.includes('id: "systemMenu"'));
assert.ok(model.includes("enabled: isDir"));
assert.ok(
  !/openInNewTab[\s\S]{0,120}stub:\s*true/.test(model),
  "openInNewTab must not be stubbed",
);
assert.equal(en.contextMenu.copyPath, "Copy path");
assert.equal(en.contextMenu.openInNewTab, "Open in new tab");
assert.equal(en.contextMenu.systemMenu, "System menu");
assert.equal(zh.contextMenu.openInNewTab, "在新标签页打开");
assert.equal(zh.contextMenu.systemMenu, "系统右键菜单");

for (const id of [
  "open",
  "openInNewTab",
  "systemMenu",
  "reveal",
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
