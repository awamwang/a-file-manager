// SPDX-License-Identifier: GPL-3.0-or-later
/** Feature-fill: WG CRUD close + F5/new file wiring. */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const lib = read("src-tauri/src/lib.rs");
for (const cmd of [
  "work_groups_remove_member",
  "work_groups_rename",
  "work_groups_delete",
  "file_ops_create_file",
]) {
  assert.ok(lib.includes(cmd), `lib missing ${cmd}`);
}

const wg = read("src/views/WorkGroupsView.vue");
assert.ok(wg.includes("work_groups_remove_member"));
assert.ok(wg.includes("work_groups_rename"));
assert.ok(wg.includes("work_groups_delete"));
assert.ok(wg.includes("removeMember"));

const shortcuts = read("src/shortcuts/registry.ts");
assert.ok(shortcuts.includes("navigator.refresh"));
assert.ok(shortcuts.includes("navigator.newFile"));

const nav = read("src/views/NavigatorView.vue");
assert.ok(nav.includes("navigator.refresh"));
assert.ok(nav.includes("shortcutNewFile"));

console.log("feature-fill: ok");
