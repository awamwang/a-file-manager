// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator 列表键入定位（type-ahead）：纯模块行为 + 视图接线静态守卫。 */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const module = readFileSync(join(root, "src/state/navigatorTypeAhead.ts"), "utf8");
const virtualWindow = readFileSync(join(root, "src/state/listVirtualWindow.ts"), "utf8");

// 纯模块导出
assert.ok(module.includes("export function feedChar"));
assert.ok(module.includes("export function findTypeAheadMatch"));
assert.ok(module.includes("export function nameStartsWith"));
assert.ok(module.includes("TYPE_AHEAD_TIMEOUT_MS"));
assert.ok(!module.includes("new Intl.Collator"), "纯模块不自建 collator（由视图注入）");

// 几何助手导出
assert.ok(virtualWindow.includes("export function scrollTopForIndex"));
assert.ok(virtualWindow.includes("export function scrollTopForVirtIndex"));
assert.ok(virtualWindow.includes("export function rowStridePx"));

// 视图接线
assert.ok(view.includes('from "../state/navigatorTypeAhead"'));
assert.ok(view.includes("feedChar(typeAheadState"));
assert.ok(view.includes("findTypeAheadMatch({"));
assert.ok(view.includes("TYPE_AHEAD_COLLATOR"));
assert.ok(view.includes('new Intl.Collator(undefined, { sensitivity: "base" })'));
assert.ok(view.includes("revealTypeAheadRow(matched, path)"));
assert.ok(view.includes("event.isComposing"), "IME 组字期拦截");
assert.ok(view.includes("event.keyCode === 229"), "WebView2 IME 229 兜底");
assert.ok(view.includes('event.key === "Process"'));
assert.ok(view.includes("scrollTopForIndex({"));
assert.ok(view.includes("scrollTopForVirtIndex(heights"));

// 排除空格与反引号（裸键快捷键）
assert.ok(view.includes('key === " " || key === "`"'));
// 缓冲清空时机
assert.ok(view.includes("typeAheadState = resetTypeAhead()"));
assert.ok(view.includes("watch([sortColumn, sortDirection]"));
assert.ok(view.includes("watch(activePane"));

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/type-ahead-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

console.log("type-ahead checks OK (view + module).");
