// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Navigator type filter — pure logic regression.
 *
 * Covers: extension → major-category mapping, dynamic facet collection,
 * match semantics (category-level and specific-type refinement, folder
 * handling, drill-through rows), filter narrowing and the selection
 * invariant (a specific type implies and requires its category).
 */
import assert from "node:assert/strict";
import {
  clearTypeFilter,
  collectTypeFacets,
  entryMatchesTypeFilter,
  entryTypeCategory,
  filterEntriesByType,
  isTypeFilterActive,
  normalizeTypeFilterSelection,
  toggleTypeFilterCategory,
  toggleTypeFilterExtension,
  TYPE_FILTER_CATEGORY_ORDER,
  typeFilterActive,
  typeFilterCategories,
  typeFilterExtensions,
  typeCategoryForExtension,
  type TypeFacet,
} from "../src/state/navigatorTypeFilter.ts";
import type { NavigatorEntry } from "../src/state/navigatorBrowse.ts";

type Row = { name: string; kind: "dir" | "file" };
const rows = (list: Row[]): NavigatorEntry[] => list as NavigatorEntry[];

// ---- 大类映射：常见扩展名 → 类别 ----
assert.equal(typeCategoryForExtension("png"), "image");
assert.equal(typeCategoryForExtension("PNG"), "image", "大小写不敏感");
assert.equal(typeCategoryForExtension(" jpg "), "image", "首尾空白被修剪");
assert.equal(typeCategoryForExtension("jpg"), "image");
assert.equal(typeCategoryForExtension("jpeg"), "image");
assert.equal(typeCategoryForExtension("svg"), "image");
assert.equal(typeCategoryForExtension("mp4"), "video");
assert.equal(typeCategoryForExtension("mkv"), "video");
assert.equal(typeCategoryForExtension("mp3"), "audio");
assert.equal(typeCategoryForExtension("flac"), "audio");
assert.equal(typeCategoryForExtension("pdf"), "document");
assert.equal(typeCategoryForExtension("docx"), "document");
assert.equal(typeCategoryForExtension("xlsx"), "document");
assert.equal(typeCategoryForExtension("md"), "document");
assert.equal(typeCategoryForExtension("zip"), "archive");
assert.equal(typeCategoryForExtension("7z"), "archive");
assert.equal(typeCategoryForExtension("ts"), "code");
assert.equal(typeCategoryForExtension("py"), "code");
assert.equal(typeCategoryForExtension("exe"), "other", "未登记扩展名归入其他");
assert.equal(typeCategoryForExtension(""), "other");

// ---- 行类别：文件夹自成一类，文件按扩展名 ----
assert.equal(entryTypeCategory("dir", "照片"), "folder", "文件夹与名字无关");
assert.equal(entryTypeCategory("file", "photo.PNG"), "image");
assert.equal(entryTypeCategory("file", "archive.7z"), "archive");
assert.equal(entryTypeCategory("file", "README"), "other", "无扩展名归入其他");
assert.equal(entryTypeCategory("file", ".gitignore"), "other", "点开头无扩展名");

// ---- 动态清单：只列出当前行里存在的类别与具体类型 ----
const listing = rows([
  { name: "notes", kind: "dir" },
  { name: "a.png", kind: "file" },
  { name: "b.png", kind: "file" },
  { name: "c.jpg", kind: "file" },
  { name: "clip.mp4", kind: "file" },
  { name: "song.mp3", kind: "file" },
  { name: "readme.pdf", kind: "file" },
  { name: "sheet.xlsx", kind: "file" },
  { name: "tool.exe", kind: "file" },
  { name: "LICENSE", kind: "file" },
]);
const facets = collectTypeFacets(listing);
assert.deepEqual(
  facets.map((facet: TypeFacet) => facet.key),
  ["folder", "image", "video", "audio", "document", "other"],
  "只出现存在的类别，且按固定顺序",
);
const imageFacet = facets.find((facet: TypeFacet) => facet.key === "image");
assert.ok(imageFacet);
assert.equal(imageFacet.count, 3);
assert.deepEqual(
  imageFacet.extensions.map((entry) => entry.ext),
  ["png", "jpg"],
  "具体类型按数量降序",
);
assert.deepEqual(imageFacet.extensions.map((entry) => entry.count), [2, 1]);
const folderFacet = facets.find((facet: TypeFacet) => facet.key === "folder");
assert.deepEqual(folderFacet?.extensions, [], "文件夹没有具体类型");
const otherFacet = facets.find((facet: TypeFacet) => facet.key === "other");
assert.equal(otherFacet?.count, 2, "exe + 无扩展名都算其他");
assert.deepEqual(
  otherFacet?.extensions.map((entry) => entry.ext),
  ["exe"],
  "无扩展名不产生具体类型片",
);
assert.deepEqual(collectTypeFacets([]), [], "空列表没有清单");
// 穿透行与普通行走同一套类别逻辑（relativePath 不影响）。
const drillRows = rows([
  { name: "a.png", kind: "file" },
  { name: "nested", kind: "dir" },
]);
assert.deepEqual(
  collectTypeFacets(drillRows).map((facet: TypeFacet) => facet.key),
  ["folder", "image"],
);

// ---- 匹配语义 ----
const empty: string[] = [];
const everything = rows([
  { name: "dir", kind: "dir" },
  { name: "a.png", kind: "file" },
  { name: "b.mp4", kind: "file" },
]);
for (const entry of everything) {
  assert.equal(
    entryMatchesTypeFilter(entry, empty, empty),
    true,
    "无选择 = 不过滤",
  );
}
assert.equal(isTypeFilterActive([], []), false);
assert.equal(isTypeFilterActive(["image"], []), true);

const dir = rows([{ name: "sub", kind: "dir" }])[0];
const png = rows([{ name: "a.png", kind: "file" }])[0];
const jpg = rows([{ name: "b.jpg", kind: "file" }])[0];
const mp4 = rows([{ name: "c.mp4", kind: "file" }])[0];

assert.equal(entryMatchesTypeFilter(dir, ["folder"], []), true);
assert.equal(entryMatchesTypeFilter(png, ["folder"], []), false, "只选文件夹时文件隐藏");
assert.equal(entryMatchesTypeFilter(png, ["image"], []), true);
assert.equal(entryMatchesTypeFilter(dir, ["image"], []), false, "只选图片时文件夹隐藏");
assert.equal(entryMatchesTypeFilter(mp4, ["image"], []), false);

// 具体类型只细化所在大类：png 选中时 jpg 不再出现
assert.equal(entryMatchesTypeFilter(png, ["image"], ["png"]), true);
assert.equal(entryMatchesTypeFilter(jpg, ["image"], ["png"]), false);
assert.equal(entryMatchesTypeFilter(png, ["folder", "image"], ["png"]), true);
assert.equal(entryMatchesTypeFilter(dir, ["folder", "image"], ["png"]), true);

// ---- 过滤数组：未激活返回同内容，激活返回子集 ----
const source = rows([
  { name: "sub", kind: "dir" },
  { name: "a.png", kind: "file" },
  { name: "c.mp4", kind: "file" },
]);
assert.deepEqual(
  filterEntriesByType(source, empty, empty).map((entry) => entry.name),
  ["sub", "a.png", "c.mp4"],
);
assert.deepEqual(
  filterEntriesByType(source, ["image"], []).map((entry) => entry.name),
  ["a.png"],
);
assert.deepEqual(
  filterEntriesByType(source, ["folder", "image"], []).map((entry) => entry.name),
  ["sub", "a.png"],
);

// ---- 选择不变式：具体类型要求其大类被选中；去重、归一化、固定类别顺序 ----
const orphan = normalizeTypeFilterSelection(["image"], ["mp4"]);
assert.deepEqual(orphan.extensions, [], "视频扩展不能挂在图片大类下");
const cleaned = normalizeTypeFilterSelection(
  ["other", "image", "bogus", "image"],
  ["PNG", "png", "exe"],
);
assert.deepEqual(cleaned.categories, ["image", "other"], "固定顺序 + 去重 + 去未知");
assert.deepEqual(cleaned.extensions, ["png", "exe"], "小写化 + 去重");

// ---- 响应式动作：具体类型自动勾上父类；关掉大类带走其具体类型 ----
clearTypeFilter();
toggleTypeFilterExtension("png");
assert.deepEqual(typeFilterCategories.value, ["image"], "选具体类型自动勾父类");
assert.deepEqual(typeFilterExtensions.value, ["png"]);
toggleTypeFilterCategory("image");
assert.deepEqual(typeFilterCategories.value, [], "关闭大类");
assert.deepEqual(typeFilterExtensions.value, [], "其具体类型一并清除");
toggleTypeFilterCategory("folder");
assert.deepEqual(typeFilterCategories.value, ["folder"]);
assert.equal(typeFilterActive.value, true, "文件夹也是一种可勾选大类");
toggleTypeFilterCategory("folder");
assert.equal(typeFilterActive.value, false);
toggleTypeFilterCategory("bogus");
assert.deepEqual(typeFilterCategories.value, [], "未知类别不生效");
clearTypeFilter();
assert.deepEqual(typeFilterCategories.value, []);
assert.deepEqual(typeFilterExtensions.value, []);

console.log("navigator-type-filter checks OK (major categories + dynamic types).");
