// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-248: context menu “Add to Work Group” submenu.
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

assert.ok(model.includes("addToWorkGroup"));
assert.ok(model.includes("workGroups?"));
assert.match(model, /Default WG is included/);
assert.ok(menu.includes("runAddToWorkGroup"));
assert.ok(menu.includes("work_groups_add_member"));
assert.ok(menu.includes("folderPathsForWorkGroup"));
assert.ok(menu.includes("refreshWorkGroupsForMenu"));
assert.equal(en.contextMenu.addToWorkGroup, "Add to Work Group");
assert.equal(zh.contextMenu.addToWorkGroup, "添加到工作组");
assert.ok(en.contextMenu.addedToWorkGroup.includes("{name}"));
assert.ok(zh.contextMenu.addedToWorkGroup.includes("{count}"));

const checks = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/context-menu-model-checks.ts")],
  { encoding: "utf8" },
);
assert.equal(checks.status, 0, checks.stderr || checks.stdout);

console.log("add-to-work-group menu (AIA-248) OK.");
