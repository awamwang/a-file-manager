// SPDX-License-Identifier: GPL-3.0-or-later
// FIL-45: Work Groups / Projects master-list drag reorder + persist IPC.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const wg = read("src/views/WorkGroupsView.vue");
const projects = read("src/views/ProjectsView.vue");
const helper = read("src/state/listPointerReorder.ts");
const libRs = read("src-tauri/src/lib.rs");
const wgMod = read("src-tauri/src/work_groups/mod.rs");
const wgSync = read("src-tauri/src/work_groups/sync.rs");
const projectsMod = read("src-tauri/src/projects/mod.rs");
const projectsIndex = read("src-tauri/src/projects/index.rs");
const projectsWorkspace = read("src-tauri/src/projects/workspace.rs");
const en = JSON.parse(read("src/i18n/locales/en.json"));
const zh = JSON.parse(read("src/i18n/locales/zh-CN.json"));

assert.ok(helper.includes("createListPointerReorder"));
assert.ok(helper.includes("shouldSuppressActivate"));
assert.ok(helper.includes("thresholdPx"));
assert.ok(helper.includes("clickSuppressMs"));

assert.ok(wg.includes("createListPointerReorder"));
assert.ok(wg.includes("work_groups_reorder"));
assert.ok(wg.includes("onListPointerDown"));
assert.ok(wg.includes("shouldSuppressGroupActivate"));
assert.ok(wg.includes("group--drop-target"));
assert.ok(wg.includes("group--dragging"));
assert.ok(wg.includes("workGroups.dragReorderHint"));
assert.ok(!wg.includes('@click="silentSwitch(group.id)"'));

assert.ok(projects.includes("createListPointerReorder"));
assert.ok(projects.includes("projects_workspace_reorder"));
assert.ok(projects.includes("onListPointerDown"));
assert.ok(projects.includes("shouldSuppressProjectActivate"));
assert.ok(projects.includes("group--drop-target"));
assert.ok(projects.includes("group--dragging"));
assert.ok(projects.includes("projects.dragReorderHint"));
assert.ok(!projects.includes('@click="selectProject(project.id)"'));

assert.ok(libRs.includes("work_groups::work_groups_reorder"));
assert.ok(libRs.includes("projects::projects_workspace_reorder"));
assert.ok(wgMod.includes("work_groups_reorder"));
assert.ok(wgSync.includes("reorder_work_groups_in_list"));
assert.ok(projectsMod.includes("projects_workspace_reorder"));
assert.ok(projectsIndex.includes("workspace_order"));
assert.ok(projectsIndex.includes("reorder_workspace_slugs"));
assert.ok(projectsWorkspace.includes("apply_workspace_order"));

assert.ok(en.workGroups.dragReorderHint.length > 3);
assert.ok(en.projects.dragReorderHint.length > 3);
assert.ok(zh.workGroups.dragReorderHint.includes("拖"));
assert.ok(zh.projects.dragReorderHint.includes("拖"));

console.log("wg/project reorder checks OK");
