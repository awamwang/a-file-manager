// SPDX-License-Identifier: GPL-3.0-or-later
/** ADR 0031 穿透（drill-through）纯逻辑回归：深度档位、树内判定、排序键、写入落点。 */
import assert from "node:assert/strict";
import {
  applyNavigatorDrillDepthFromValues,
  clampDrillDepth,
  DEFAULT_DRILL_DEPTH,
  DEFAULT_DRILL_LIMIT,
  drillDepth,
  drillTargetPath,
  isInsideDrillTree,
  MAX_DRILL_DEPTH,
  MAX_DRILL_LIMIT,
  nextDrillLimit,
  primaryDrill,
  resolveDrillPane,
  secondaryDrill,
} from "../src/state/navigatorDrill.ts";
import {
  clearSortedEntriesCache,
  sortedEntries,
} from "../src/state/navigatorSort.ts";
import type { NavigatorEntry } from "../src/state/navigatorBrowse.ts";

// ---- 深度档位：0..5，越界收敛，默认 2 ----
assert.equal(clampDrillDepth(1), 1);
assert.equal(clampDrillDepth(5), 5);
assert.equal(clampDrillDepth(0), 1, "0 只在 UI 上表示关闭，不落盘");
assert.equal(clampDrillDepth(9), MAX_DRILL_DEPTH);
assert.equal(clampDrillDepth(-3), 1);
assert.equal(clampDrillDepth(Number.NaN), DEFAULT_DRILL_DEPTH);
assert.equal(clampDrillDepth(2.7), 2);

// ---- 水合：档位来自 settings，缺失/非法回落默认 ----
applyNavigatorDrillDepthFromValues({ "core.navigatorDrillDepth": 4 });
assert.equal(drillDepth.value, 4);
applyNavigatorDrillDepthFromValues({});
assert.equal(drillDepth.value, DEFAULT_DRILL_DEPTH);
applyNavigatorDrillDepthFromValues({ "core.navigatorDrillDepth": "3" });
assert.equal(drillDepth.value, DEFAULT_DRILL_DEPTH, "字符串不是合法档位");

// ---- 树内判定：根、后代、同前缀兄弟目录 ----
assert.equal(isInsideDrillTree("C:\\a", "C:\\a"), true);
assert.equal(isInsideDrillTree("C:\\a\\b\\c", "C:\\a"), true);
assert.equal(isInsideDrillTree("C:\\ab", "C:\\a"), false, "前缀相同但不是子目录");
assert.equal(isInsideDrillTree("C:\\b", "C:\\a"), false);
assert.equal(isInsideDrillTree("D:\\a\\b", "C:\\a"), false);
assert.equal(isInsideDrillTree("C:\\x", "C:\\"), true, "盘符根包含一切");
assert.equal(isInsideDrillTree("", "C:\\a"), false);
assert.equal(isInsideDrillTree("C:\\a", ""), false);
// macOS / POSIX paths keep `/`; the separator must follow the path, not default to `\`.
assert.equal(isInsideDrillTree("/Users/me/dir", "/Users/me/dir"), true, "POSIX 根");
assert.equal(
  isInsideDrillTree("/Users/me/dir/sub/deep", "/Users/me/dir"),
  true,
  "POSIX 后代：树内导航不得被误判为离开穿透树",
);
assert.equal(
  isInsideDrillTree("/Users/me/directory", "/Users/me/dir"),
  false,
  "POSIX 同前缀兄弟目录",
);
assert.equal(isInsideDrillTree("/Users/me/other/sub", "/Users/me/dir"), false);
assert.equal(isInsideDrillTree("/Users/me/x", "/"), true, "POSIX 根包含一切");
assert.equal(isInsideDrillTree("/C:\\a\\b", "C:\\a"), false, "POSIX 路径不匹配 Windows 根");

// ---- 焦点窗格：双栏时副窗格，否则主窗格 ----
assert.equal(resolveDrillPane("secondary", true), "secondary");
assert.equal(resolveDrillPane("secondary", false), "primary");
assert.equal(resolveDrillPane("primary", true), "primary");

// ---- 生效深度是每窗格的：全局偏好只决定「下次开启用哪档」 ----
primaryDrill.enabled.value = true;
primaryDrill.depth.value = 2;
secondaryDrill.enabled.value = true;
secondaryDrill.depth.value = 5;
assert.equal(primaryDrill.depth.value, 2, "主窗格保留自己的生效深度");
assert.equal(secondaryDrill.depth.value, 5, "副窗格保留自己的生效深度");
assert.notEqual(
  primaryDrill.depth.value,
  secondaryDrill.depth.value,
  "两个窗格可以同时跑在不同深度",
);

// ---- 行数上限：加载更多是翻倍递增，触顶后不再增长 ----
assert.equal(nextDrillLimit(DEFAULT_DRILL_LIMIT), 4_000);
assert.equal(nextDrillLimit(4_000), 8_000);
assert.equal(nextDrillLimit(16_000), MAX_DRILL_LIMIT);
assert.equal(nextDrillLimit(MAX_DRILL_LIMIT), MAX_DRILL_LIMIT, "触顶后保持不变");
assert.equal(nextDrillLimit(0), DEFAULT_DRILL_LIMIT, "非法值回落到默认");
assert.equal(nextDrillLimit(Number.NaN), DEFAULT_DRILL_LIMIT);
assert.ok(MAX_DRILL_LIMIT > DEFAULT_DRILL_LIMIT, "上限必须大于默认值");

// ---- 写入落点：穿透开启时落在穿透根，否则是当前目录 ----
primaryDrill.enabled.value = false;
assert.equal(drillTargetPath("primary", "C:\\current"), "C:\\current");
primaryDrill.enabled.value = true;
primaryDrill.root.value = "C:\\root";
assert.equal(drillTargetPath("primary", "C:\\root\\sub"), "C:\\root");
assert.equal(drillTargetPath("secondary", "C:\\current"), "C:\\current", "副窗格不受主窗格影响");
secondaryDrill.enabled.value = false;

// ---- 排序键：穿透行按相对路径排，普通行仍按文件名排 ----
type Row = {
  name: string;
  path: string;
  kind: "dir" | "file";
  size: number;
  relativePath?: string | null;
};

const asEntries = (rows: Row[]): NavigatorEntry[] => rows as NavigatorEntry[];

const drillRows: Row[] = [
  { name: "b.txt", path: "C:\\root\\sub\\b.txt", kind: "file", size: 1, relativePath: "sub\\b.txt" },
  { name: "a.txt", path: "C:\\root\\a.txt", kind: "file", size: 1, relativePath: "a.txt" },
  { name: "a.txt", path: "C:\\root\\zz\\a.txt", kind: "file", size: 1, relativePath: "zz\\a.txt" },
];
clearSortedEntriesCache();
const sortedDrill = sortedEntries(asEntries(drillRows));
assert.deepEqual(
  sortedDrill.map((entry) => entry.relativePath),
  ["a.txt", "sub\\b.txt", "zz\\a.txt"],
  "同名文件按相对路径分层排序",
);

const plainRows: Row[] = [
  { name: "b.txt", path: "C:\\dir\\b.txt", kind: "file", size: 1 },
  { name: "a.txt", path: "C:\\dir\\a.txt", kind: "file", size: 1 },
];
clearSortedEntriesCache();
const sortedPlain = sortedEntries(asEntries(plainRows));
assert.deepEqual(
  sortedPlain.map((entry) => entry.name),
  ["a.txt", "b.txt"],
  "普通列表仍按文件名排序",
);

console.log("navigator-drill checks OK (ADR 0031).");
