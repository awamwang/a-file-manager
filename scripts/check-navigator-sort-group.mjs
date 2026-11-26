// SPDX-License-Identifier: GPL-3.0-or-later
/** v17（ADR 0030）：排序/分组下拉 + 右键子菜单 + 持久化静态契约。 */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const view = read("src/views/NavigatorView.vue");
const mode = read("src/state/navigatorViewMode.ts");
const sort = read("src/state/navigatorSort.ts");
const grouping = read("src/state/navigatorGrouping.ts");
const shell = read("src/components/shell/AppShell.vue");
const ctxModel = read("src/components/navigator/contextMenuModel.ts");
const ctxMenu = read("src/components/navigator/NavigatorContextMenu.vue");
const glyphs = read("src/components/icons/glyphs.ts");
const fields = read("src/i18n/settingsFields.ts");
const types = read("src-tauri/src/settings/types.rs");
const defaults = read("src-tauri/src/settings/defaults.rs");
const migrate = read("src-tauri/src/settings/migrate.rs");

// ---- 后端 schema v17：新键 + 迁移 + 旧键保留（legacy） ----
// The sort/group keys landed in v17; later schema bumps (ADR 0031 → v18) must
// not break this gate, so assert the floor instead of pinning one version.
const schemaVersion = Number(
  types.match(/SETTINGS_SCHEMA_VERSION:\s*u32\s*=\s*(\d+)/)?.[1] ?? 0,
);
assert.ok(schemaVersion >= 17, `settings schema must be at least v17 (got v${schemaVersion})`);
for (const key of [
  "core.navigatorSortColumn",
  "core.navigatorSortDirection",
  "core.navigatorGroupBy",
  "core.navigatorGroupDirection",
]) {
  assert.ok(defaults.includes(`"${key}"`), `defaults.rs 定义 ${key}`);
}
assert.ok(defaults.includes('"core.navigatorGroupByType"'), "旧键保留（legacy）");
assert.ok(migrate.includes("(16, 17)"), "16→17 迁移步骤");
assert.ok(
  migrate.includes("navigatorGroupBy") && migrate.toLowerCase().includes("group_by_type"),
  "迁移把 groupByType 布尔映射为 navigatorGroupBy",
);

// ---- FE schema 清单与后端键名一致（改键名 = 静默断链） ----
for (const key of [
  "core.navigatorSortColumn",
  "core.navigatorSortDirection",
  "core.navigatorGroupBy",
  "core.navigatorGroupDirection",
]) {
  assert.ok(fields.includes(`"${key}"`), `settingsFields.ts 含 ${key}`);
}
assert.ok(fields.includes('"core.navigatorGroupByType"'), "旧键同步保留");

// ---- 排序状态：显式选列/方向 + 持久化 ----
assert.ok(sort.includes("export const sortColumn = ref"), "sortColumn ref 字面量");
assert.ok(sort.includes("export const sortDirection = ref"), "sortDirection ref 字面量");
assert.ok(sort.includes("export function setSortColumn"));
assert.ok(sort.includes("export function setSortDirection"));
assert.ok(sort.includes("export function toggleSort"));
assert.ok(sort.includes("applyNavigatorSortFromValues"));
assert.ok(sort.includes('"core.navigatorSortColumn"'));
assert.ok(sort.includes('"core.navigatorSortDirection"'));

// ---- 分组状态：字段/方向/折叠 + 切换重置 ----
assert.ok(mode.includes("export const navigatorGroupBy = ref"));
assert.ok(mode.includes("export const navigatorGroupDirection = ref"));
assert.ok(mode.includes("export const collapsedGroupKeys"));
assert.ok(mode.includes("function setNavigatorGroupBy"));
assert.ok(mode.includes("function setNavigatorGroupDirection"));
assert.ok(mode.includes("export function toggleGroupCollapsed"));
assert.ok(mode.includes("defaultDirectionForGroupField"));
assert.ok(mode.includes('navigatorGroupBy.value = "none"'));

// ---- 分组纯逻辑：字段分桶 + 桶垫底 ----
assert.ok(grouping.includes("export function groupEntriesBy"));
assert.ok(grouping.includes("export function nameBucketKey"));
assert.ok(grouping.includes("export function dateBucketKey"));
assert.ok(grouping.includes("FOLDER_GROUP_KEY"));
assert.ok(grouping.includes("pinnedLast"), "“#”/unknown 桶垫底机制");

// ---- NavigatorView：分组渲染 + 可折叠组头 ----
assert.ok(view.includes("isGrouped"));
assert.ok(view.includes("primaryGroups"));
assert.ok(view.includes("groupEntriesBy("));
assert.ok(view.includes("flattenGroupedItems(primaryGroups.value, collapsedGroupKeys.value)"));
assert.ok(view.includes("toggleGroupCollapsed(item.groupKey)"));
assert.ok(view.includes("'chevronRight'"), "折叠态 chevron");
assert.ok(view.includes("'chevronDown'"), "展开态 chevron");
assert.ok(view.includes("groupDateEarlier"));

// ---- 工具栏：排序/分组下拉（Win11 对齐） ----
assert.ok(shell.includes("'chrome.sortBy'"));
assert.ok(shell.includes("'chrome.groupBy'"));
assert.ok(shell.includes("chooseSortColumn"));
assert.ok(shell.includes("chooseSortDirection"));
assert.ok(shell.includes("chooseGroupField"));
assert.ok(shell.includes("chooseGroupDirection"));
assert.ok(shell.includes("sortMenuOpen"));
assert.ok(shell.includes("groupMenuOpen"));
assert.ok(shell.includes('id="navigator-sort-menu"'));
assert.ok(shell.includes('id="navigator-group-menu"'));
// 工具条顺序：分组紧跟排序，中间不能被视图切换隔开（两者是同一族操作）。
const sortMenuAt = shell.indexOf('id="navigator-sort-menu"');
const groupMenuAt = shell.indexOf('id="navigator-group-menu"');
const viewModeAt = shell.indexOf('id="navigator-view-mode-menu"');
assert.ok(sortMenuAt > 0 && groupMenuAt > 0 && viewModeAt > 0, "三个菜单都在工具条里");
assert.ok(
  groupMenuAt > sortMenuAt && groupMenuAt < viewModeAt,
  "分组按钮必须排在排序之后、视图切换之前",
);

// ---- 右键菜单：空白菜单排序/分组子菜单（含 radio 状态） ----
assert.ok(ctxModel.includes("buildSortGroupMenuItems(options)"), "空白菜单挂排序/分组子菜单");
assert.ok(ctxModel.includes("`sortColumn:${"));
assert.ok(ctxModel.includes('"sortDirection:asc"'));
assert.ok(ctxModel.includes('"sortDirection:desc"'));
assert.ok(ctxModel.includes("`groupBy:${"));
assert.ok(ctxModel.includes('"groupDirection:asc"'));
assert.ok(ctxModel.includes('"groupDirection:desc"'));
assert.ok(ctxModel.includes("checked?: boolean"));
assert.ok(ctxMenu.includes("sortState:"));
assert.ok(ctxMenu.includes("groupState:"));
assert.ok(ctxMenu.includes("setSortColumn"));
assert.ok(ctxMenu.includes("setNavigatorGroupBy"));
assert.ok(ctxMenu.includes("menuitemcheckbox"), "子菜单 radio/checkbox 语义");

// ---- 图标：方向箭头 + 排序/分组隐喻 ----
assert.ok(glyphs.includes('"arrowUp"'));
assert.ok(glyphs.includes('"arrowDown"'));
assert.ok(glyphs.includes('"sort"'));
assert.ok(glyphs.includes('"group"'));
assert.ok(glyphs.includes('"chevronRight"'));

// ---- i18n：六语言键齐备 ----
const LOCALES = ["en", "zh-CN", "zh-TW", "ja", "de", "fr"];
for (const loc of LOCALES) {
  const doc = JSON.parse(read(`src/i18n/locales/${loc}.json`));
  for (const key of [
    "sortBy",
    "groupBy",
    "groupNone",
    "sortAscending",
    "sortDescending",
    "sortName",
    "sortModified",
    "sortType",
    "sortSize",
  ]) {
    assert.ok(
      typeof doc.chrome[key] === "string" && doc.chrome[key].length > 0,
      `${loc}.chrome.${key}`,
    );
  }
  assert.ok(!("viewGroupByType" in doc.chrome), `${loc}.chrome.viewGroupByType 已删除`);
  for (const key of [
    "groupNameOther",
    "groupSizeEmpty",
    "groupSizeSmall",
    "groupSizeMedium",
    "groupSizeLarge",
    "groupSizeHuge",
    "groupDateUnknown",
    "groupDateToday",
    "groupDateYesterday",
    "groupDateThisWeek",
    "groupDateLastWeek",
    "groupDateEarlier",
  ]) {
    assert.ok(
      typeof doc.navigator[key] === "string" && doc.navigator[key].length > 0,
      `${loc}.navigator.${key}`,
    );
  }
  const core = doc.settings.fields.core;
  for (const key of [
    "navigatorSortColumn",
    "navigatorSortDirection",
    "navigatorGroupBy",
    "navigatorGroupDirection",
  ]) {
    assert.ok(core[key]?.title, `${loc}.settings.fields.core.${key}.title`);
    assert.ok(core[key]?.description, `${loc}.settings.fields.core.${key}.description`);
  }
  assert.ok(core.navigatorSortColumn.options?.name, `${loc} sortColumn options`);
  assert.ok(core.navigatorGroupBy.options?.none, `${loc} groupBy options`);
  assert.ok(core.navigatorSortDirection.options?.asc, `${loc} sortDirection options`);
}
const en = JSON.parse(read("src/i18n/locales/en.json"));
assert.equal(en.chrome.sortBy, "Sort by");
assert.equal(en.chrome.groupBy, "Group by");
const zh = JSON.parse(read("src/i18n/locales/zh-CN.json"));
assert.equal(zh.chrome.sortBy, "排序依据");
assert.equal(zh.chrome.groupBy, "分组依据");

// ---- 真执行：分桶/折叠/方向/水合纯逻辑 ----
const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/navigator-sort-group-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0, "navigator-sort-group-checks.ts 执行失败");

console.log("navigator-sort-group checks OK (v17 / ADR 0030).");
