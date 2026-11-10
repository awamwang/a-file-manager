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

assert.ok(state.includes("computeListVirtualWindow"));
assert.ok(state.includes("LIST_VIRTUALIZE_MIN_ENTRIES"));
assert.ok(view.includes("listVirtualWindow") || view.includes("computeListVirtualWindow"));
assert.ok(view.includes("primaryWindowedEntries") || view.includes("windowedEntries"));
assert.ok(view.includes("virt-spacer") || view.includes("offsetBeforePx"));
assert.ok(view.includes("@scroll"), "list surfaces must track scroll for the virtual window");

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/list-virtual-window-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

console.log("list-virtual checks OK");
