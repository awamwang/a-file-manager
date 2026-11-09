// SPDX-License-Identifier: GPL-3.0-or-later
// Folder context menu “Add as skill” → AI public Skill library (ai_skills_import).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const model = readFileSync(join(root, "src/components/navigator/contextMenuModel.ts"), "utf8");
const menu = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));

assert.ok(model.includes("addAsSkill"));
assert.ok(model.includes("buildAddAsSkillItems"));
assert.match(model, /Folder-only: import into AFM public Skill library/);
assert.ok(menu.includes("runAddAsSkill"));
assert.ok(menu.includes("ai_skills_import"));
assert.ok(menu.includes("AI_SKILL_EXISTS"));
assert.ok(menu.includes('case "addAsSkill"'));
assert.equal(en.contextMenu.addAsSkill, "Add as skill");
assert.equal(zh.contextMenu.addAsSkill, "添加为 skill");
assert.ok(en.contextMenu.addedAsSkill.includes("{name}"));
assert.ok(zh.contextMenu.addedAsSkill.includes("{name}"));
assert.ok(en.contextMenu.addAsSkillExists.includes("{name}"));
assert.ok(zh.contextMenu.addAsSkillExists.includes("{name}"));

const checks = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/context-menu-model-checks.ts")],
  { encoding: "utf8" },
);
assert.equal(checks.status, 0, checks.stderr || checks.stdout);

console.log("add-as-skill menu OK.");
