// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-273: Work Group / Project copy — UI entry + bound WG + fresh openState.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const lib = read("src-tauri/src/lib.rs");
assert.ok(lib.includes("work_groups_copy"), "lib missing work_groups_copy");
assert.ok(lib.includes("projects_workspace_copy"), "lib missing projects_workspace_copy");

const sync = read("src-tauri/src/work_groups/sync.rs");
assert.ok(sync.includes("copy_work_group_in_list"));
assert.ok(sync.includes("clone_open_state_with_fresh_tab_ids"));
assert.ok(sync.includes("unique_copy_name"));
assert.ok(sync.includes("copy_work_group_remaps_open_state_tab_ids"));

const lifecycle = read("src-tauri/src/projects/lifecycle.rs");
assert.ok(lifecycle.includes("copy_workspace"));
assert.ok(lifecycle.includes("copy_workspace_copies_bound_wg_with_fresh_open_state"));
assert.ok(lifecycle.includes("remap_board_task_ids"));

const wgMod = read("src-tauri/src/work_groups/mod.rs");
assert.ok(wgMod.includes("work_groups_copy"));
assert.ok(wgMod.includes("copy_work_group_in_list"));

const projectsMod = read("src-tauri/src/projects/mod.rs");
assert.ok(projectsMod.includes("projects_workspace_copy"));
assert.ok(projectsMod.includes("lifecycle::copy_workspace"));

const wgView = read("src/views/WorkGroupsView.vue");
assert.ok(wgView.includes('invoke<[UserWorkGroups, WorkGroup]>("work_groups_copy"'));
assert.ok(wgView.includes("workGroups.copy"));
assert.ok(wgView.includes("copyGroup("));

const projectsView = read("src/views/ProjectsView.vue");
assert.ok(projectsView.includes('"projects_workspace_copy"'));
assert.ok(projectsView.includes("projects.copy"));
assert.ok(projectsView.includes("copyProject("));
assert.ok(projectsView.includes("copiedWithWorkGroup"));

for (const locale of ["en", "zh-CN", "ja", "de", "fr"]) {
  const json = JSON.parse(read(`src/i18n/locales/${locale}.json`));
  assert.ok(json.workGroups?.copy, `${locale} missing workGroups.copy`);
  assert.ok(json.workGroups?.copied, `${locale} missing workGroups.copied`);
  assert.ok(json.projects?.copy, `${locale} missing projects.copy`);
  assert.ok(json.projects?.copied, `${locale} missing projects.copied`);
  assert.ok(
    json.projects?.copiedWithWorkGroup,
    `${locale} missing projects.copiedWithWorkGroup`,
  );
}

console.log("wg-project-copy (AIA-273) OK.");
