// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  entryExtension,
  entryTypeKey,
  entryTypeLabel,
} from "../src/state/entryType.ts";
import {
  DEFAULT_DETAILS_COLUMN_WIDTHS,
  detailsGridTemplate,
  resizeColumnWidth,
} from "../src/state/navigatorColumns.ts";
assert.equal(entryExtension("notes.txt"), "txt");
assert.equal(entryExtension("archive.tar.gz"), "gz");
assert.equal(entryExtension("README"), "");
assert.equal(entryTypeKey("dir", "Docs"), "dir");
assert.equal(entryTypeKey("file", "a.PNG"), "file:png");

const labels = {
  folder: "File folder",
  file: "File",
  typedFile: (ext: string) => `${ext} File`,
};
assert.equal(entryTypeLabel("dir", "Docs", labels), "File folder");
assert.equal(entryTypeLabel("file", "a.txt", labels), "TXT File");

const resized = resizeColumnWidth(DEFAULT_DETAILS_COLUMN_WIDTHS, "type", 40);
assert.equal(resized.type, DEFAULT_DETAILS_COLUMN_WIDTHS.type + 40);
assert.ok(detailsGridTemplate(resized).includes(`${resized.type}px`));
assert.equal(
  resizeColumnWidth(DEFAULT_DETAILS_COLUMN_WIDTHS, "name", -10_000).name,
  56,
);

// Mirror groupEntriesByType without importing the Vue-adjacent module graph.
const raw = [
  { name: "a.txt", path: "C:\\Temp\\afm-fixture\\a.txt", kind: "file" },
  { name: "b.txt", path: "C:\\Temp\\afm-fixture\\b.txt", kind: "file" },
  { name: "Docs", path: "C:\\Temp\\afm-fixture\\Docs", kind: "dir" },
  { name: "c.png", path: "C:\\Temp\\afm-fixture\\c.png", kind: "file" },
];
const buckets = new Map<string, typeof raw>();
for (const entry of raw) {
  const key = entryTypeKey(entry.kind, entry.name);
  const list = buckets.get(key) ?? [];
  list.push(entry);
  buckets.set(key, list);
}
assert.equal(buckets.get("dir")?.length, 1);
assert.equal(buckets.get("file:txt")?.length, 2);

const byType = ["z.txt", "a.png", "Folder"].map((name) => ({
  name,
  key: entryTypeKey(name === "Folder" ? "dir" : "file", name),
}));
byType.sort((a, b) => a.key.localeCompare(b.key));
assert.equal(byType[0]?.name, "Folder");
assert.equal(byType[1]?.name, "a.png");

console.log("view-layout checks OK (AIA-191).");
