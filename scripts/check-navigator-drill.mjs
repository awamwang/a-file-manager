// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * ADR 0031 穿透（drill-through）静态契约 + 纯逻辑回归。
 *
 * 覆盖：后端命令与深度语义、settings v18 迁移、FE 状态与工具栏/面包屑/
 * 状态栏/标签/行内路径五处显示改写、写入落点=穿透根、排序键=i18n 六语言、
 * 以及 scripts/navigator-drill-checks.ts 的真执行。
 */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const drillRust = read("src-tauri/src/navigator/drill.rs");
const navMod = read("src-tauri/src/navigator/mod.rs");
const lib = read("src-tauri/src/lib.rs");
const types = read("src-tauri/src/settings/types.rs");
const defaults = read("src-tauri/src/settings/defaults.rs");
const migrate = read("src-tauri/src/settings/migrate.rs");
const state = read("src/state/navigatorDrill.ts");
const browse = read("src/state/navigatorBrowse.ts");
const sort = read("src/state/navigatorSort.ts");
const session = read("src/state/applySessionSettings.ts");
const shell = read("src/components/shell/AppShell.vue");
const view = read("src/views/NavigatorView.vue");
const glyphs = read("src/components/icons/glyphs.ts");
const fields = read("src/i18n/settingsFields.ts");
const settingsUi = read("src/state/settingsUi.ts");
const mock = read("webdemo/mocks/vfs.ts");

// ---- 后端：独立命令 + 深度语义 + 上限 ----
assert.ok(drillRust.includes("pub fn drill_directory"), "drill.rs 提供 drill_directory");
assert.ok(drillRust.includes("pub struct DrillEntry"), "DrillEntry 结构");
assert.ok(drillRust.includes("pub struct DrillListing"), "DrillListing 结构");
assert.ok(drillRust.includes("pub relative_path: String"), "条目带根相对路径");
assert.ok(drillRust.includes("pub source_dir_count: u64"), "来源子文件夹计数");
assert.ok(drillRust.includes("pub truncated: bool"), "截断标志");
assert.ok(drillRust.includes("const MAX_RESULTS: usize = 2_000;"), "结果上限 2000");
assert.ok(drillRust.includes("const MAX_VISITED: usize = 20_000;"), "访问上限 20000");
assert.ok(drillRust.includes("pub const MAX_DRILL_DEPTH: u32 = 5;"), "最深 5 层");
assert.ok(drillRust.includes("max_depth(max_walk_depth)"), "按深度限制遍历");
assert.ok(drillRust.includes("min_depth(1)"), "不含根自身");
assert.ok(drillRust.includes("follow_links(false)"), "不跟随链接");
assert.ok(drillRust.includes("depth == 0"), "深度 0 = 关闭（不遍历）");
assert.ok(navMod.includes("mod drill;"), "navigator/mod.rs 声明 drill 模块");
assert.ok(navMod.includes("pub async fn navigator_drill_list"), "navigator_drill_list 命令");
assert.ok(navMod.includes("spawn_blocking"), "慢路径走 blocking 池");
assert.ok(lib.includes("navigator::navigator_drill_list"), "lib.rs 注册命令");
// 既有搜索命令语义不动（空查询不遍历）。
assert.ok(read("src-tauri/src/navigator/search.rs").includes("if query.is_empty()"));

// ---- settings v18：新键 + 迁移 + FE 清单 + 设置页隐藏 ----
// The drill depth key landed in v18; later schema bumps must not break this gate.
const schemaVersion = Number(
  types.match(/SETTINGS_SCHEMA_VERSION:\s*u32\s*=\s*(\d+)/)?.[1] ?? 0,
);
assert.ok(schemaVersion >= 18, `settings schema must be at least v18 (got v${schemaVersion})`);
assert.ok(defaults.includes('"core.navigatorDrillDepth"'), "defaults.rs 定义新键");
assert.ok(defaults.includes("SettingValueKind::Number"), "档位是数字");
assert.ok(migrate.includes("(17, 18)"), "17→18 迁移步骤");
assert.ok(fields.includes('"core.navigatorDrillDepth"'), "settingsFields.ts 含新键");
assert.ok(settingsUi.includes('"core.navigatorDrillDepth"'), "设置页隐藏（唯一编辑入口在工具栏）");
assert.ok(session.includes("applyNavigatorDrillDepthFromValues"), "启动水合应用档位");

// ---- FE 状态：会话开关 + 持久化档位 + 树内判定 + 写入落点 ----
assert.ok(state.includes("export const drillDepth = ref"), "档位 ref");
assert.ok(state.includes("export function setDrillDepth"), "档位持久化");
assert.ok(state.includes('"core.navigatorDrillDepth"'), "写盘到 settings");
assert.ok(state.includes("export async function enableDrill"), "开启穿透");
assert.ok(state.includes("export function clearDrill"), "关闭穿透");
assert.ok(state.includes("export function isInsideDrillTree"), "树内判定");
assert.ok(state.includes("export function drillTargetPath"), "写入落点");
assert.ok(state.includes("export function resolveDrillPane"), "焦点窗格判定");
assert.ok(state.includes("export const MAX_DRILL_DEPTH = 5;"), "档位上限 5");
assert.ok(state.includes('invoke<DrillListingWire>("navigator_drill_list"'), "调用新命令");
// 开关是会话态：不得随 settings 落盘。
assert.ok(!state.includes('"core.navigatorDrillEnabled"'), "开关不落盘");

// ---- 离开这棵树则自动关闭 ----
assert.ok(browse.includes("syncDrillAfterNavigation"), "导航后同步穿透态");
assert.ok(browse.includes('import {\n  clearDrill,'), "浏览状态引用穿透模块");
assert.ok(browse.includes("await refreshDrill(\"primary\")"), "变更后刷新摊平行");
assert.ok(browse.includes("await refreshDrill(\"secondary\")"), "副窗格变更后刷新");

// ---- 工具栏：图标 + chevron 弹层（关闭 / 1..5） ----
assert.ok(shell.includes('id="navigator-drill-menu"'), "穿透菜单容器");
assert.ok(shell.includes('name="drill"'), "穿透图标");
assert.ok(shell.includes("chooseDrillDepth"), "选择深度档位");
assert.ok(shell.includes("chooseDrillOff"), "关闭穿透");
assert.ok(shell.includes("DRILL_DEPTHS"), "档位列表");
assert.ok(shell.includes("menuitemradio"), "档位单选语义");
assert.ok(shell.includes("drillMenuOpen"), "菜单开合状态");

// ---- 五处路径显示改写 ----
assert.ok(shell.includes("crumb--drill"), "面包屑 chip");
assert.ok(shell.includes("chrome.drillChip"), "面包屑 chip 文案");
assert.ok(shell.includes("chrome.drillSources"), "状态栏来源子文件夹数");
assert.ok(shell.includes("tabDrillSuffix(tab.path)"), "标签标题后缀");
assert.ok(shell.includes("chrome.drillTabSuffix"), "标签后缀文案");
assert.ok(view.includes("pane-path-drill"), "窗格路径行徽标");
assert.ok(view.includes("navigator.drillBadge"), "窗格徽标文案");
assert.ok(view.includes("entryRelativePath(item.entry)"), "行内相对路径");
assert.ok(view.includes("navigator.drillTruncated"), "截断横幅");

// ---- 写入落点=穿透根；双击=树内定位 ----
assert.ok(view.includes("drillTargetPath(\"primary\", currentPath.value)"), "新建落点=穿透根");
assert.ok(view.includes("drillTargetPath(\"primary\", currentPath.value)"), "粘贴落点=穿透根");
assert.ok(view.includes("navigator.drillWriteTarget"), "确认框写明落点");
assert.ok(view.includes("primaryDrill.enabled.value"), "树内定位条件");

// ---- 排序：名称按相对路径 ----
assert.ok(sort.includes("function sortNameKey"), "排序键取相对路径");
assert.ok(sort.includes("entry.relativePath || entry.name"), "相对路径优先");

// ---- 字形：AFM 自绘（Segoe 无对应符号） ----
assert.ok(glyphs.includes('| "drill"'), "字形 id 登记");
assert.ok(glyphs.includes("drill: {"), "字形定义");
assert.ok(/drill:\s*\{[\s\S]{0,400}svgOnly: true/.test(glyphs), "drill 是 svgOnly 自绘字形");

// ---- webdemo mock 同步新命令 ----
assert.ok(mock.includes('case "navigator_drill_list"'), "mock 实现新命令");
assert.ok(mock.includes('"core.navigatorDrillDepth": 2'), "mock 设置含新键");

// ---- i18n：六语言键齐备 ----
const LOCALES = ["en", "zh-CN", "zh-TW", "ja", "de", "fr"];
for (const loc of LOCALES) {
  const doc = JSON.parse(read(`src/i18n/locales/${loc}.json`));
  for (const key of [
    "drillThrough",
    "drillOff",
    "drillDepth",
    "drillChip",
    "drillChipTitle",
    "drillTabSuffix",
    "drillSources",
  ]) {
    assert.ok(
      typeof doc.chrome[key] === "string" && doc.chrome[key].length > 0,
      `${loc}.chrome.${key}`,
    );
  }
  for (const key of ["drillBadge", "drillWriteTarget", "drillTruncated"]) {
    assert.ok(
      typeof doc.navigator[key] === "string" && doc.navigator[key].length > 0,
      `${loc}.navigator.${key}`,
    );
  }
  const core = doc.settings.fields.core;
  assert.ok(core.navigatorDrillDepth?.title, `${loc} settings.fields.core.navigatorDrillDepth.title`);
  assert.ok(
    core.navigatorDrillDepth?.description,
    `${loc} settings.fields.core.navigatorDrillDepth.description`,
  );
}

// ---- 真执行：纯逻辑回归 ----
const registerUrl = pathToFileURL(join(root, "scripts/register-ts-resolve.mjs")).href;
const result = spawnSync(
  process.execPath,
  [
    "--import",
    registerUrl,
    "--experimental-strip-types",
    join(root, "scripts/navigator-drill-checks.ts"),
  ],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0, "navigator-drill-checks.ts 执行失败");

console.log("navigator-drill checks OK (ADR 0031).");
