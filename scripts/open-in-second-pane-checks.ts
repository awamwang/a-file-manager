// SPDX-License-Identifier: GPL-3.0-or-later
/** FIL-74: 「在第二窗口打开」纯菜单模型契约（不依赖 Tauri 运行时）。 */
import assert from "node:assert/strict";
import {
  buildContextMenuItems,
  canOpenInSecondPane,
} from "../src/components/navigator/contextMenuModel.ts";

const dir = {
  name: "Docs",
  path: "C:/Temp/afm-fixture/Docs",
  kind: "dir" as const,
  size: 0,
};
const file = {
  name: "a.txt",
  path: "C:/Temp/afm-fixture/Docs/a.txt",
  kind: "file" as const,
  size: 12,
};

assert.equal(canOpenInSecondPane(dir), true);
assert.equal(canOpenInSecondPane(file), false);
assert.equal(canOpenInSecondPane(null), false);

const dirItems = buildContextMenuItems(dir);
const second = dirItems.find((item) => item.id === "openInSecondPane");
assert.ok(second?.enabled, "dir menu must expose Open in second pane");
assert.equal(second?.labelKey, "contextMenu.openInSecondPane");
assert.equal(second?.icon, "dualPane");
assert.equal(second?.children, undefined, "single action must not own a flyout");
assert.ok(
  dirItems.findIndex((item) => item.id === "openWith") <
    dirItems.findIndex((item) => item.id === "openInSecondPane"),
  "entry sits right after Open with",
);
assert.equal(
  buildContextMenuItems(file).find((item) => item.id === "openInSecondPane"),
  undefined,
  "files must not get Open in second pane",
);
assert.equal(
  buildContextMenuItems(null, { emptyArea: true }).find(
    (item) => item.id === "openInSecondPane",
  ),
  undefined,
  "New-only empty area must not get Open in second pane",
);
assert.equal(
  buildContextMenuItems(dir, { recycleMode: true }).find(
    (item) => item.id === "openInSecondPane",
  ),
  undefined,
  "Recycle Bin restore menu must not get Open in second pane",
);

console.log("Open in second pane model checks OK (FIL-74).");
