// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  COMPACT_LIST_ROW_HEIGHT_PX,
  computeListVirtualWindow,
  computeThumbRequestWindow,
  computeVariableListVirtualWindow,
  estimateListColumns,
  LIST_VIRTUALIZE_MIN_ENTRIES,
  resolveListRowHeightPx,
  standardDetailsRowHeight,
  THUMB_REQUEST_FALLBACK_ROWS,
} from "../src/state/listVirtualWindow.ts";
import {
  buildGroupedGridLayout,
  buildGridRows,
  computeGridListVirtualWindow,
  flattenEntryItems,
  flattenGroupedItems,
  LIST_GROUP_HEADER_HEIGHT_PX,
  nextGroupedGridPath,
  virtItemHeightPx,
} from "../src/state/listVirtItems.ts";
import { groupEntriesBy } from "../src/state/navigatorGrouping.ts";
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

// FIL-50: thumb request window covers full viewport (+ overscan), not ~2 rows / BATCH_THUMB.
const thumbCols = 5;
const thumbRow = 160; // iconsLarge-ish after FIL-34 caption
const thumbViewport = 800;
const thumb = computeThumbRequestWindow({
  total: 5000,
  scrollTop: 0,
  viewportHeight: thumbViewport,
  rowHeight: thumbRow,
  columns: thumbCols,
  mode: "iconsLarge",
  overscanRows: 1,
});
const minVisibleRows = Math.ceil(thumbViewport / thumbRow);
assert.ok(
  thumb.visibleCount >= minVisibleRows * thumbCols,
  "thumb window must cover at least a full viewport of cells",
);
assert.ok(
  thumb.visibleCount > 8,
  "thumb window must exceed one BATCH_THUMB (8) when the viewport holds more",
);
assert.equal(thumb.startIndex % thumbCols, 0);

const thumbScrolled = computeThumbRequestWindow({
  total: 5000,
  scrollTop: thumbRow * 4,
  viewportHeight: thumbViewport,
  rowHeight: thumbRow,
  columns: thumbCols,
  mode: "iconsLarge",
  overscanRows: 1,
});
assert.ok(thumbScrolled.startIndex > 0);
assert.ok(thumbScrolled.visibleCount >= minVisibleRows * thumbCols);

const thumbFallback = computeThumbRequestWindow({
  total: 5000,
  scrollTop: 0,
  viewportHeight: 0,
  rowHeight: thumbRow,
  columns: thumbCols,
  mode: "iconsLarge",
});
assert.equal(thumbFallback.startIndex, 0);
assert.equal(thumbFallback.endIndex, thumbCols * THUMB_REQUEST_FALLBACK_ROWS);

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
const groupLabels = {
  folder: "Folders",
  file: "Files",
  shortcut: "Shortcuts",
  typedFile: (ext: string) => `${ext} file`,
  nameOther: "#",
  sizeEmpty: "Empty",
  sizeSmall: "Small",
  sizeMedium: "Medium",
  sizeLarge: "Large",
  sizeHuge: "Huge",
  dateToday: "Today",
  dateYesterday: "Yesterday",
  dateThisWeek: "This week",
  dateLastWeek: "Last week",
  dateEarlier: "Earlier",
  dateUnknown: "Unknown",
};
const groups = groupEntriesBy(sampleEntries, "type", "asc", groupLabels, 0);
const grouped = flattenGroupedItems(groups, new Set<string>());
assert.ok(grouped.some((item) => item.kind === "header"));
// v17：折叠组只留组头；组头携带原始 groupKey 与 collapsed 状态。
const collapsedItems = flattenGroupedItems(
  groups,
  new Set<string>(groups.map((group) => group.key)),
);
assert.equal(
  collapsedItems.length,
  groups.length,
  "collapsed groups render header only",
);
assert.ok(
  collapsedItems.every(
    (item) => item.kind === "header" && item.collapsed && item.groupKey.length > 0,
  ),
);
assert.equal(
  virtItemHeightPx(
    {
      kind: "header",
      key: "h:g",
      groupKey: "g",
      label: "x",
      count: 1,
      collapsed: false,
    },
    32,
  ),
  LIST_GROUP_HEADER_HEIGHT_PX,
);

clearSortedEntriesCache();
const once = sortedEntries(sampleEntries);
const twice = sortedEntries(sampleEntries);
assert.equal(once, twice);

// ---- 分组 + 网格：组头独占一行，组内按列流动 ----
const mkEntry = (path: string): NavigatorEntry => ({
  name: path.split("\\").pop()!,
  path,
  kind: "file",
  size: 0,
  modifiedMs: 0,
});

// 两组：A 有 3 项，B 有 2 项；列数 2 → A 占 2 行（2+1），B 占 1 行（2）。
// 用「名称」分组才能分出两组（同类型只会分出一组）。
const gridEntries: NavigatorEntry[] = [
  mkEntry("C:\\t\\a1"),
  mkEntry("C:\\t\\a2"),
  mkEntry("C:\\t\\a3"),
  mkEntry("C:\\t\\b1"),
  mkEntry("C:\\t\\b2"),
];
const gridGroups = groupEntriesBy(gridEntries, "name", "asc", groupLabels, 0);
assert.equal(gridGroups.length, 2, "a/b 两组");
const gridItems = flattenGroupedItems(gridGroups, new Set<string>());
const gridRows = buildGridRows(gridItems, {
  columns: 2,
  headerHeight: LIST_GROUP_HEADER_HEIGHT_PX,
  entryRowHeight: 32,
});
// 每个组一个组头行；组内 ceil(n / columns) 行。
const headerRows = gridRows.filter((row) => row.spansFullWidth);
assert.equal(headerRows.length, gridGroups.length, "每组一个组头行");
assert.ok(
  gridRows.every((row) => row.endItem > row.startItem),
  "每行至少覆盖一个虚拟项",
);
assert.equal(
  gridRows.reduce((sum, row) => sum + (row.spansFullWidth ? 0 : row.endItem - row.startItem), 0),
  gridItems.filter((item) => item.kind === "entry").length,
  "组内行必须覆盖全部条目（不能丢行）",
);

// 组内条目按列切分：3 项 / 2 列 → 2+1。
const aRowSizes = gridRows
  .filter((row) => !row.spansFullWidth)
  .map((row) => row.endItem - row.startItem);
assert.deepEqual(aRowSizes, [2, 1, 2], "组内按列数切行，末行可不满");

// ---- 行级窗口：可见范围对齐组边界 ----
const gridWin = computeGridListVirtualWindow({
  rows: gridRows,
  scrollTop: 0,
  viewportHeight: 40,
});
assert.equal(gridWin.active, true);
assert.equal(gridWin.startIndex, 0, "从组头开始");
assert.ok(gridWin.endIndex >= gridWin.startIndex);
// 窗口落在某一行的中间时，必须回退到该组的组头行。
const gridWinMid = computeGridListVirtualWindow({
  rows: gridRows,
  scrollTop: LIST_GROUP_HEADER_HEIGHT_PX + 32,
  viewportHeight: 10,
});
assert.equal(
  gridWinMid.startIndex,
  gridRows.findIndex((row) => row.spansFullWidth),
  "可见范围必须对齐到组边界（组头与组同批渲染）",
);
assert.ok(gridWinMid.offsetBeforePx >= 0);

// ---- 分组网格的方向键：按视觉行列，不按「序号 ÷ 列数」 ----
const layout = buildGroupedGridLayout(gridItems, gridRows);
assert.equal(layout.rows.length, gridRows.length);
// 组头行没有条目。
assert.deepEqual(layout.rows[0], [], "组头行不含条目");
// A 组占第 1、2 行（每行 2 列），B 组占第 3 行。
assert.equal(layout.cells.get("C:\\t\\a1")?.row, 1);
assert.equal(layout.cells.get("C:\\t\\a1")?.col, 0);
assert.equal(layout.cells.get("C:\\t\\a2")?.col, 1);
assert.equal(layout.cells.get("C:\\t\\a3")?.row, 2);
assert.equal(layout.cells.get("C:\\t\\a3")?.col, 0, "组内第二行的首项在第 0 列");
assert.equal(layout.cells.get("C:\\t\\b1")?.row, 4, "新组前面有自己的组头行");
assert.equal(layout.cells.get("C:\\t\\b1")?.col, 0, "新组的首项回到第 0 列");

assert.equal(nextGroupedGridPath(layout, "C:\\t\\a1", "right"), "C:\\t\\a2");
assert.equal(nextGroupedGridPath(layout, "C:\\t\\a2", "right"), null, "行末不跨行");
assert.equal(nextGroupedGridPath(layout, "C:\\t\\a2", "left"), "C:\\t\\a1");
assert.equal(nextGroupedGridPath(layout, "C:\\t\\a1", "left"), null, "行首不跨组");
assert.equal(nextGroupedGridPath(layout, "C:\\t\\a3", "down"), "C:\\t\\b1", "向下进入下一组");
assert.equal(nextGroupedGridPath(layout, "C:\\t\\a1", "down"), "C:\\t\\a3", "向下在本组换行");
assert.equal(nextGroupedGridPath(layout, "C:\\t\\b2", "down"), null, "末行下方无处可去");
assert.equal(nextGroupedGridPath(layout, "C:\\t\\b1", "up"), "C:\\t\\a3", "向上跳过组头行");
assert.equal(nextGroupedGridPath(layout, "C:\\t\\a1", "up"), null, "首行上方无处可去");

console.log("list-virtual-window checks OK");
