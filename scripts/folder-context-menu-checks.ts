// SPDX-License-Identifier: GPL-3.0-or-later
/** AIA-183: breadcrumb + empty-area use the same folder context menu as a dir entry. */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { dirEntryFromPath } from "../src/state/folderContextMenu.ts";
import { buildContextMenuItems } from "../src/components/navigator/contextMenuModel.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel: string) => readFileSync(join(root, rel), "utf8");

const drive = dirEntryFromPath("C:\\");
assert.equal(drive.kind, "dir");
assert.equal(drive.name, "C:");
assert.equal(drive.path, "C:\\");

const nested = dirEntryFromPath("C:\\Users\\Docs");
assert.equal(nested.kind, "dir");
assert.equal(nested.name, "Docs");
assert.equal(nested.path, "C:\\Users\\Docs");

const slashy = dirEntryFromPath("D:/Work");
assert.equal(slashy.kind, "dir");
assert.equal(slashy.name, "Work");
assert.equal(slashy.path, "D:/Work");

const folderItems = buildContextMenuItems(nested);
assert.ok(folderItems.find((item) => item.id === "openWith")?.enabled);
assert.ok(folderItems.find((item) => item.id === "openInTerminal")?.enabled);
assert.ok(folderItems.find((item) => item.id === "toggleFavorite")?.enabled);
assert.ok(folderItems.find((item) => item.id === "properties")?.enabled);
assert.ok(folderItems.find((item) => item.id === "newFolder")?.enabled);
// Background must not fall back to New-only emptyArea list.
assert.ok(folderItems.length > 2);

const nav = read("src/views/NavigatorView.vue");
assert.ok(nav.includes("dirEntryFromPath"));
assert.ok(nav.includes("folderContextMenuRequest"));
assert.ok(nav.includes("openFolderMenu"));
assert.ok(
  /onListContextMenu[\s\S]*?dirEntryFromPath\(path\)/.test(nav),
  "empty-area list menu must open current folder entry",
);
assert.ok(
  !/onListContextMenu[\s\S]*?menuEmpty\.value = true/.test(nav),
  "empty-area must not use New-only emptyArea menu",
);

const shell = read("src/components/shell/AppShell.vue");
assert.ok(shell.includes("onBreadcrumbContextMenu"));
assert.ok(shell.includes("requestFolderContextMenu"));
assert.ok(
  shell.includes('@contextmenu="onBreadcrumbContextMenu($event, crumb)"') ||
    shell.includes("@contextmenu=\"onBreadcrumbContextMenu($event, crumb)\""),
);
assert.ok(
  /onBreadcrumbContextMenu[\s\S]*?kind === "root"[\s\S]*?return/.test(shell),
  "root crumb must not open a filesystem folder menu",
);

console.log("Folder context menu checks OK (AIA-183).");
