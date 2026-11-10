// SPDX-License-Identifier: GPL-3.0-or-later
/** FIL-26/27: WG→Project 升级直达新项目；不改 explorer WG / OpenState；promote IPC 异步化。 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const wg = read("src/views/WorkGroupsView.vue");
const projects = read("src/views/ProjectsView.vue");
const lifecycle = read("src-tauri/src/projects/lifecycle.rs");
const mod = read("src-tauri/src/projects/mod.rs");

// FIL-26: promote 后跳转到 projects?slug=…
assert.match(
  wg,
  /projects_workspace_promote_from_work_group[\s\S]*router\.push\(\{\s*name:\s*"projects",\s*query:\s*\{\s*slug\s*\}\s*\}\)/,
);
assert.ok(wg.includes("promoting"), "promote button busy flag");

const promoteFn = wg.match(
  /async function promoteToProject\(\) \{[\s\S]*?\n\}/,
)?.[0];
assert.ok(promoteFn, "promoteToProject function present");
assert.ok(
  !promoteFn.includes("restoreWorkGroupOpenState"),
  "promote must not restore OpenState",
);
assert.ok(
  !promoteFn.includes("snapshotLeavingOpenState"),
  "promote must not snapshot OpenState for leave",
);
assert.ok(
  !promoteFn.includes("silentSwitch") &&
    !promoteFn.includes("projects_workspace_activate") &&
    !promoteFn.includes("activeWorkGroupId.value"),
  "promote must not activate / mutate explorer WG",
);

// Projects 落地：query.slug 聚焦详情，不走 selectProject/activate
assert.ok(projects.includes("focusProjectWithoutActivate"));
assert.ok(projects.includes("route.query.slug"));
assert.match(
  projects,
  /focusProjectWithoutActivate\(focusSlug\)[\s\S]*router\.replace\(\{\s*name:\s*"projects"\s*\}\)/,
);
const focusFn = projects.match(
  /async function focusProjectWithoutActivate\([\s\S]*?\n\}/,
)?.[0];
assert.ok(focusFn, "focusProjectWithoutActivate present");
assert.ok(
  !focusFn.includes("selectProject") &&
    !focusFn.includes("projects_workspace_activate") &&
    !focusFn.includes("applyProjectSilentSwitch"),
  "focus-without-activate path must not call activate",
);
const mountBlock = projects.match(/onMounted\(\(\) => \{[\s\S]*?\n\}\);/)?.[0] ?? "";
assert.ok(mountBlock.includes("focusProjectWithoutActivate(focusSlug)"));
assert.ok(
  !mountBlock.includes("selectProject(") &&
    !mountBlock.includes("projects_workspace_activate"),
  "promote landing mount must not activate",
);

// Backend: promote 设 thin-index active，但不改 WG 文档
assert.ok(lifecycle.includes("set_index_active(index, Some(&created.project.slug))"));
assert.ok(lifecycle.includes("Does not delete or modify the WG itself / OpenState"));
assert.ok(lifecycle.includes("pub index: ProjectsIndex"));

// FIL-27: promote IPC async + spawn_blocking
assert.match(
  mod,
  /#\[tauri::command\][\s\S]*?pub async fn projects_workspace_promote_from_work_group[\s\S]*?spawn_blocking/,
);

console.log("check-wg-promote-jump: ok");
