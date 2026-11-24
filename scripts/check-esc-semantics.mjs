// SPDX-License-Identifier: GPL-3.0-or-later
// SEL-010 (已拍板 2026-11-22): Esc 三层优先级静态钉 —
//   1. rename inline：重命名输入框自身的 @keydown.escape 拦截（优先级最高）；
//   2. 有选中：列表键盘 handler 清空选中并消费事件；
//   3. 无选中：不消费，交给全局处理器。
// 行为层（三层判定顺序）由 esc-semantics-checks.ts 真执行。
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const selection = readFileSync(
  join(root, "src/state/navigatorSelection.ts"),
  "utf8",
);

// 第 1 层：inline rename 输入框在 Esc 时取消 rename 并阻止冒泡。
assert.ok(
  view.includes('@keydown.escape.prevent.stop="cancelInlineRename"'),
  "SEL-010 layer 1: inline rename input must cancel on Esc (.stop keeps it highest priority)",
);

// 第 2/3 层：列表键盘 handler 走纯函数判定，清空选中并消费事件。
assert.ok(
  view.includes("escSemantics(false, selectedPaths.value.length)"),
  "SEL-010 layer 2/3: the list Escape branch must route through escSemantics",
);
assert.ok(
  /if \(event\.key === "Escape"\) \{[\s\S]{0,220}?clearSelection\(\);[\s\S]{0,120}?stopPropagation\(\);/.test(
    view,
  ),
  "SEL-010 layer 2: Escape with a selection must clearSelection + consume the event",
);
assert.ok(
  selection.includes("export function escSemantics"),
  "SEL-010: the pure three-layer decision must live in navigatorSelection.ts",
);

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/esc-semantics-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

console.log("Esc semantics checks OK (SEL-010).");
