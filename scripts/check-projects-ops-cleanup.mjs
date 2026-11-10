// SPDX-License-Identifier: GPL-3.0-or-later
/** 退役未注册 legacy projects.json IPC；helpers 抽出；ops 仅测。 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const mod = read("src-tauri/src/projects/mod.rs");
const helpers = read("src-tauri/src/projects/helpers.rs");
const ops = read("src-tauri/src/projects/ops.rs");
const lib = read("src-tauri/src/lib.rs");

assert.ok(mod.includes("pub(crate) mod helpers"), "mod must declare helpers");
assert.ok(
  /#\[cfg\(test\)\]\s*\nmod ops;/.test(mod) || mod.includes("#[cfg(test)]\nmod ops;"),
  "ops must be test-only",
);

for (const dead of [
  "pub fn projects_get",
  "pub fn projects_create",
  "pub fn projects_rename",
  "pub fn projects_delete",
  "pub fn projects_set_active",
  "pub fn projects_activate",
  "pub fn projects_bind_work_group",
  "pub fn projects_promote_from_work_group",
  "pub fn projects_save_members_from_open_state",
  "pub fn projects_add_workspace_link",
  "pub fn projects_remove_workspace_link",
  "pub fn projects_set_context_notes",
  "pub fn projects_add_task",
  "pub fn projects_update_task",
  "pub fn projects_move_task",
  "pub fn projects_delete_task",
  "pub fn projects_open_all_links",
]) {
  assert.ok(!mod.includes(dead), `dead IPC must be removed: ${dead}`);
  assert.ok(!lib.includes(dead.replace("pub fn ", "")), `lib must not register ${dead}`);
}

for (const helper of [
  "pub fn now_millis",
  "pub fn next_id",
  "pub fn basename_label",
  "pub fn validate_bindable_work_group",
]) {
  assert.ok(helpers.includes(helper), `helpers missing ${helper}`);
}

assert.ok(
  ops.includes("Legacy `projects.json`"),
  "ops.rs should document legacy-only role",
);
assert.ok(
  mod.includes("projects_workspace_create"),
  "Workspace IPC must remain",
);

console.log("check-projects-ops-cleanup: ok");
