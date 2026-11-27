// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator 大目录列表窗口化门槛（PERF）。 */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const state = readFileSync(join(root, "src/state/listVirtualWindow.ts"), "utf8");
const virtItems = readFileSync(join(root, "src/state/listVirtItems.ts"), "utf8");
const sortState = readFileSync(join(root, "src/state/navigatorSort.ts"), "utf8");

assert.ok(state.includes("computeListVirtualWindow"));
assert.ok(state.includes("computeVariableListVirtualWindow"));
assert.ok(state.includes("computeThumbRequestWindow"), "FIL-50 thumb viewport window");
assert.ok(state.includes("LIST_VIRTUALIZE_MIN_ENTRIES"));
assert.ok(state.includes("THUMB_REQUEST_OVERSCAN_ROWS"));
assert.ok(virtItems.includes("flattenGroupedItems"));
assert.ok(virtItems.includes("LIST_GROUP_HEADER_HEIGHT_PX"));
// 分组 + 图标/平铺视图：组内按列流动（组头横跨整行），不再是「一行一个」。
assert.ok(virtItems.includes("buildGridRows"), "grouped grid row mapping");
assert.ok(virtItems.includes("computeGridListVirtualWindow"), "grouped grid window");
assert.ok(virtItems.includes("buildGroupedGridLayout"), "grouped grid visual layout");
assert.ok(virtItems.includes("nextGroupedGridPath"), "grouped grid arrow navigation");
assert.ok(
  view.includes("list--grouped-grid"),
  "grouped icon/tiles views must use the grid canvas, not a single-column flow",
);
assert.ok(
  view.includes("groupedGridColumns"),
  "grouped grid column count must be explicit so the virtual window matches the layout",
);
assert.ok(
  view.includes("--group-cols"),
  "the grid canvas must receive the explicit column count",
);
assert.ok(
  !view.includes("'list--canvas': (isIconView || isTilesView) && !isGrouped"),
  "the secondary pane is never grouped — it must keep the grid canvas",
);
assert.ok(sortState.includes("clearSortedEntriesCache"));
assert.ok(sortState.includes("list_sort"));
assert.ok(view.includes("listVirtualWindow") || view.includes("computeListVirtualWindow"));
assert.ok(view.includes("computeVariableListVirtualWindow"));
assert.ok(view.includes("primaryWindowedItems") || view.includes("primaryWindowedEntries"));
assert.ok(view.includes("list--virt-flat"));
assert.ok(view.includes("list_window"));
assert.ok(view.includes("virt-spacer") || view.includes("offsetBeforePx"));
assert.ok(
  view.includes("gridVirtualPaddingStyle"),
  "FIL-58: icon/tiles virtualize via padding offsets (not in-grid spacers)",
);
assert.ok(
  !view.includes("isGridIconOrTilesMode"),
  "FIL-58: icon/tiles must not force-disable list virtualization",
);
assert.ok(view.includes("@scroll"), "list surfaces must track scroll for the virtual window");
assert.ok(!view.includes("groupedVisibleEntries"), "grouped path must use flattened virt items");
// Q2（2026-11-27 grilling）：折叠组/切分组字段让内容总高骤缩，DOM 更新后必须
// 读回浏览器已 clamp 的 scrollTop 同步进响应式状态，否则虚拟窗口按过期深度
// 切片（一帧空白/错位）。
assert.ok(
  view.includes("watch([collapsedGroupKeys, navigatorGroupBy]"),
  "collapse/group-field switch must re-sync the clamped scrollTop after DOM update",
);
// 分组缩略图请求窗（2026-11-27 真机 bug：分组后缩略图不加载/首屏不加载）：
// 请求/取消窗必须跟分组视觉模型（桶序 + 组头高度），不能沿用扁平排序序几何。
assert.ok(
  virtItems.includes("computeGridThumbRequestWindow"),
  "grouped grid thumbnail request window",
);
assert.ok(
  view.includes("syncGroupedThumbRequests"),
  "grouped views must sync thumbnail requests through the grouped visual model",
);
assert.ok(
  view.includes("const primary = isGrouped.value") &&
    view.includes("? syncGroupedThumbRequests(primaryThumbRetainedPaths)"),
  "refreshViewportIconRequests must branch to the grouped sync when grouped",
);
assert.ok(
  view.includes("groupedThumbItemRange(THUMB_REQUEST_OVERSCAN_ROWS)"),
  "grouped request window must use the narrow overscan",
);
assert.ok(
  view.includes("groupedThumbItemRange(THUMB_REQUEST_CANCEL_OVERSCAN_ROWS)"),
  "grouped cancel window must use the wide overscan",
);
// 分组切换/折叠不动 visibleEntries——图标请求 watch 必须单独监听分组状态，
// 否则切分组后到下一次滚动前完全不重同步。
assert.ok(
  view.includes("navigatorGroupBy,") && view.includes("collapsedGroupKeys,"),
  "icon request watch must observe grouping state",
);

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/list-virtual-window-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

console.log("list-virtual checks OK");
