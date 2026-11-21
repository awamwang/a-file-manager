// SPDX-License-Identifier: GPL-3.0-or-later
// W2 baseline: "Add to Work Group" from the navigator — the folder-path reduction is
// a pure, unit-tested reducer (src/components/navigator/workGroupPaths.ts). This gate
// pins the contract AND spawns the behavioral unit, so a refactor that drops the
// extraction or changes the multi-select/files-skipped semantics fails `npm test`.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pure = readFileSync(
  join(root, "src/components/navigator/workGroupPaths.ts"),
  "utf8",
);
const menu = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);

// Pure reducer exists and is genuinely pure: no reactivity / invoke inside the module.
assert.ok(
  pure.includes("export function folderPathsForWorkGroup"),
  "folderPathsForWorkGroup must be an exported pure function",
);
assert.ok(!pure.includes("invoke("), "the reducer must not call invoke (stays pure)");
assert.ok(!pure.includes(".value"), "the reducer must not read refs (stays pure)");
// Core semantics are present: non-dir → [], multi-select → selected dirs, fallback to entry.
assert.ok(
  pure.includes('entry.kind !== "dir"') && pure.includes("return []"),
  "non-directory entries must reduce to no paths",
);
assert.ok(
  pure.includes("selectedPaths.length > 1") && pure.includes('row.kind === "dir"'),
  "multi-select must expand to the selected directories only",
);

// The navigator menu delegates to the pure reducer (no logic forked back inline).
assert.ok(
  menu.includes("./workGroupPaths") &&
    menu.includes("reduceFolderPathsForWorkGroup(entry, selectedPaths.value, listing.value?.entries)"),
  "NavigatorContextMenu must delegate folderPathsForWorkGroup to the pure reducer",
);

const unit = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/work-group-paths-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (unit.stdout) process.stdout.write(unit.stdout);
if (unit.stderr) process.stderr.write(unit.stderr);
assert.equal(unit.status, 0, "work-group-paths behavior checks failed");

console.log("work-group-paths (W2) OK: pure reducer + files-skipped multi-select delegated from the menu.");
