// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-247: after creating a Work Group / Project, select/activate the new item.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const wg = readFileSync(join(root, "src/views/WorkGroupsView.vue"), "utf8");
const projects = readFileSync(join(root, "src/views/ProjectsView.vue"), "utf8");
const sync = readFileSync(join(root, "src-tauri/src/work_groups/sync.rs"), "utf8");
const wgMod = readFileSync(join(root, "src-tauri/src/work_groups/mod.rs"), "utf8");

assert.ok(sync.includes("create_and_activate_work_group"));
assert.ok(sync.includes("create_and_activate_selects_new_group"));
assert.ok(wgMod.includes("create_and_activate_work_group"));
assert.ok(wgMod.includes("live_open_state"));
assert.ok(wgMod.includes("SilentSwitchResult"));

assert.match(
  wg,
  /work_groups_create[\s\S]*liveOpenState:\s*snapshotLeavingOpenState\(\)/,
);
assert.ok(wg.includes('invoke<SilentSwitchResult>("work_groups_create"'));
assert.ok(wg.includes("restoreWorkGroupOpenState"));
assert.ok(
  wg.includes("result.document.activeWorkGroupId") ||
    wg.includes("result.document.activeWorkGroupId,"),
);

assert.match(
  projects,
  /async function createProject\(\)[\s\S]*await refresh\(slug\)[\s\S]*await selectProject\(slug\)/,
);

console.log("select-after-create (AIA-247) OK.");
