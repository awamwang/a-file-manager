// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  buildContextMenuItems,
  canOpenInNewTab,
} from "../src/components/navigator/contextMenuModel.ts";

const dir = { name: "Docs", path: "C:/Docs", kind: "dir" as const, size: 0 };
const file = { name: "a.txt", path: "C:/Docs/a.txt", kind: "file" as const, size: 12 };

assert.equal(canOpenInNewTab(dir), true);
assert.equal(canOpenInNewTab(file), false);
assert.equal(canOpenInNewTab(null), false);

const dirItems = buildContextMenuItems(dir);
const openInNewTab = dirItems.find((item) => item.id === "openInNewTab");
assert.ok(openInNewTab);
assert.equal(openInNewTab.enabled, true);
assert.equal(openInNewTab.stub, undefined);

const fileItems = buildContextMenuItems(file);
const fileOpenInNewTab = fileItems.find((item) => item.id === "openInNewTab");
assert.ok(fileOpenInNewTab);
assert.equal(fileOpenInNewTab.enabled, false);

assert.equal(buildContextMenuItems(null).length, 0);

console.log("Context menu model checks OK (AIA-54 openInNewTab).");
