// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  COMPACT_LIST_ROW_HEIGHT_PX,
  computeListVirtualWindow,
  estimateListColumns,
  LIST_VIRTUALIZE_MIN_ENTRIES,
  resolveListRowHeightPx,
  standardDetailsRowHeight,
} from "../src/state/listVirtualWindow.ts";

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

console.log("list-virtual-window checks OK");
