// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Navigator type filter — static contract + pure logic regression.
 *
 * Covers: category/facet/match logic exports, the toolbar dialog (trigger,
 * popover, dynamic categories + specific types, clear), wiring into the
 * Navigator row sources (plain listing AND drill-through rows), the empty
 * banner, the status-bar counts, the glyph catalog entries and i18n keys in
 * all six locales — plus scripts/navigator-type-filter-checks.ts executed.
 */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8").replace(/\r\n/g, "\n");

const state = read("src/state/navigatorTypeFilter.ts");
const view = read("src/views/NavigatorView.vue");
const shell = read("src/components/shell/AppShell.vue");
const glyphs = read("src/components/icons/glyphs.ts");

// ---- 状态模块：大类 + 动态清单 + 匹配语义 + 会话选择 ----
for (const key of [
  "folder",
  "image",
  "video",
  "audio",
  "document",
  "archive",
  "code",
  "other",
]) {
  assert.ok(
    state.includes(`"${key}"`),
    `major category registry must contain ${key}`,
  );
}
assert.ok(state.includes("export const TYPE_FILTER_CATEGORY_ORDER"), "类别固定顺序");
assert.ok(state.includes("export function typeCategoryForExtension"), "扩展名 → 大类");
assert.ok(state.includes("export function entryTypeCategory"), "行 → 大类");
assert.ok(state.includes("export function collectTypeFacets"), "动态清单");
assert.ok(state.includes("export function entryMatchesTypeFilter"), "匹配语义");
assert.ok(state.includes("export function filterEntriesByType"), "过滤助手");
assert.ok(state.includes("export function normalizeTypeFilterSelection"), "选择不变式");
assert.ok(state.includes("export function isTypeFilterActive"), "激活判定");
assert.ok(state.includes("export const typeFilterCategories"), "大类选择状态");
assert.ok(state.includes("export const typeFilterExtensions"), "具体类型选择状态");
assert.ok(state.includes("export const typeFilterDialogOpen"), "弹窗开关状态");
assert.ok(state.includes("export const typeFilterFacetEntries"), "清单数据源状态");
assert.ok(state.includes("export const typeFilterActive"), "激活 computed");
assert.ok(state.includes("export function setTypeFilterFacetEntries"), "清单同步入口");
assert.ok(state.includes("export function toggleTypeFilterCategory"), "切换大类");
assert.ok(state.includes("export function toggleTypeFilterExtension"), "切换具体类型");
assert.ok(state.includes("export function clearTypeFilter"), "清空筛选");
assert.ok(state.includes("export function toggleTypeFilterDialog"), "弹窗开合");
// 会话态：不落盘、不迁移 settings。
assert.ok(!state.includes("settings_set"), "筛选选择是会话态，不写设置");

// ---- NavigatorView：行来源过滤（含穿透）+ 清单同步 + 空态 ----
assert.ok(view.includes("filterEntriesByType"), "可见行经过类型过滤");
assert.ok(view.includes("const primaryRowSource"), "主窗格未过滤行来源");
assert.ok(view.includes("const secondaryRowSource"), "副窗格未过滤行来源");
assert.ok(
  view.includes(
    "filterEntriesByType(\n      primaryRowSource.value,",
  ),
  "主窗格行来源应用类型过滤（普通列表/搜索/穿透共用）",
);
assert.ok(
  view.includes(
    "filterEntriesByType(\n      secondaryRowSource.value,",
  ),
  "副窗格行来源应用类型过滤",
);
assert.ok(
  view.includes("primaryDrill.entries.value") &&
    view.includes("return listing.value?.entries ?? [];"),
  "行来源优先级：搜索 → 穿透 → 目录列表",
);
assert.ok(view.includes("setTypeFilterFacetEntries(entries)"), "清单数据跟随主窗格行来源");
assert.ok(view.includes("typeFilterActive &&"), "空态横幅检查类型筛选激活");
assert.ok(view.includes('t("navigator.typeFilterEmpty")'), "空态文案");
assert.ok(view.includes("clearTypeFilter()"), "空态横幅可一键清除");

// ---- AppShell：工具栏弹窗（触发钮 + 面板 + 动态类别/具体类型 + 清除） ----
assert.ok(shell.includes('id="navigator-type-filter-menu"'), "弹窗容器");
assert.ok(shell.includes('name="filter"'), "漏斗触发图标");
assert.ok(shell.includes("toggleTypeFilterMenu"), "弹窗开关");
assert.ok(shell.includes("closeTypeFilterMenu"), "关闭弹窗");
assert.ok(shell.includes("const typeFacets = computed("), "动态清单 computed");
assert.ok(shell.includes("toggleTypeFilterCategory(facet.key)"), "类别勾选");
assert.ok(shell.includes("toggleTypeFilterExtension(extFacet.ext)"), "具体类型勾选");
assert.ok(shell.includes('role="menuitemcheckbox"'), "勾选语义");
assert.ok(shell.includes(":aria-checked=\"typeCategoryChecked(facet.key)\""), "类别勾选态");
assert.ok(shell.includes("chooseTypeFilterClear"), "全部类型（清除）");
assert.ok(shell.includes("navigator.typeFilterTitle"), "弹窗标题");
assert.ok(shell.includes("navigator.typeFilterFormats"), "具体类型展开说明");
assert.ok(shell.includes("navigator.typeFilterNoTypes"), "无行可筛时的占位");
assert.ok(shell.includes("typeFilterDialogOpen || typeFilterActive"), "触发钮激活态");

// ---- 菜单互斥 / 点击外部 / Esc ----
assert.ok(
  shell.includes("closeTypeFilterDialog();\n  drillMenuOpen.value = !drillMenuOpen.value;"),
  "打开穿透菜单时关闭类型弹窗",
);
assert.ok(
  shell.includes("closeTypeFilterMenu();\n  closeCrumbMenu();"),
  "closeChromeMenus 关闭类型弹窗",
);
assert.ok(
  shell.includes('const typeFilterRoot = document.getElementById("navigator-type-filter-menu");'),
  "点击外部锚点",
);
assert.ok(
  shell.includes("typeFilterDialogOpen.value &&"),
  "点击外部/Esc 检查弹窗开启",
);
assert.ok(
  shell.includes("!typeFilterRoot.contains(target)"),
  "点击弹窗外关闭",
);
assert.ok(
  /drillMenuOpen\.value \|\|\n\s*typeFilterDialogOpen\.value \|\|/.test(shell),
  "Esc 关闭类型弹窗",
);

// ---- 状态栏计数跟随筛选（含穿透行） ----
assert.ok(shell.includes("typeFilteredDrillEntries"), "穿透行计数随筛选收窄");
assert.ok(shell.includes("typeFilteredListingEntries"), "目录列表计数随筛选收窄");
assert.ok(shell.includes('t("navigator.typeFilterStatus"'), "状态栏筛选提示");

// ---- 字形：漏斗与代码为新登记字形 ----
assert.ok(glyphs.includes('| "filter"'), "filter 字形 id 登记");
assert.ok(glyphs.includes("filter: {"), "filter 字形定义");
assert.ok(glyphs.includes('| "code"'), "code 字形 id 登记");
assert.ok(glyphs.includes("code: {"), "code 字形定义");
for (const existing of [
  "folder",
  "picture",
  "video",
  "musicNote",
  "document",
  "library",
  "board",
]) {
  assert.ok(glyphs.includes(`${existing}: {`), `类别复用已有字形 ${existing}`);
}

// ---- i18n：六语言键齐备 ----
const LOCALES = ["en", "zh-CN", "zh-TW", "ja", "de", "fr"];
for (const loc of LOCALES) {
  const doc = JSON.parse(read(`src/i18n/locales/${loc}.json`));
  for (const key of [
    "typeFilterToggle",
    "typeFilterTitle",
    "typeFilterNoTypes",
    "typeFilterFormats",
    "typeFilterClear",
    "typeFilterEmpty",
    "typeFilterStatus",
    "typeCatFolder",
    "typeCatImage",
    "typeCatVideo",
    "typeCatAudio",
    "typeCatDocument",
    "typeCatArchive",
    "typeCatCode",
    "typeCatOther",
  ]) {
    assert.ok(
      typeof doc.navigator[key] === "string" && doc.navigator[key].length > 0,
      `${loc}.navigator.${key}`,
    );
  }
}

// ---- 真执行：纯逻辑回归 ----
const registerUrl = pathToFileURL(join(root, "scripts/register-ts-resolve.mjs")).href;
const result = spawnSync(
  process.execPath,
  [
    "--import",
    registerUrl,
    "--experimental-strip-types",
    join(root, "scripts/navigator-type-filter-checks.ts"),
  ],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0, "navigator-type-filter-checks.ts 执行失败");

console.log("navigator-type-filter checks OK (toolbar dialog + drill-through filtering).");
