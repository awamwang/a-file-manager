// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { planMultiRename } from "../src/state/multiRename.ts";
import { buildContextMenuItems } from "../src/components/navigator/contextMenuModel.ts";

const plan = planMultiRename(
  ["C:\\Temp\\afm-fixture\\a\\photo.jpg", "C:\\Temp\\afm-fixture\\a\\shot.png", "C:\\Temp\\afm-fixture\\a\\note"],
  "Vacation.jpg",
);
assert.deepEqual(
  plan.map((item) => item.newName),
  ["Vacation.jpg", "Vacation (2).jpg", "Vacation (3).jpg"],
);

const keepExt = planMultiRename(["D:\\Temp\\afm-fixture\\x\\a.txt", "D:\\Temp\\afm-fixture\\x\\b.md"], "Report");
assert.deepEqual(
  keepExt.map((item) => item.newName),
  ["Report.txt", "Report (2).md"],
);

const empty = buildContextMenuItems(null, {
  emptyArea: true,
  newFileTemplates: [
    { extension: "txt", label: "Text Document", defaultName: "New Text Document.txt" },
    { extension: "docx", label: "Word Document", defaultName: "New Word Document.docx" },
  ],
});
const newFile = empty.find((item) => item.id === "newFile");
assert.ok(newFile?.children?.length);
assert.equal(newFile?.children?.[0]?.id, "newFileTemplate:txt");
assert.equal(newFile?.children?.[0]?.rawLabel, "Text Document");
assert.ok(newFile?.children?.some((child) => child.id === "newFileCustom"));

console.log("new-file / multi-rename checks OK (AIA-197).");
