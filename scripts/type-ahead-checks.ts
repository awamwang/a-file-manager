// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  TYPE_AHEAD_TIMEOUT_MS,
  feedChar,
  findTypeAheadMatch,
  initialTypeAheadState,
  nameStartsWith,
  resetTypeAhead,
} from "../src/state/navigatorTypeAhead.ts";
import {
  rowStridePx,
  scrollTopForIndex,
  scrollTopForVirtIndex,
} from "../src/state/listVirtualWindow.ts";
import { clearSortedEntriesCache, sortedEntries } from "../src/state/navigatorSort.ts";
import type { NavigatorEntry } from "../src/state/navigatorBrowse.ts";

const collator = new Intl.Collator(undefined, { sensitivity: "base" });

// ---- initial / reset ----
const init = initialTypeAheadState();
assert.equal(init.buffer, "");
assert.equal(init.cycle, false);
assert.equal(init.lastMatchedIndex, -1);
assert.equal(TYPE_AHEAD_TIMEOUT_MS, 1200);
assert.deepEqual(resetTypeAhead(), init);

// ---- feedChar ----
const first = feedChar(init, "a", 1000);
assert.equal(first.buffer, "a");
assert.equal(first.cycle, false);
assert.equal(first.lastAt, 1000);

// 不同字符扩展前缀
const extended = feedChar(first, "p", 1100);
assert.equal(extended.buffer, "ap");
assert.equal(extended.cycle, false);

// 单字符重复 → cycle，buffer 不变
const cycled = feedChar(first, "A", 1200);
assert.equal(cycled.buffer, "a");
assert.equal(cycled.cycle, true);
assert.equal(cycled.lastAt, 1200);

// 多字符缓冲再敲同字符 ≠ cycle（扩展）
const multi = feedChar(extended, "p", 1300);
assert.equal(multi.buffer, "app");
assert.equal(multi.cycle, false);

// 超时重置
const timedOut = feedChar(extended, "b", 1100 + TYPE_AHEAD_TIMEOUT_MS + 1);
assert.equal(timedOut.buffer, "b");
assert.equal(timedOut.cycle, false);
assert.equal(timedOut.lastMatchedIndex, -1);

// 大写归一化
assert.equal(feedChar(init, "Q", 10).buffer, "q");

// ---- nameStartsWith：大小写 / 重音 / 码点 ----
assert.equal(nameStartsWith("apple", "a", collator), true);
assert.equal(nameStartsWith("Apple", "a", collator), true);
assert.equal(nameStartsWith("Äpfel", "a", collator), true, "重音不敏感");
assert.equal(nameStartsWith("Été.txt", "e", collator), true, "重音不敏感");
assert.equal(nameStartsWith("banana", "a", collator), false, "仅前缀，非子串");
assert.equal(nameStartsWith("ab", "abc", collator), false, "buffer 长于 name");
assert.equal(nameStartsWith("apple", "", collator), false, "空 buffer");
assert.equal(nameStartsWith("😀emoji", "😀", collator), true, "代理对码点完整");

// ---- findTypeAheadMatch ----
const names = ["apple.txt", "Apple note", "banana", "Ävocado", "app.config"];
assert.equal(
  findTypeAheadMatch({ names, buffer: "a", fromIndex: -1, collator, cycle: false }),
  0,
  "非循环跳到第一个命中",
);
assert.equal(
  findTypeAheadMatch({ names, buffer: "app", fromIndex: -1, collator, cycle: false }),
  0,
  "前缀命中",
);
assert.equal(
  findTypeAheadMatch({ names, buffer: "z", fromIndex: -1, collator, cycle: false }),
  -1,
  "无命中",
);
assert.equal(
  findTypeAheadMatch({ names, buffer: "", fromIndex: -1, collator, cycle: false }),
  -1,
  "空 buffer",
);

// 循环：从 fromIndex+1 起，含 wrap
assert.equal(
  findTypeAheadMatch({ names, buffer: "a", fromIndex: 0, collator, cycle: true }),
  1,
  "循环下一个",
);
assert.equal(
  findTypeAheadMatch({ names, buffer: "a", fromIndex: 3, collator, cycle: true }),
  4,
  "循环 wrap 前仍按顺序",
);
assert.equal(
  findTypeAheadMatch({ names, buffer: "a", fromIndex: 4, collator, cycle: true }),
  0,
  "循环 wrap 回到开头",
);

// 与排序 collator 一致：type-ahead 命中顺序 == 视觉排序顺序
clearSortedEntriesCache();
const entries = [
  { path: "e1", name: "zebra", kind: "file", size: 1, modifiedMs: 0 },
  { path: "e2", name: "Äpfel", kind: "file", size: 1, modifiedMs: 0 },
  { path: "e3", name: "apple", kind: "file", size: 1, modifiedMs: 0 },
] as NavigatorEntry[];
const sorted = sortedEntries(entries).map((e) => e.name);
const matchIdx = findTypeAheadMatch({
  names: sorted,
  buffer: "a",
  fromIndex: -1,
  collator,
  cycle: false,
});
assert.equal(sorted[matchIdx], "Äpfel", "命中项与排序后首个 A 打头一致");

// ---- geometry ----
assert.equal(rowStridePx(28, 1, "details"), 28);
assert.equal(rowStridePx(28, 2, "details"), 36, "多列计 gap");
assert.equal(rowStridePx(80, 3, "iconsMedium"), 80, "图标无行间 gap");
assert.equal(scrollTopForIndex({ index: 0, columns: 1, mode: "details", rowHeight: 28 }), 0);
assert.equal(scrollTopForIndex({ index: 5, columns: 1, mode: "details", rowHeight: 28 }), 140);
assert.equal(scrollTopForIndex({ index: 5, columns: 3, mode: "iconsMedium", rowHeight: 80 }), 80, "图标无行间 gap，row 1");
assert.equal(scrollTopForVirtIndex([28, 28, 28, 28], 3), 84);

console.log("type-ahead checks OK");
