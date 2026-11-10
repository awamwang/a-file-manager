// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  COMPACT_LIST_ROW_HEIGHT_PX,
  computeListVirtualWindow,
  computeVariableListVirtualWindow,
  estimateListColumns,
  LIST_VIRTUALIZE_MIN_ENTRIES,
  resolveListRowHeightPx,
  standardDetailsRowHeight,
} from "../src/state/listVirtualWindow.ts";
import {
  flattenEntryItems,
  flattenGroupedItems,
  LIST_GROUP_HEADER_HEIGHT_PX,
  virtItemHeightPx,
} from "../src/state/listVirtItems.ts";
import type { NavigatorEntry } from "../src/state/navigatorBrowse.ts";
import { clearSortedEntriesCache, sortedEntries } from "../src/state/navigatorSort.ts";

assert.equal(estimateListColumns("details", 800), 1);
assert.equal(estimateListColumns("iconsMedium", 480), 5);
assert.ok(estimateListColumns("tiles", 480) >= 2);

assert.equal(resolveListRowHeightPx("details", false), standardDetailsRowHeight());
assert.equal(resolveListRowHeightPx("details", true), COMPACT_LIST_ROW_HEIGHT_PX);
assert.equal(resolveListRowHeightPx("list", true), COMPACT_LIST_ROW_HEIGHT_PX);

const small = computeListVirtualWindow({
  total: LIST_VIRTUALIZE_MIN_ENTRIES,
  scrollTop: 0,
  viewportHeight: 320,
  rowHeight: 32,
  columns: 1,
  mode: "details",
});
assert.equal(small.active, false);
assert.equal(small.endIndex, LIST_VIRTUALIZE_MIN_ENTRIES);

const large = computeListVirtualWindow({
  total: 5000,
  scrollTop: 0,
  viewportHeight: 320,
  rowHeight: 32,
  columns: 1,
  mode: "details",
});
assert.equal(large.active, true);
assert.equal(large.startIndex, 0);
assert.ok(large.endIndex < 5000);
assert.ok(large.endIndex > 10);
assert.equal(large.offsetBeforePx, 0);
assert.ok(large.offsetAfterPx > 0);

const scrolled = computeListVirtualWindow({
  total: 5000,
  scrollTop: 32 * 100,
  viewportHeight: 320,
  rowHeight: 32,
  columns: 1,
  mode: "details",
  overscanRows: 2,
});
assert.equal(scrolled.active, true);
assert.ok(scrolled.startIndex >= 90);
assert.ok(scrolled.offsetBeforePx > 0);
assert.ok(scrolled.endIndex - scrolled.startIndex < 40);

const grid = computeListVirtualWindow({
  total: 5000,
  scrollTop: 0,
  viewportHeight: 400,
  rowHeight: 104,
  columns: 5,
  mode: "iconsMedium",
  overscanRows: 1,
});
assert.equal(grid.active, true);
assert.equal(grid.startIndex % 5, 0);
assert.ok(grid.endIndex <= 5000);

const heights = Array.from({ length: 200 }, (_, i) =>
  i % 20 === 0 ? LIST_GROUP_HEADER_HEIGHT_PX : 32,
);
const variable = computeVariableListVirtualWindow({
  heights,
  scrollTop: 0,
  viewportHeight: 320,
});
assert.equal(variable.active, true);
assert.equal(variable.startIndex, 0);
assert.ok(variable.endIndex < 200);
assert.equal(variable.offsetBeforePx, 0);
assert.ok(variable.offsetAfterPx > 0);

const variableScrolled = computeVariableListVirtualWindow({
  heights,
  scrollTop: 800,
  viewportHeight: 320,
  overscanItems: 2,
});
assert.equal(variableScrolled.active, true);
assert.ok(variableScrolled.startIndex > 0);
assert.ok(variableScrolled.offsetBeforePx > 0);

const sampleEntries: NavigatorEntry[] = Array.from({ length: 5 }, (_, i) => ({
  name: `f${i}.txt`,
  path: `C:\\t\\f${i}.txt`,
  kind: "file",
  size: i,
  modifiedMs: i,
}));
const flat = flattenEntryItems(sampleEntries);
assert.equal(flat.length, 5);
assert.equal(flat[0]?.kind, "entry");
const grouped = flattenGroupedItems(sampleEntries, {
  folder: "Folders",
  file: "Files",
  typedFile: (ext) => `${ext} file`,
});
assert.ok(grouped.some((item) => item.kind === "header"));
assert.equal(
  virtItemHeightPx({ kind: "header", key: "h", label: "x", count: 1 }, 32),
  LIST_GROUP_HEADER_HEIGHT_PX,
);

clearSortedEntriesCache();
const once = sortedEntries(sampleEntries);
const twice = sortedEntries(sampleEntries);
assert.equal(once, twice);

console.log("list-virtual-window checks OK");
