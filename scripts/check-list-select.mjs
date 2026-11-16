// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const registry = readFileSync(join(root, "src/shortcuts/registry.ts"), "utf8");
const dndPaths = readFileSync(join(root, "src/state/dnd/paths.ts"), "utf8");
const fileClipboard = readFileSync(join(root, "src/state/fileClipboard.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(view.includes("toggleSort"));
assert.ok(view.includes("onListMouseDown"));
assert.ok(view.includes("marquee"));
// FIL-71：框选按视口坐标 + 图标槽相交，不再用 offsetLeft/Top（grid 画布下会漏选）。
assert.ok(view.includes("hitTestRowsClient") || view.includes("hitTestMarqueeEntries"));
assert.ok(view.includes("getBoundingClientRect"));
assert.ok(
  !/const top = row\.offsetTop[\s\S]*offsetLeft/.test(view),
  "marquee must not hit-test via offsetTop/offsetLeft",
);
const marqueeHit = readFileSync(join(root, "src/state/marqueeHitTest.ts"), "utf8");
assert.ok(marqueeHit.includes("hitTestMarqueeEntries"));
assert.ok(marqueeHit.includes("marqueeHitMode"));
// AIA-205：AFM MIME 常量集中在 dnd/paths.ts（AFM_MIME）。
assert.ok(dndPaths.includes("application/x-afm-paths"));
assert.ok(fileClipboard.includes("transferEntries"));
assert.ok(fileClipboard.includes("file_ops_move"));
assert.ok(view.includes("selectRange"));
assert.ok(view.includes("navigator.selectAll"));
assert.ok(registry.includes('id: "navigator.selectAll"'));
assert.equal(en.navigator.colName, "Name");
assert.equal(en.navigator.colSize, "Size");

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/list-select-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

const marquee = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/marquee-hit-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (marquee.stdout) process.stdout.write(marquee.stdout);
if (marquee.stderr) process.stderr.write(marquee.stderr);
assert.equal(marquee.status, 0, "marquee hit-test checks failed");

console.log("List select shell checks OK (AIA-64).");
