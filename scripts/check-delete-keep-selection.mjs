// SPDX-License-Identifier: GPL-3.0-or-later
// 删除后保持选中：删除/永久删除成功后，选中移到被删条目的下一个可见条目；
// 删到末尾回退到剩余最后一项，目录清空才清除选中（Explorer 习惯）。
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const selection = readFileSync(join(root, "src/state/navigatorSelection.ts"), "utf8");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const menu = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);
const reveal = readFileSync(join(root, "src/state/revealInNavigator.ts"), "utf8");
const pkg = readFileSync(join(root, "package.json"), "utf8");

// 共享算法：后继 → 末尾回退 → 全空 null，且必须在移除 listing 前调用。
assert.ok(selection.includes("export function nextSelectionAfterRemoval("));
assert.match(
  selection,
  /nextSelectionAfterRemoval[\s\S]*?lastIndex[\s\S]*?lastIndex \+ 1[\s\S]*?lastIndex - 1/,
);
assert.ok(selection.includes("normalizePathKey"));
assert.match(
  selection,
  /nextSelectionAfterRemoval[\s\S]*?if \(!orderedPaths\.length \|\| !removedPaths\.length\) return null;/,
);

// 滚动到新选中行：复用 reveal 的 data-entry-path 定位并导出。
assert.ok(reveal.includes("export function scrollSelectionIntoView("));

// 键盘删除路径（NavigatorView）：删除前基于活动窗格顺序计算，刷新后再恢复选中。
assert.ok(view.includes("nextSelectionAfterRemoval"));
assert.ok(view.includes("activeOrderedPaths"));
assert.match(
  view,
  /const activeOrderedPaths = computed\(\(\) =>[\s\S]*?activePane\.value === "secondary"[\s\S]*?orderedPaths\.value,/,
);
assert.match(
  view,
  /async function afterShortcutDelete[\s\S]*?nextSelectionAfterRemoval\(activeOrderedPaths\.value, paths\)[\s\S]*?removePathsFromListings\(paths\)[\s\S]*?await refreshListing\(\);[\s\S]*?selectOnly\(nextPath\);[\s\S]*?scrollSelectionIntoView\(nextPath\);/,
);
// 选中恢复必须在 removePathsFromListings 之前计算（顺序快照语义）。
assert.ok(
  view.indexOf("nextSelectionAfterRemoval(activeOrderedPaths.value, paths)") <
    view.indexOf("removePathsFromListings(paths)"),
);
assert.ok(view.includes(':ordered-paths="activeOrderedPaths"'));

// 右键菜单删除路径：runDelete 入口先快照 prop，成功回调用快照算下一个选中。
assert.ok(menu.includes("orderedPaths?: string[]"));
assert.match(
  menu,
  /async function runDelete\(entry: NavigatorEntry\) \{[\s\S]*?const orderedSnapshot = \[\.\.\.\(props\.orderedPaths \?\? \[\]\)\];/,
);
assert.match(
  menu,
  /async function afterDeleteSuccess\(\s*paths: string\[\],\s*notice: string,\s*orderedSnapshot: string\[\],\s*\)/,
);
assert.match(
  menu,
  /afterDeleteSuccess\([\s\S]*?nextSelectionAfterRemoval\(orderedSnapshot, paths\)[\s\S]*?await refreshListing\(\);[\s\S]*?selectOnly\(nextPath\);[\s\S]*?scrollSelectionIntoView\(nextPath\);/,
);
assert.ok(menu.includes("afterDeleteSuccess(done, t(\"contextMenu.deletedPermanent\"), orderedSnapshot)"));
assert.ok(menu.includes("afterDeleteSuccess(paths, t(\"contextMenu.deleted\"), orderedSnapshot)"));
// 快照必须先于任何会改变 listing 的调用。
assert.ok(
  menu.indexOf("const orderedSnapshot") < menu.indexOf("deleteEntries(recyclable, false)"),
);

assert.ok(pkg.includes('"test:delete-keep-selection"'));

console.log("delete-keep-selection OK.");
