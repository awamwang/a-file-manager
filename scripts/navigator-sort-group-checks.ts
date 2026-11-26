// SPDX-License-Identifier: GPL-3.0-or-later
/** v17（ADR 0030）排序/分组纯逻辑回归：分桶、折叠、方向、水合。 */
import assert from "node:assert/strict";
import {
  groupEntriesBy,
  nameBucketKey,
  dateBucketKey,
  FOLDER_GROUP_KEY,
  type GroupableEntry,
} from "../src/state/navigatorGrouping.ts";
import {
  defaultDirectionForGroupField,
  groupFieldLabelKey,
  NAVIGATOR_GROUP_FIELDS,
  toggleGroupCollapsed,
  collapsedGroupKeys,
  applyNavigatorViewModeFromValues,
  navigatorGroupBy,
  navigatorGroupDirection,
} from "../src/state/navigatorViewMode.ts";
import {
  applyNavigatorSortFromValues,
  clearSortedEntriesCache,
  NAVIGATOR_SORT_COLUMNS,
  setSortColumn,
  setSortDirection,
  sortColumn,
  sortColumnLabelKey,
  sortDirection,
  sortedEntries,
  toggleSort,
} from "../src/state/navigatorSort.ts";
import {
  flattenGroupedItems,
} from "../src/state/listVirtItems.ts";

const labels = {
  folder: "Folders",
  file: "Files",
  shortcut: "Shortcuts",
  typedFile: (ext: string) => `${ext} File`,
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

const NOW = Date.UTC(2026, 10, 26, 12, 0, 0); // 2026-11-26（周四）
const DAY = 86_400_000;

type Entry = GroupableEntry & { kind: "dir" | "file" };

const dir = (name: string): Entry => ({
  name,
  path: `C:\\t\\${name}`,
  kind: "dir",
});
const file = (name: string, size: number, modifiedMs: number): Entry => ({
  name,
  path: `C:\\t\\${name}`,
  kind: "file",
  size,
  modifiedMs,
});

// ---- 名称分桶：拉丁大写 / 汉字拼音 / “#”垫底 ----
assert.equal(nameBucketKey("apple"), "A");
assert.equal(nameBucketKey("Zebra"), "Z");
assert.equal(nameBucketKey("文档"), "W");
assert.equal(nameBucketKey("啊"), "A");
assert.equal(nameBucketKey("击"), "J");
assert.equal(nameBucketKey("匝"), "Z");
assert.equal(nameBucketKey("1readme"), "#");
assert.equal(nameBucketKey("_cache"), "#");

// ---- 日期分桶（周一为一周开始） ----
assert.equal(dateBucketKey(NOW - 3 * 3600_000, NOW), "today");
assert.equal(dateBucketKey(NOW - DAY, NOW), "yesterday");
assert.equal(dateBucketKey(NOW - 3 * DAY, NOW), "thisWeek");
assert.equal(dateBucketKey(NOW - 9 * DAY, NOW), "lastWeek");
assert.equal(dateBucketKey(NOW - 40 * DAY, NOW), "earlier");
assert.equal(dateBucketKey(null, NOW), "unknown");

// ---- 分组：文件夹永远单独成组置顶 ----
const mixed: Entry[] = [
  file("b.txt", 10, NOW),
  dir("Docs"),
  file("a.txt", 10, NOW),
  file("c.png", 10, NOW),
];
const byType = groupEntriesBy(mixed, "type", "asc", labels, NOW);
assert.equal(byType[0]?.key, FOLDER_GROUP_KEY);
assert.equal(byType[0]?.entries.length, 1);
assert.equal(byType.length, 3); // folder + file:txt + file:png

// ---- 名称分组：字母桶顺序 + “#”垫底（不随方向翻转） ----
const names: Entry[] = [
  file("beta", 1, NOW),
  file("1num", 1, NOW),
  file("alpha", 1, NOW),
  file("中文", 1, NOW),
];
const byNameAsc = groupEntriesBy(names, "name", "asc", labels, NOW);
assert.deepEqual(
  byNameAsc.map((g) => g.key),
  ["A", "B", "Z", "#"],
);
const byNameDesc = groupEntriesBy(names, "name", "desc", labels, NOW);
assert.deepEqual(
  byNameDesc.map((g) => g.key),
  ["Z", "B", "A", "#"],
  "desc 翻转字母桶但 # 永远垫底",
);

// ---- 大小分组：桶 rank 顺序随方向翻转 ----
const sizes: Entry[] = [
  file("big.bin", 2 * 1024 ** 3, NOW),
  file("tiny.txt", 512, NOW),
  file("mid.doc", 8 * 1024 ** 2, NOW),
];
const bySizeAsc = groupEntriesBy(sizes, "size", "asc", labels, NOW);
assert.deepEqual(
  bySizeAsc.map((g) => g.key),
  ["small", "medium", "huge"],
);
const bySizeDesc = groupEntriesBy(sizes, "size", "desc", labels, NOW);
assert.deepEqual(
  bySizeDesc.map((g) => g.key),
  ["huge", "medium", "small"],
);

// ---- 修改日期分组：unknown 桶永远垫底 ----
const dates: Entry[] = [
  file("old.log", 1, NOW - 40 * DAY),
  file("mystery.dat", 1, NOW),
  file("fresh.txt", 1, NOW),
];
const dated = dates.map((entry) =>
  entry.name === "mystery.dat" ? { ...entry, modifiedMs: undefined } : entry,
);
const byDateAsc = groupEntriesBy(dated, "modified", "asc", labels, NOW);
assert.deepEqual(
  byDateAsc.map((g) => g.key),
  ["earlier", "today", "unknown"],
);
const byDateDesc = groupEntriesBy(dated, "modified", "desc", labels, NOW);
assert.deepEqual(
  byDateDesc.map((g) => g.key),
  ["today", "earlier", "unknown"],
);

// ---- 折叠：collapsedKeys 只留组头；组头带 groupKey/collapsed ----
const collapsed = flattenGroupedItems(byType, new Set([FOLDER_GROUP_KEY]));
assert.equal(collapsed.length, 6, "折叠组只剩组头，未折叠组保留条目");
const visibleNames = collapsed
  .filter((item) => item.kind === "entry")
  .map((item) => (item.kind === "entry" ? item.entry.name : ""));
assert.ok(!visibleNames.includes("Docs"), "已折叠组的条目不渲染");
const header = collapsed[0];
assert.equal(header?.kind, "header");
if (header?.kind === "header") {
  assert.equal(header.groupKey, FOLDER_GROUP_KEY);
  assert.equal(header.collapsed, true);
  assert.equal(header.count, 1);
}

// ---- 切换字段默认方向（Win11 对齐） ----
assert.equal(defaultDirectionForGroupField("name"), "asc");
assert.equal(defaultDirectionForGroupField("type"), "asc");
assert.equal(defaultDirectionForGroupField("size"), "desc");
assert.equal(defaultDirectionForGroupField("modified"), "desc");

// ---- 会话级折叠记忆 ----
toggleGroupCollapsed("g1");
assert.ok(collapsedGroupKeys.value.has("g1"));
toggleGroupCollapsed("g1");
assert.ok(!collapsedGroupKeys.value.has("g1"));

// ---- 水合：sort/group 键从已水合 settings 应用 ----
applyNavigatorSortFromValues({
  "core.navigatorSortColumn": "size",
  "core.navigatorSortDirection": "desc",
});
assert.equal(sortColumn.value, "size");
assert.equal(sortDirection.value, "desc");
applyNavigatorViewModeFromValues({
  "core.navigatorGroupBy": "type",
  "core.navigatorGroupDirection": "desc",
});
assert.equal(navigatorGroupBy.value, "type");
assert.equal(navigatorGroupDirection.value, "desc");
// 非法值回落默认。
applyNavigatorSortFromValues({ "core.navigatorSortColumn": "bogus" });
assert.equal(sortColumn.value, "name");
applyNavigatorViewModeFromValues({ "core.navigatorGroupBy": "bogus" });
assert.equal(navigatorGroupBy.value, "none");

// ---- 菜单字段/标签 ----
assert.deepEqual([...NAVIGATOR_SORT_COLUMNS], ["name", "modified", "type", "size"]);
assert.deepEqual([...NAVIGATOR_GROUP_FIELDS], ["none", "name", "modified", "type", "size"]);
assert.equal(sortColumnLabelKey("modified"), "chrome.sortModified");
assert.equal(groupFieldLabelKey("none"), "chrome.groupNone");
assert.equal(groupFieldLabelKey("size"), "chrome.sortSize");

// ---- 显式选列保持方向；方向单选；Details 列头 toggle ----
applyNavigatorSortFromValues({
  "core.navigatorSortColumn": "name",
  "core.navigatorSortDirection": "desc",
});
setSortColumn("type");
assert.equal(sortColumn.value, "type");
assert.equal(sortDirection.value, "desc", "选列保持当前方向");
setSortDirection("asc");
assert.equal(sortDirection.value, "asc");
toggleSort("type");
assert.equal(sortDirection.value, "desc", "同列 toggle 翻转方向");
toggleSort("size");
assert.equal(sortColumn.value, "size");
assert.equal(sortDirection.value, "asc", "换列重置为升序");

// ---- 排序本体：文件夹置顶 + 列/方向 ----
clearSortedEntriesCache();
const sorted = sortedEntries([
  file("z.txt", 5, NOW),
  dir("aDir"),
  file("a.txt", 9, NOW - DAY),
]);
assert.equal(sorted[0]?.name, "aDir");
assert.equal(sorted[1]?.name, "z.txt", "size asc: 5 < 9");
assert.equal(sorted[2]?.name, "a.txt");

console.log("navigator-sort-group checks OK (v17 / ADR 0030)");
