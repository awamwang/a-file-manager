// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 知识页「在 AFM 中显示」基线门禁（ADR 0029）。
 *
 * 知识页（搜索面板 → 知识源命中 → Enter 打开的单实例 Markdown 弹窗）右上角的
 * 「在 AFM 中显示」按钮，行为口径：
 *   1. 不再拉起**系统**文件管理器（revealItemInDir），改为留在 AFM 内定位；
 *   2. 关掉知识页 → 切资源管理器路由 → 在**当前打开组（标签条）的最后**新建一个
 *      标签打开笔记所在目录，并在列表里选中该 .md；
 *   3. 落位永远是末尾新标签：同一目录已在别处打开也不复用那个标签。
 *
 * 回归信号：按钮改回系统 Explorer、落位跳回标签条中间、文案又去写系统文件管理器
 * 专名（访达 / Finder）——任一出现都算行为回归，门禁必须红。
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const readFrom = (rel) => readFileSync(join(root, rel), "utf8");

// --- 1. 知识页弹窗：改走 AFM 资源管理器，不再拉起系统文件管理器 ---
const dialog = readFrom("src/components/knowledge/KnowledgeDocDialog.vue");
assert.ok(
  !dialog.includes("revealItemInDir"),
  "知识页弹窗不得再调 revealItemInDir（ADR 0029：该按钮改为 AFM 内定位）",
);
assert.ok(
  dialog.includes("onShowInNavigator") &&
    dialog.includes('@click="onShowInNavigator"'),
  "知识页按钮必须绑定 onShowInNavigator",
);
assert.ok(
  /revealItemsInNavigator\(\s*\[target\]\s*,\s*\{\s*appendTab:\s*true\s*\}\s*\)/.test(dialog),
  "知识页按钮必须以 appendTab:true 调用 revealItemsInNavigator（落位=当前打开组最后）",
);
assert.ok(
  dialog.includes("closeKnowledgeDoc()") &&
    dialog.indexOf("closeKnowledgeDoc()") < dialog.indexOf("revealItemsInNavigator"),
  "知识页必须先 closeKnowledgeDoc() 再跳资源管理器（弹窗 z-index 1050 会盖住资源管理器）",
);
assert.ok(
  dialog.includes('t("knowledgeView.showInNavigator")'),
  "知识页按钮文案必须用 knowledgeView.showInNavigator",
);

// --- 2. 状态层：appendTab = 标签条末尾新建标签；默认分支仍是复用 ---
const reveal = readFrom("src/state/revealInNavigator.ts");
assert.ok(
  reveal.includes("appendTab?: boolean"),
  "revealInNavigator 必须提供 appendTab 选项（末尾新建 vs 复用）",
);
assert.ok(
  reveal.includes("newNavigatorTab(parentDir)"),
  "appendTab 分支必须走 newNavigatorTab（末尾新建标签）",
);
assert.ok(
  reveal.includes("openOrFocusNavigatorTab(parentDir)"),
  "默认分支必须仍是 open-or-focus（复用已打开标签）",
);
const browse = readFrom("src/state/navigatorBrowse.ts");
assert.ok(
  browse.includes("openStateWithNewTab(liveOpenState.value, target)"),
  "newNavigatorTab 必须用 openStateWithNewTab（标签条末尾追加并激活）",
);

// --- 3. 文案：6 语言统一「在 AFM 中显示」，系统文件管理器专名退役 ---
const localeIds = ["en", "zh-CN", "zh-TW", "ja", "de", "fr"];
for (const id of localeIds) {
  const catalog = JSON.parse(readFrom(`src/i18n/locales/${id}.json`));
  const label = catalog.knowledgeView?.showInNavigator;
  assert.ok(
    typeof label === "string" && label.trim().length > 0,
    `${id} 缺少 knowledgeView.showInNavigator`,
  );
  assert.ok(
    !("openInExplorer" in (catalog.knowledgeView ?? {})),
    `${id}: knowledgeView.openInExplorer 必须退役（它指系统文件管理器）`,
  );
  assert.ok(
    !("knowledgeView.openInExplorer" in (catalog.platform?.macos ?? {})),
    `${id}: macOS 覆盖不得再有 knowledgeView.openInExplorer（按钮已不打开系统文件管理器）`,
  );
}

// --- 4. 平台词门禁不得再把知识页当系统文件管理器文案 ---
const platformGate = readFrom("scripts/check-platform-terms.mjs");
assert.ok(
  !platformGate.includes("knowledgeView.openInExplorer"),
  "check-platform-terms 不得再要求知识页写 Finder / 访达（按钮已不打开系统文件管理器）",
);

console.log(
  "knowledge reveal-in-navigator OK (AFM-internal reveal; new tab at the end of the current open group; 6 locales on the AFM wording).",
);
