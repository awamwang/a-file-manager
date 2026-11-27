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
  THUMB_REQUEST_CANCEL_OVERSCAN_ROWS,
  THUMB_REQUEST_FALLBACK_ROWS,
  THUMB_REQUEST_OVERSCAN_ROWS,
} from "../src/state/listVirtualWindow.ts";
import {
  buildGroupedGridLayout,
  buildGridRows,
  computeGridListVirtualWindow,
  computeGridThumbRequestWindow,
  flattenEntryItems,
  flattenGroupedItems,
  LIST_GROUP_HEADER_HEIGHT_PX,
  nextGroupedGridPath,
  virtItemHeightPx,
  type ListVirtGridRow,
} from "../src/state/listVirtItems.ts";
import { groupEntriesBy } from "../src/state/navigatorGrouping.ts";
import type { NavigatorEntry } from "../src/state/navigatorBrowse.ts";
import {
  clearSortedEntriesCache,
  setSortColumn,
  setSortDirection,
  sortedEntries,
} from "../src/state/navigatorSort.ts";
import type { ListVirtItem } from "../src/state/listVirtItems.ts";

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
  sizeTiny: "Tiny",
  sizeSmall: "Small",
  sizeMedium: "Medium",
  sizeLarge: "Large",
  sizeHuge: "Huge",
  sizeUnspecified: "Unspecified",
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

// ---- 分组 + 窗口化 × 排序切换：切排序不动分组，窗口与渲染序保持一致 ----
// 场景来自 2026-11-27 grilling：懒加载（虚拟窗口）适配分组后，排序切换是否安全。
// 不变量：组头序列不变、条目集合不变（只重排）、变高窗口界内且非空、
// orderedPaths 跟随分组渲染序而非扁平排序序（Phase5 Bug3 钉死）。
const sortSwitchEntries: NavigatorEntry[] = [];
const SORT_SWITCH_EXTS = ["txt", "png", "doc"];
for (let i = 0; i < 120; i += 1) {
  const name = `f${String(i).padStart(3, "0")}.${SORT_SWITCH_EXTS[i % 3]}`;
  sortSwitchEntries.push({
    name,
    path: `C:\\t\\${name}`,
    kind: "file",
    size: (i % 17) * 1024,
    modifiedMs: i,
  });
}
sortSwitchEntries.push({ name: "zDir", path: "C:\\t\\zDir", kind: "dir", size: 0, modifiedMs: 0 });
sortSwitchEntries.push({ name: "aDir", path: "C:\\t\\aDir", kind: "dir", size: 0, modifiedMs: 0 });

const entryPathsOf = (items: ListVirtItem<NavigatorEntry>[]): string[] =>
  items
    .filter((item) => item.kind === "entry")
    .map((item) => (item.kind === "entry" ? item.entry.path : ""));
const headerKeysOf = (items: ListVirtItem<NavigatorEntry>[]): string[] =>
  items
    .filter((item) => item.kind === "header")
    .map((item) => (item.kind === "header" ? item.groupKey : ""));

// 基线：名称升序 + 按类型分组，分组纵向视图的变高窗口。
setSortColumn("name");
clearSortedEntriesCache();
const groupedByName = groupEntriesBy(
  sortedEntries(sortSwitchEntries),
  "type",
  "asc",
  groupLabels,
  0,
);
const itemsNameSort = flattenGroupedItems(groupedByName, new Set<string>());
assert.ok(
  itemsNameSort.length > LIST_VIRTUALIZE_MIN_ENTRIES,
  "样本量必须超过窗口化阈值",
);
const winNameSort = computeVariableListVirtualWindow({
  heights: itemsNameSort.map((item) => virtItemHeightPx(item, 32)),
  scrollTop: 1500,
  viewportHeight: 600,
});
assert.ok(winNameSort.active, "超过阈值必须启用窗口化");
assert.ok(
  winNameSort.startIndex >= 0 &&
    winNameSort.startIndex <= winNameSort.endIndex &&
    winNameSort.endIndex <= itemsNameSort.length,
  "窗口切片必须落在界内",
);
assert.ok(
  itemsNameSort.slice(winNameSort.startIndex, winNameSort.endIndex).length > 0,
  "视口内必须挂载内容",
);

// 切排序（名称 → 大小）：组头序列与条目集合不变，仅组内重排。
setSortColumn("size");
const groupedBySize = groupEntriesBy(
  sortedEntries(sortSwitchEntries),
  "type",
  "asc",
  groupLabels,
  0,
);
const itemsSizeSort = flattenGroupedItems(groupedBySize, new Set<string>());
assert.deepEqual(
  headerKeysOf(itemsSizeSort),
  headerKeysOf(itemsNameSort),
  "切排序不动分组字段：组头序列保持不变",
);
assert.deepEqual(
  [...entryPathsOf(itemsSizeSort)].sort(),
  [...entryPathsOf(itemsNameSort)].sort(),
  "切排序只重排、不增删条目",
);
assert.notDeepEqual(
  entryPathsOf(itemsSizeSort),
  sortedEntries(sortSwitchEntries).map((entry) => entry.path),
  "orderedPaths 必须跟随分组渲染序；退回扁平排序序即 Phase5 Bug3 回归",
);
const winSizeSort = computeVariableListVirtualWindow({
  heights: itemsSizeSort.map((item) => virtItemHeightPx(item, 32)),
  scrollTop: 1500,
  viewportHeight: 600,
});
assert.ok(
  winSizeSort.startIndex >= 0 &&
    winSizeSort.startIndex <= winSizeSort.endIndex &&
    winSizeSort.endIndex <= itemsSizeSort.length,
  "切排序后窗口切片仍界内",
);
assert.ok(
  itemsSizeSort.slice(winSizeSort.startIndex, winSizeSort.endIndex).length > 0,
  "切排序后视口内必须挂载内容",
);

// 折叠组骤缩内容 + 过期 scrollTop：窗口函数仍必须切在界内
// （.vue 层负责 DOM 更新后回同步 clamp，见 watch([collapsedGroupKeys, navigatorGroupBy])）。
const collapseKey = groupedBySize[0]?.key ?? "";
assert.ok(collapseKey.length > 0, "必须存在可折叠的分组");
const itemsCollapsed = flattenGroupedItems(groupedBySize, new Set([collapseKey]));
assert.ok(itemsCollapsed.length < itemsSizeSort.length, "折叠组必须减少虚拟项");
const winStale = computeVariableListVirtualWindow({
  heights: itemsCollapsed.map((item) => virtItemHeightPx(item, 32)),
  scrollTop: Number.MAX_SAFE_INTEGER,
  viewportHeight: 600,
});
assert.ok(
  winStale.startIndex >= 0 &&
    winStale.startIndex <= winStale.endIndex &&
    winStale.endIndex <= itemsCollapsed.length,
  "内容骤缩 + 过期 scrollTop：窗口不得越界",
);
assert.ok(
  itemsCollapsed.slice(winStale.startIndex, winStale.endIndex).length > 0,
  "过期 scrollTop 时窗口仍落在内容上",
);

// ---- 分组网格窗口必须覆盖视口（2026-11-27 真机 bug：照片目录按名称分组首屏全空）----
// 症状：组头后是一个大组（几百个 IMG_*.jpg），视口底边落在大组内部时
// 窗口被截空——首屏完全没有数据；向下滚动、视口底边跨进下一组后，
// 才显示上一个组的内容。不变量：挂载的行区间必须覆盖整个视口。
const photoEntries: NavigatorEntry[] = [];
for (let i = 0; i < 280; i += 1) {
  photoEntries.push(mkEntry(`C:\\hainan\\IMG_${String(i + 1).padStart(4, "0")}.jpg`));
}
for (let i = 0; i < 40; i += 1) {
  photoEntries.push(mkEntry(`C:\\hainan\\PANO_${String(i + 1).padStart(4, "0")}.jpg`));
}
const photoGroups = groupEntriesBy(photoEntries, "name", "asc", groupLabels, 0);
assert.ok(photoGroups.length >= 2, "IMG/PANO 按名称至少分出两桶");
const photoItems = flattenGroupedItems(photoGroups, new Set<string>());
const photoRows = buildGridRows(photoItems, {
  columns: 8,
  headerHeight: LIST_GROUP_HEADER_HEIGHT_PX,
  entryRowHeight: 100,
});
const photoPrefix: number[] = [0];
for (const row of photoRows) {
  photoPrefix.push(photoPrefix[photoPrefix.length - 1]! + row.height);
}
const photoTotalHeight = photoPrefix[photoPrefix.length - 1]!;

function assertGridWindowCoversViewport(
  rows: readonly ListVirtGridRow[],
  prefix: readonly number[],
  scrollTop: number,
  viewportHeight: number,
  label: string,
): void {
  const win = computeGridListVirtualWindow({ rows, scrollTop, viewportHeight });
  assert.ok(win.active, `${label}: 必须启用窗口化`);
  assert.ok(win.endIndex > win.startIndex, `${label}: 窗口切片不能为空`);
  const mountedTop = prefix[win.startRow] ?? 0;
  const mountedBottom = prefix[win.endRow] ?? 0;
  assert.ok(
    mountedTop < scrollTop + viewportHeight && mountedBottom > scrollTop,
    `${label}: 挂载行必须覆盖视口（mounted=[${mountedTop},${mountedBottom}] viewport=[${scrollTop},${scrollTop + viewportHeight}]）`,
  );
}

// 首屏：视口底边深陷第一个大组内部。
assertGridWindowCoversViewport(photoRows, photoPrefix, 0, 800, "首屏 scrollTop=0");
// 大组中部：视口完全在单个大组内部。
assertGridWindowCoversViewport(photoRows, photoPrefix, 1200, 800, "大组中部");
// 跨组边界：视口横跨第一个大组尾部与第二个组组头。
assertGridWindowCoversViewport(photoRows, photoPrefix, 3200, 800, "跨组边界");
// 贴底：最后一屏。
assertGridWindowCoversViewport(
  photoRows,
  photoPrefix,
  Math.max(0, photoTotalHeight - 800),
  800,
  "贴底最后一屏",
);
// 单一大组（无下一组组头）同样必须覆盖视口。
const singleGroupEntries: NavigatorEntry[] = [];
for (let i = 0; i < 200; i += 1) {
  singleGroupEntries.push(mkEntry(`C:\\one\\IMG_${String(i + 1).padStart(4, "0")}.jpg`));
}
const singleGroupRows = buildGridRows(
  flattenGroupedItems(
    groupEntriesBy(singleGroupEntries, "name", "asc", groupLabels, 0),
    new Set<string>(),
  ),
  {
    columns: 8,
    headerHeight: LIST_GROUP_HEADER_HEIGHT_PX,
    entryRowHeight: 100,
  },
);
const singleGroupPrefix: number[] = [0];
for (const row of singleGroupRows) {
  singleGroupPrefix.push(singleGroupPrefix[singleGroupPrefix.length - 1]! + row.height);
}
assertGridWindowCoversViewport(singleGroupRows, singleGroupPrefix, 0, 800, "单一大组首屏");
assertGridWindowCoversViewport(singleGroupRows, singleGroupPrefix, 900, 800, "单一大组中部");

// ---- 分组缩略图请求窗（2026-11-27 真机 bug：分组后缩略图不加载、首屏不加载）----
// 症状：缩略图请求/取消窗此前按「扁平排序序 + 无组头几何」切片
// （computeThumbRequestWindow over visibleEntries），而分组渲染按桶序 + 含组头高度：
//   1) 桶序 ≠ 扁平排序序 → 请求打到屏幕外条目，可见条目（含首屏）永不被请求；
//   2) 组头累计高度 → 深滚时扁平映射超前于可见区，请求/取消窗双双漂移：
//      轻则漏掉视口顶行，重则越过 totalRows 变空窗（请求清零 + 全量 cancel）。
// 修复：分组时请求/取消窗改用分组视觉模型（网格行窗 computeGridThumbRequestWindow /
// 变高项窗 computeVariableListVirtualWindow），不做组边界对齐——渲染窗的
// 「整组挂载」语义不进请求侧，避免大组一次性放大成整组缩略图 IPC。
// 不变量：请求窗（窄，overscan=THUMB_REQUEST_OVERSCAN_ROWS）覆盖视口相交行上的
// 全部条目；取消窗（宽，overscan=THUMB_REQUEST_CANCEL_OVERSCAN_ROWS）⊇ 请求窗。
const THUMB_VH = 800;

function visibleGridEntryPaths(
  rows: readonly ListVirtGridRow[],
  prefix: readonly number[],
  items: readonly ListVirtItem<NavigatorEntry>[],
  scrollTop: number,
  viewportHeight: number,
): Set<string> {
  const paths = new Set<string>();
  for (let r = 0; r < rows.length; r += 1) {
    const top = prefix[r] ?? 0;
    const bottom = prefix[r + 1] ?? top;
    if (bottom <= scrollTop || top >= scrollTop + viewportHeight) continue;
    const row = rows[r]!;
    for (let i = row.startItem; i < row.endItem; i += 1) {
      const item = items[i];
      if (item && item.kind === "entry") paths.add(item.entry.path);
    }
  }
  return paths;
}

function assertGridThumbWindowCoversViewport(
  rows: readonly ListVirtGridRow[],
  prefix: readonly number[],
  items: readonly ListVirtItem<NavigatorEntry>[],
  scrollTop: number,
  label: string,
): void {
  const win = computeGridThumbRequestWindow({
    rows,
    scrollTop,
    viewportHeight: THUMB_VH,
    overscanRows: THUMB_REQUEST_OVERSCAN_ROWS,
  });
  const keep = computeGridThumbRequestWindow({
    rows,
    scrollTop,
    viewportHeight: THUMB_VH,
    overscanRows: THUMB_REQUEST_CANCEL_OVERSCAN_ROWS,
  });
  assert.ok(win.endIndex > win.startIndex, `${label}: 请求窗不能为空`);
  assert.ok(
    keep.startIndex <= win.startIndex && win.endIndex <= keep.endIndex,
    `${label}: 取消窗必须覆盖请求窗`,
  );
  const visible = visibleGridEntryPaths(rows, prefix, items, scrollTop, THUMB_VH);
  assert.ok(visible.size > 0, `${label}: 场景必须存在可见条目`);
  const requested = new Set(entryPathsOf(items.slice(win.startIndex, win.endIndex)));
  for (const path of visible) {
    assert.ok(requested.has(path), `${label}: 请求窗必须覆盖可见条目 ${path}`);
  }
}

assertGridThumbWindowCoversViewport(photoRows, photoPrefix, photoItems, 0, "首屏 scrollTop=0");
assertGridThumbWindowCoversViewport(photoRows, photoPrefix, photoItems, 1200, "大组中部");
assertGridThumbWindowCoversViewport(photoRows, photoPrefix, photoItems, 3200, "跨组边界");
assertGridThumbWindowCoversViewport(
  photoRows,
  photoPrefix,
  photoItems,
  Math.max(0, photoTotalHeight - THUMB_VH),
  "贴底最后一屏",
);

// 多组深滚：12 个首字母桶 × 20 项，组头累计高度把扁平映射推出 totalRows。
const driftEntries: NavigatorEntry[] = [];
for (let bucket = 0; bucket < 12; bucket += 1) {
  for (let i = 0; i < 20; i += 1) {
    driftEntries.push(
      mkEntry(`C:\\drift\\${String.fromCharCode(65 + bucket)}${String(i).padStart(2, "0")}.jpg`),
    );
  }
}
const driftGroups = groupEntriesBy(driftEntries, "name", "asc", groupLabels, 0);
assert.equal(driftGroups.length, 12, "12 个首字母桶");
const driftItems = flattenGroupedItems(driftGroups, new Set<string>());
const driftRows = buildGridRows(driftItems, {
  columns: 8,
  headerHeight: LIST_GROUP_HEADER_HEIGHT_PX,
  entryRowHeight: 100,
});
const driftPrefix: number[] = [0];
for (const row of driftRows) driftPrefix.push(driftPrefix[driftPrefix.length - 1]! + row.height);
const driftTotalHeight = driftPrefix[driftPrefix.length - 1]!;
assertGridThumbWindowCoversViewport(driftRows, driftPrefix, driftItems, 0, "多组首屏");
assertGridThumbWindowCoversViewport(driftRows, driftPrefix, driftItems, 3700, "多组深滚");
assertGridThumbWindowCoversViewport(
  driftRows,
  driftPrefix,
  driftItems,
  driftTotalHeight - THUMB_VH,
  "多组贴底",
);
// 场景自检（钉死旧路径的故障机制，防止测试场景失效后静默通过）：
// 同一深滚位置，旧扁平几何越过 totalRows → 空窗（startIndex >= endIndex）。
{
  const flatWin = computeThumbRequestWindow({
    total: driftEntries.length,
    scrollTop: 3700,
    viewportHeight: THUMB_VH,
    rowHeight: 100,
    columns: 8,
    mode: "iconsLarge",
  });
  assert.ok(
    flatWin.endIndex <= flatWin.startIndex,
    "扁平几何必须在多组深滚处漂移成空窗（否则本场景不再复现原 bug）",
  );
}

// 桶序 ≠ 扁平排序序：降序排序 + 升序分组，首屏可见条目部分落在扁平请求窗之外。
setSortColumn("name");
setSortDirection("desc");
clearSortedEntriesCache();
const descSorted = sortedEntries(photoEntries);
const descGroups = groupEntriesBy(descSorted, "name", "asc", groupLabels, 0);
const descItems = flattenGroupedItems(descGroups, new Set<string>());
const descRows = buildGridRows(descItems, {
  columns: 8,
  headerHeight: LIST_GROUP_HEADER_HEIGHT_PX,
  entryRowHeight: 100,
});
const descPrefix: number[] = [0];
for (const row of descRows) descPrefix.push(descPrefix[descPrefix.length - 1]! + row.height);
assertGridThumbWindowCoversViewport(descRows, descPrefix, descItems, 0, "倒序分组首屏");
{
  const flatWin = computeThumbRequestWindow({
    total: descSorted.length,
    scrollTop: 0,
    viewportHeight: THUMB_VH,
    rowHeight: 100,
    columns: 8,
    mode: "iconsLarge",
  });
  const flatPaths = new Set(
    descSorted.slice(flatWin.startIndex, flatWin.endIndex).map((entry) => entry.path),
  );
  const visible = visibleGridEntryPaths(descRows, descPrefix, descItems, 0, THUMB_VH);
  const missed = [...visible].filter((path) => !flatPaths.has(path));
  assert.ok(
    missed.length > 0,
    "扁平几何在桶序≠排序序时必须错过部分可见条目（否则本场景不再复现原 bug）",
  );
}

// 纵向分组视图（list/details/content）：请求/取消窗走变高项窗，同样覆盖可见条目。
const photoHeights = photoItems.map((item) => virtItemHeightPx(item, 32));
function visibleVirtEntryPaths(
  heights: readonly number[],
  items: readonly ListVirtItem<NavigatorEntry>[],
  scrollTop: number,
  viewportHeight: number,
): Set<string> {
  const prefix: number[] = [0];
  for (const height of heights) prefix.push(prefix[prefix.length - 1]! + height);
  const paths = new Set<string>();
  for (let i = 0; i < items.length; i += 1) {
    const top = prefix[i]!;
    const bottom = prefix[i + 1]!;
    if (bottom <= scrollTop || top >= scrollTop + viewportHeight) continue;
    const item = items[i]!;
    if (item.kind === "entry") paths.add(item.entry.path);
  }
  return paths;
}
const virtPrefix: number[] = [0];
for (const height of photoHeights) virtPrefix.push(virtPrefix[virtPrefix.length - 1]! + height);
const virtTotalHeight = virtPrefix[virtPrefix.length - 1]!;
for (const scrollTop of [0, 1500, 3210, Math.max(0, virtTotalHeight - 600)]) {
  const win = computeVariableListVirtualWindow({
    heights: photoHeights,
    scrollTop,
    viewportHeight: 600,
    overscanItems: THUMB_REQUEST_OVERSCAN_ROWS,
  });
  const keep = computeVariableListVirtualWindow({
    heights: photoHeights,
    scrollTop,
    viewportHeight: 600,
    overscanItems: THUMB_REQUEST_CANCEL_OVERSCAN_ROWS,
  });
  const visible = visibleVirtEntryPaths(photoHeights, photoItems, scrollTop, 600);
  const requested = new Set(entryPathsOf(photoItems.slice(win.startIndex, win.endIndex)));
  for (const path of visible) {
    assert.ok(requested.has(path), `纵向 scrollTop=${scrollTop}: 请求窗必须覆盖可见条目 ${path}`);
  }
  assert.ok(
    keep.startIndex <= win.startIndex && win.endIndex <= keep.endIndex,
    `纵向 scrollTop=${scrollTop}: 取消窗必须覆盖请求窗`,
  );
}

// 过期 scrollTop（折叠组骤缩后）：分组请求窗必须保持界内非退化。
{
  const staleWin = computeGridThumbRequestWindow({
    rows: photoRows,
    scrollTop: photoTotalHeight * 2,
    viewportHeight: THUMB_VH,
    overscanRows: THUMB_REQUEST_OVERSCAN_ROWS,
  });
  assert.ok(
    staleWin.startIndex >= 0 &&
      staleWin.startIndex <= staleWin.endIndex &&
      staleWin.endIndex <= photoItems.length,
    "过期 scrollTop：分组请求窗不得越界",
  );
}

console.log("list-virtual-window checks OK");
