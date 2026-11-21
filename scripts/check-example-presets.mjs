// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Baseline gate for the one-time example presets (example Work Group + two
 * Projects) and the "rename to make it yours" guidance steps.
 *
 * Layers: STATIC here; CARGO behavior in src-tauri/src/examples.rs; the tour
 * steps/i18n keys are additionally guarded by check-help-guidance + check-i18n.
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const examples = read("src-tauri/src/examples.rs");
const lib = read("src-tauri/src/lib.rs");
const registry = read("src/help/registry.ts");
const wgView = read("src/views/WorkGroupsView.vue");
const projectsView = read("src/views/ProjectsView.vue");

// --- backend: stable presets -------------------------------------------------
assert.ok(examples.includes('pub const EXAMPLE_WORK_GROUP_ID: &str = "work-group-example-1"'));
assert.ok(examples.includes('pub const EXAMPLE_WORK_GROUP_NAME: &str = "示例工作组一"'));
assert.ok(examples.includes('pub const EXAMPLE_PROJECT_NAME_1: &str = "我的项目一"'));
assert.ok(examples.includes('pub const EXAMPLE_PROJECT_NAME_2: &str = "我的项目二"'));
// One-time marker: seed is a no-op after the marker exists (no resurrection).
assert.match(examples, /fn already_seeded\(/);
assert.match(examples, /if already_seeded\(config_dir\)[\s\S]*?return Ok\(/);
// Presets are bindable non-default WG with a home-directory tab + real projects.
assert.ok(examples.includes("is_default: false"));
assert.ok(examples.includes("home_open_state(home)"));
assert.ok(examples.includes('tab_type: "directory".to_string()'));
// Only project 1 is bound to the example WG; project 2 stays unbound so it can
// be activated independently of any work group.
assert.ok(examples.includes("bound_work_group_id: Option<String>"));
assert.ok(examples.includes("bound_work_group.map(|id| id.to_string())"));
assert.ok(examples.includes('(EXAMPLE_PROJECT_NAME_2, "example-project-2", None)'));
// Idempotent per-item creation (partial runs never duplicate).
assert.match(examples, /fn ensure_example_work_group\(/);
assert.match(examples, /fn ensure_example_project\(/);

// --- startup wiring: primary instance only, non-fatal -----------------------
assert.ok(lib.includes("mod examples;"));
assert.match(lib, /primary_instance[\s\S]{0,200}examples::seed_examples_for_app/);
assert.match(lib, /examples::seed_examples_for_app\(app\.handle\(\)\)/);

// --- guidance: rename steps appended to both tours --------------------------
assert.ok(registry.includes('anchor: "tour-workgroups-rename"'));
assert.ok(registry.includes('titleKey: "tour.workGroups.rename.title"'));
assert.ok(registry.includes('anchor: "tour-projects-rename"'));
assert.ok(registry.includes('titleKey: "tour.projects.rename.title"'));

// The Work Groups tour must demo rename on the EXAMPLE group, never the default
// one: the default title is not renameable, so highlighting it would make the
// step contradict itself.
const examplesConfig = read("src/config/examples.ts");
assert.ok(examplesConfig.includes('EXAMPLE_WORK_GROUP_ID = "work-group-example-1"'));
assert.ok(wgView.includes('import { EXAMPLE_WORK_GROUP_ID } from "../config/examples.ts"'));
assert.ok(
  wgView.includes("group.id === EXAMPLE_WORK_GROUP_ID ? 'tour-workgroups-group' : undefined"),
);
assert.ok(wgView.includes("active.isDefault ? undefined : 'tour-workgroups-rename'"));
// No static anchor may remain on the default group or its (non-renameable) title.
assert.ok(!wgView.includes('data-tour-id="tour-workgroups-group"'));
assert.ok(!wgView.includes('data-tour-id="tour-workgroups-rename"'));

assert.ok(projectsView.includes('data-tour-id="tour-projects-rename"'));

console.log("check-example-presets: ok");
