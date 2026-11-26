// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  buildContextMenuItems,
  buildContextToolbarItems,
  canOpenInNewTab,
  canOpenInSecondPane,
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
// FIL-74: 「在第二窗口打开」只对目录可见，顶层、紧随 Open with。
const secondPane = dirItems.find((item) => item.id === "openInSecondPane");
assert.ok(secondPane?.enabled);
assert.equal(secondPane?.labelKey, "contextMenu.openInSecondPane");
assert.equal(secondPane?.icon, "dualPane");
assert.equal(secondPane?.children, undefined);
assert.equal(canOpenInSecondPane(dir), true);
assert.equal(canOpenInSecondPane(file), false);
assert.equal(canOpenInSecondPane(null), false);
const systemMenu = dirItems.find((item) => item.id === "systemMenu");
assert.ok(systemMenu);
assert.equal(systemMenu.enabled, true);
assert.equal(systemMenu.stub, undefined);
// FIL-53: native TrackPopupMenu cascade; still no nested systemMenuShow hop (AIA-224).
assert.equal(systemMenu.shellCascade, true);
assert.equal(systemMenu.shellEmbed, undefined);
const openInTerminal = dirItems.find((item) => item.id === "openInTerminal");
assert.ok(openInTerminal);
assert.equal(openInTerminal.enabled, true);
assert.equal(openInTerminal.stub, undefined);
assert.ok(dirItems.find((item) => item.id === "openInTerminal")?.children?.length);
assert.equal(dirItems.find((item) => item.id === "systemMenu")?.children, undefined);
assert.ok(dirItems.find((item) => item.id === "properties")?.enabled);
assert.ok(dirItems.find((item) => item.id === "compressZip")?.enabled);
assert.equal(dirItems.find((item) => item.id === "extractZip")?.enabled, false);
const addFavorite = dirItems.find((item) => item.id === "toggleFavorite");
assert.ok(addFavorite);
assert.equal(addFavorite.labelKey, "contextMenu.addFavorite");
assert.equal(addFavorite.enabled, true);
// AIA-248: Add to Work Group submenu (folders only; includes Default WG for members).
const addWgEmpty = dirItems.find((item) => item.id === "addToWorkGroup");
assert.ok(addWgEmpty);
assert.equal(addWgEmpty.enabled, false);
assert.ok(addWgEmpty.children?.some((c) => c.id === "addToWorkGroup:empty"));
const addWg = buildContextMenuItems(dir, {
  workGroups: [
    { id: "default-work-group", name: "Default" },
    { id: "wg-focus", name: "Focus" },
  ],
}).find((item) => item.id === "addToWorkGroup");
assert.ok(addWg?.enabled);
assert.ok(addWg?.children?.some((c) => c.id === "addToWorkGroup:default-work-group"));
assert.ok(addWg?.children?.some((c) => c.id === "addToWorkGroup:wg-focus" && c.rawLabel === "Focus"));
assert.equal(
  buildContextMenuItems(file, {
    workGroups: [{ id: "wg-focus", name: "Focus" }],
  }).find((item) => item.id === "addToWorkGroup"),
  undefined,
  "files must not get Add to Work Group",
);
const addAsSkill = dirItems.find((item) => item.id === "addAsSkill");
assert.ok(addAsSkill);
assert.equal(addAsSkill.labelKey, "contextMenu.addAsSkill");
assert.equal(addAsSkill.enabled, true);
assert.equal(
  buildContextMenuItems(file).find((item) => item.id === "addAsSkill"),
  undefined,
  "files must not get Add as skill",
);
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
assert.equal(fileItems.find((item) => item.id === "openInSecondPane"), undefined);
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
assert.equal(empty.length, 4, "空白菜单：排序 + 分组 + 新建两项（ADR 0030）");
assert.equal(empty.find((item) => item.id === "paste"), undefined);
const sortMenu = empty.find((item) => item.id === "sortBy");
const groupMenu = empty.find((item) => item.id === "groupBy");
assert.ok(sortMenu?.children?.some((c) => c.id === "sortColumn:name" && c.checked));
assert.ok(sortMenu?.children?.some((c) => c.id === "sortDirection:desc" && !c.checked));
assert.ok(groupMenu?.children?.some((c) => c.id === "groupBy:none" && c.checked));
assert.ok(empty.find((item) => item.id === "newFolder")?.enabled);
assert.ok(empty.find((item) => item.id === "newFile")?.enabled);
assert.equal(
  buildContextToolbarItems({ emptyArea: true, canPaste: false }).find((i) => i.id === "paste")
    ?.enabled,
  false,
);

assert.equal(buildContextMenuItems(null).length, 0);

// macOS 降级：native Open With 对话框与回收站还原均为 Windows-only。
// 能力标记明确为 false 时，菜单不得提供点了必报错的入口；
// 未传 caps 时保持 Windows-first 行为（兼容旧调用方）。
const { NEUTRAL_PLATFORM } = await import("../src/config/platform.ts");
const macCaps = {
  ...NEUTRAL_PLATFORM,
  nativeOpenWithDialog: false,
  trashListRestore: false,
};
const dirWithMacCaps = buildContextMenuItems(dir, { caps: macCaps });
assert.equal(
  dirWithMacCaps.find((item) => item.id === "openWith")?.children?.some(
    (c) => c.id === "openWithNative",
  ),
  false,
  "macOS: folder Open-with submenu must not offer the Windows-only native dialog",
);
assert.ok(
  dirWithMacCaps.find((item) => item.id === "openWith")?.children?.some(
    (c) => c.id === "openInNewTab",
  ),
  "macOS: other folder Open-with verbs must remain",
);
const recycleWithMacCaps = buildContextMenuItems(file, {
  recycleMode: true,
  caps: macCaps,
});
assert.equal(
  recycleWithMacCaps.find((item) => item.id === "restore"),
  undefined,
  "macOS: recycle-bin menu must not offer always-failing restore",
);
assert.ok(
  recycleWithMacCaps.find((item) => item.id === "properties")?.enabled,
  "macOS: recycle-bin menu keeps Properties",
);
// Windows-first fallback: no caps → both verbs still present.
assert.ok(
  buildContextMenuItems(dir).find((item) => item.id === "openWith")?.children?.some(
    (c) => c.id === "openWithNative",
  ),
  "without caps the native verb stays (Windows-first default)",
);
assert.ok(
  buildContextMenuItems(file, { recycleMode: true }).find((item) => item.id === "restore"),
  "without caps the restore verb stays (Windows-first default)",
);

console.log("Context menu model checks OK (FIL-53 native shell cascade).");
