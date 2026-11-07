// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const state = readFileSync(join(root, "src/state/listPlaceholders.ts"), "utf8");

assert.ok(state.includes("estimatePlaceholderCount"));
assert.ok(state.includes("iconSlotCssSize"));
assert.ok(view.includes("estimatePlaceholderCount"));
assert.ok(view.includes("row--placeholder") || view.includes("list-placeholder"));
assert.ok(view.includes("--nav-icon-size"));
assert.ok(
  view.includes("listing || listLoading") || view.includes("listLoading || listing"),
  "list surface must mount while loading so placeholders can fill the viewport",
);
assert.ok(
  /kind.*nav-icon-size|nav-icon-size.*kind|--nav-icon-size/.test(view),
  "icon slot must reserve current view size before shell icons arrive",
);

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/list-placeholders-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

console.log("List placeholder checks OK (AIA-267).");
