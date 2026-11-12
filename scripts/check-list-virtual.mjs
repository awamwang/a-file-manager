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

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/list-virtual-window-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

console.log("list-virtual checks OK");
