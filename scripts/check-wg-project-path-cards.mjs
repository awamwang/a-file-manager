// SPDX-License-Identifier: GPL-3.0-or-later
// Work Groups / Projects: clickable Fluent path cards + usage hints.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const wg = readFileSync(join(root, "src/views/WorkGroupsView.vue"), "utf8");
const projects = readFileSync(join(root, "src/views/ProjectsView.vue"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));

assert.ok(wg.includes("path-card-grid"));
assert.ok(wg.includes("openOrFocusNavigatorTab"));
assert.ok(wg.includes("workGroups.usageHint"));
assert.ok(projects.includes("path-card-grid"));
assert.ok(projects.includes("openLink"));
assert.ok(projects.includes("projects.usageHint"));
assert.ok(en.workGroups.usageHint.length > 10);
assert.ok(en.projects.usageHint.length > 10);
assert.ok(zh.workGroups.usageHint.includes("标签") || zh.workGroups.usageHint.length > 8);
assert.ok(zh.projects.usageHint.includes("项目") || zh.projects.usageHint.length > 8);

console.log("wg/project path cards + usage hints OK.");
