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

const dirItems = buildContextMenuItems(dir, { canPaste: true });
const openInNewTab = dirItems.find((item) => item.id === "openInNewTab");
assert.ok(openInNewTab);
assert.equal(openInNewTab.enabled, true);
assert.equal(openInNewTab.stub, undefined);
const systemMenu = dirItems.find((item) => item.id === "systemMenu");
assert.ok(systemMenu);
assert.equal(systemMenu.enabled, true);
assert.equal(systemMenu.stub, undefined);
const openInTerminal = dirItems.find((item) => item.id === "openInTerminal");
assert.ok(openInTerminal);
assert.equal(openInTerminal.enabled, true);
assert.equal(openInTerminal.stub, undefined);
assert.equal(dirItems.find((item) => item.id === "openWith")?.enabled, true);
assert.ok(dirItems.find((item) => item.id === "openWith")?.children?.length);
assert.ok(dirItems.find((item) => item.id === "openInTerminal")?.children?.length);
assert.ok(dirItems.find((item) => item.id === "systemMenu")?.children?.length);
assert.ok(dirItems.find((item) => item.id === "properties")?.enabled);
assert.ok(dirItems.find((item) => item.id === "compressZip")?.enabled);
assert.equal(dirItems.find((item) => item.id === "extractZip")?.enabled, false);
const addFavorite = dirItems.find((item) => item.id === "toggleFavorite");
assert.ok(addFavorite);
assert.equal(addFavorite.labelKey, "contextMenu.addFavorite");
assert.equal(addFavorite.enabled, true);
assert.ok(dirItems.find((item) => item.id === "copy")?.enabled);
assert.ok(dirItems.find((item) => item.id === "cut")?.enabled);
assert.equal(dirItems.find((item) => item.id === "paste")?.enabled, true);
assert.ok(dirItems.find((item) => item.id === "rename")?.enabled);
assert.ok(dirItems.find((item) => item.id === "newFolder")?.enabled);
assert.equal(dirItems.find((item) => item.id === "delete")?.stub, undefined);
assert.equal(dirItems.find((item) => item.id === "delete")?.danger, true);

const favorited = buildContextMenuItems(dir, { isFavorite: true });
assert.equal(
  favorited.find((item) => item.id === "toggleFavorite")?.labelKey,
  "contextMenu.removeFavorite",
);

const fileItems = buildContextMenuItems(file);
const fileOpenInNewTab = fileItems.find((item) => item.id === "openInNewTab");
assert.ok(fileOpenInNewTab);
assert.equal(fileOpenInNewTab.enabled, false);
assert.ok(fileItems.find((item) => item.id === "systemMenu")?.enabled);
assert.ok(fileItems.find((item) => item.id === "openInTerminal")?.enabled);
assert.ok(fileItems.find((item) => item.id === "toggleFavorite")?.enabled);
// Files share Open-with (system default / choose app / IDEs) + Reveal.
const fileOpenWith = fileItems.find((item) => item.id === "openWith");
assert.ok(fileOpenWith?.enabled);
assert.ok(fileOpenWith?.children?.some((c) => c.id === "openWithSystemDefault"));
assert.ok(fileOpenWith?.children?.some((c) => c.id === "openWithNative"));
assert.ok(
  buildContextMenuItems(file, {
    ideApps: [{ id: "cursor", name: "Cursor", available: true }],
  })
    .find((item) => item.id === "openWith")
    ?.children?.some((c) => c.id === "openWithIde:cursor"),
);
assert.ok(fileItems.find((item) => item.id === "reveal")?.enabled);
assert.equal(fileItems.find((item) => item.id === "paste")?.enabled, false);
assert.ok(fileItems.find((item) => item.id === "compressZip")?.enabled);
assert.equal(fileItems.find((item) => item.id === "extractZip")?.enabled, false);

const zip = { name: "pack.zip", path: "C:/Docs/pack.zip", kind: "file" as const, size: 100 };
const zipItems = buildContextMenuItems(zip);
assert.equal(zipItems.find((item) => item.id === "extractZip")?.enabled, true);

const withTerms = buildContextMenuItems(dir, {
  terminals: [
    { id: "powershell", name: "Windows PowerShell", available: true, supportsElevation: true },
    { id: "missing", name: "Nope", available: false },
  ],
});
const termKids = withTerms.find((item) => item.id === "openInTerminal")?.children ?? [];
assert.ok(termKids.some((c) => c.id === "terminal:powershell"));
assert.ok(termKids.some((c) => c.id === "terminal:powershell:admin"));
assert.ok(!termKids.some((c) => c.id.startsWith("terminal:missing")));

const empty = buildContextMenuItems(null, { emptyArea: true, canPaste: false });
assert.equal(empty.length, 3);
assert.equal(empty.find((item) => item.id === "paste")?.enabled, false);
assert.ok(empty.find((item) => item.id === "newFolder")?.enabled);
assert.ok(empty.find((item) => item.id === "newFile")?.enabled);

assert.equal(buildContextMenuItems(null).length, 0);

console.log("Context menu model checks OK (AIA-54..62 file ops).");
