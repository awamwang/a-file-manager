// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  buildContextMenuItems,
  buildContextToolbarItems,
  canOpenInNewTab,
} from "../src/components/navigator/contextMenuModel.ts";

const dir = { name: "Docs", path: "C:/Temp/afm-fixture/Docs", kind: "dir" as const, size: 0 };
const file = { name: "a.txt", path: "C:/Temp/afm-fixture/Docs/a.txt", kind: "file" as const, size: 12 };

assert.equal(canOpenInNewTab(dir), true);
assert.equal(canOpenInNewTab(file), false);
assert.equal(canOpenInNewTab(null), false);

const dirItems = buildContextMenuItems(dir, { canPaste: true });
// No top-level Open / Open in new tab — new tab lives under Open with.
assert.equal(dirItems.find((item) => item.id === "open"), undefined);
assert.equal(dirItems.find((item) => item.id === "openInNewTab"), undefined);
const dirOpenWith = dirItems.find((item) => item.id === "openWith");
assert.ok(dirOpenWith?.enabled);
assert.ok(dirOpenWith?.children?.some((c) => c.id === "openInNewTab" && c.enabled));
assert.ok(dirOpenWith?.children?.some((c) => c.id === "openWithNative"));
const systemMenu = dirItems.find((item) => item.id === "systemMenu");
assert.ok(systemMenu);
assert.equal(systemMenu.enabled, true);
assert.equal(systemMenu.stub, undefined);
const openInTerminal = dirItems.find((item) => item.id === "openInTerminal");
assert.ok(openInTerminal);
assert.equal(openInTerminal.enabled, true);
assert.equal(openInTerminal.stub, undefined);
assert.ok(dirItems.find((item) => item.id === "openInTerminal")?.children?.length);
assert.ok(dirItems.find((item) => item.id === "systemMenu")?.children?.length);
assert.ok(dirItems.find((item) => item.id === "properties")?.enabled);
assert.ok(dirItems.find((item) => item.id === "compressZip")?.enabled);
assert.equal(dirItems.find((item) => item.id === "extractZip")?.enabled, false);
const addFavorite = dirItems.find((item) => item.id === "toggleFavorite");
assert.ok(addFavorite);
assert.equal(addFavorite.labelKey, "contextMenu.addFavorite");
assert.equal(addFavorite.enabled, true);
// Clipboard / rename / copy-path moved to Windows-style top toolbar.
assert.equal(dirItems.find((item) => item.id === "copy"), undefined);
assert.equal(dirItems.find((item) => item.id === "cut"), undefined);
assert.equal(dirItems.find((item) => item.id === "paste"), undefined);
assert.equal(dirItems.find((item) => item.id === "rename"), undefined);
assert.equal(dirItems.find((item) => item.id === "copyPath"), undefined);
const toolbar = buildContextToolbarItems({ canPaste: true });
assert.ok(toolbar.find((item) => item.id === "copy")?.enabled);
assert.ok(toolbar.find((item) => item.id === "cut")?.enabled);
assert.equal(toolbar.find((item) => item.id === "paste")?.enabled, true);
assert.ok(toolbar.find((item) => item.id === "rename")?.enabled);
assert.ok(toolbar.find((item) => item.id === "copyPath")?.enabled);
assert.equal(toolbar.find((item) => item.id === "copy")?.icon, "copy");
assert.ok(dirItems.find((item) => item.id === "newFolder")?.enabled);
assert.equal(dirItems.find((item) => item.id === "delete")?.stub, undefined);
assert.equal(dirItems.find((item) => item.id === "delete")?.danger, true);

const favorited = buildContextMenuItems(dir, { isFavorite: true });
assert.equal(
  favorited.find((item) => item.id === "toggleFavorite")?.labelKey,
  "contextMenu.removeFavorite",
);

const fileItems = buildContextMenuItems(file);
assert.equal(fileItems.find((item) => item.id === "open"), undefined);
assert.equal(fileItems.find((item) => item.id === "openInNewTab"), undefined);
assert.ok(fileItems.find((item) => item.id === "systemMenu")?.enabled);
assert.ok(fileItems.find((item) => item.id === "openInTerminal")?.enabled);
assert.ok(fileItems.find((item) => item.id === "toggleFavorite")?.enabled);
// Files share Open-with (system default / choose app / IDEs) + Reveal; no new-tab.
const fileOpenWith = fileItems.find((item) => item.id === "openWith");
assert.ok(fileOpenWith?.enabled);
assert.ok(fileOpenWith?.children?.some((c) => c.id === "openWithSystemDefault"));
assert.ok(fileOpenWith?.children?.some((c) => c.id === "openWithNative"));
assert.ok(!fileOpenWith?.children?.some((c) => c.id === "openInNewTab"));
assert.ok(
  buildContextMenuItems(file, {
    ideApps: [{ id: "cursor", name: "Cursor", available: true, enabled: true }],
  })
    .find((item) => item.id === "openWith")
    ?.children?.some((c) => c.id === "openWithIde:cursor"),
);
assert.ok(
  !buildContextMenuItems(file, {
    ideApps: [{ id: "cursor", name: "Cursor", available: true, enabled: false }],
  })
    .find((item) => item.id === "openWith")
    ?.children?.some((c) => c.id === "openWithIde:cursor"),
  "disabled open-tool must not appear in Open with",
);
assert.equal(fileItems.find((item) => item.id === "reveal"), undefined);
assert.equal(dirItems.find((item) => item.id === "reveal"), undefined);
assert.ok(
  dirOpenWith?.children?.some((c) => c.id === "openWithNative"),
  "folder Open with keeps Explorer entry",
);
assert.equal(
  buildContextToolbarItems({ canPaste: false }).find((item) => item.id === "paste")?.enabled,
  false,
);
const emptyToolbar = buildContextToolbarItems({ emptyArea: true, canPaste: true });
assert.equal(emptyToolbar.find((item) => item.id === "copy")?.enabled, false);
assert.equal(emptyToolbar.find((item) => item.id === "paste")?.enabled, true);
assert.ok(fileItems.find((item) => item.id === "compressZip")?.enabled);
assert.equal(fileItems.find((item) => item.id === "extractZip")?.enabled, false);

const zip = { name: "pack.zip", path: "C:/Temp/afm-fixture/Docs/pack.zip", kind: "file" as const, size: 100 };
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
assert.equal(empty.length, 2);
assert.equal(empty.find((item) => item.id === "paste"), undefined);
assert.ok(empty.find((item) => item.id === "newFolder")?.enabled);
assert.ok(empty.find((item) => item.id === "newFile")?.enabled);
assert.equal(
  buildContextToolbarItems({ emptyArea: true, canPaste: false }).find((i) => i.id === "paste")
    ?.enabled,
  false,
);

assert.equal(buildContextMenuItems(null).length, 0);

console.log("Context menu model checks OK (AIA-54..62 file ops).");
