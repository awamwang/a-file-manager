// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Work Groups — "当前打开的标签" 行内 ↑：把单个打开的标签追加为固定成员。
 *
 * 锁死的契约：
 *  • 行内向上箭头按钮存在，且只在普通文件夹标签上出现；
 *  • 已是成员的标签行按钮禁用并说明原因（不重复添加）；
 *  • 动作是**追加**单个成员，不覆盖整组成员列表（覆盖式是底部
 *    "打开标签 → 成员配置" 的既有语义，两者必须并存且不互相污染）；
 *  • FE↔BE 命令名一致（work_groups_add_member）。
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const wg = read("src/views/WorkGroupsView.vue");
const cards = read("src/styles/path-cards.css");
const mod = read("src-tauri/src/work_groups/mod.rs");
const sync = read("src-tauri/src/work_groups/sync.rs");
const localeIds = ["en", "zh-CN", "zh-TW", "ja", "de", "fr"];
const catalogs = Object.fromEntries(
  localeIds.map((id) => [id, JSON.parse(read(`src/i18n/locales/${id}.json`))]),
);

// ---- 行内 ↑ 按钮 -------------------------------------------------------
assert.ok(wg.includes("path-card-promote"), "open-tab card carries the promote button");
assert.ok(wg.includes('name="arrowUp"'), "promote button uses the up arrow glyph");
assert.ok(wg.includes('v-if="canBeMember(tab)"'), "only folder tabs can be promoted");
assert.ok(
  wg.includes('@click.stop="addTabToMembers(tab)"'),
  "promote button wires the tab action",
);
assert.ok(
  wg.includes(':disabled="isHardMember(tab.path)"'),
  "tabs that are already members disable the button",
);
assert.ok(
  /workGroups\.addTabToMembers/.test(wg) &&
    /workGroups\.alreadyMember/.test(wg) &&
    /workGroups\.addedTabToMembers/.test(wg),
  "action tooltip, disabled hint and confirmation are wired",
);
assert.ok(wg.includes("isHardMember"), "member membership is derived, not guessed");
assert.ok(wg.includes("canBeMember"), "non-folder tabs are excluded from members");

// ---- 动作语义：追加单个成员 --------------------------------------------
const action = wg.match(/async function addTabToMembers\([\s\S]*?\n\}/)?.[0];
assert.ok(action, "addTabToMembers action present");
assert.ok(
  action.includes('"work_groups_add_member"'),
  "action uses the additive add_member IPC",
);
assert.ok(
  !action.includes("work_groups_save_members_from_open_state"),
  "promoting one tab must not overwrite the whole member list",
);
assert.ok(action.includes("memberPath: path"), "the clicked tab path is the member");
assert.ok(action.includes("toastSuccess"), "promotion is confirmed to the user");

// ---- FE↔BE 命令名一致 + 后端追加语义 ----------------------------------
assert.ok(
  mod.includes("pub fn work_groups_add_member"),
  "backend command name must match the invoke name",
);
assert.ok(
  sync.includes("members.push(member_path.to_string())") &&
    sync.includes("unique_member_paths(&members)"),
  "backend appends (and de-dupes) instead of replacing the member list",
);

// ---- 样式 ---------------------------------------------------------------
assert.ok(cards.includes(".path-card-promote"), "promote button has its own card style");
assert.ok(
  /\.path-card-promote:disabled\s*\{/.test(cards),
  "disabled promote button is styled",
);

// ---- i18n 六个语种都要有这三个 key -------------------------------------
for (const id of localeIds) {
  const group = catalogs[id].workGroups;
  for (const key of ["addTabToMembers", "addedTabToMembers", "alreadyMember"]) {
    assert.equal(
      typeof group[key],
      "string",
      `${id} missing workGroups.${key}`,
    );
    assert.ok(group[key].trim().length > 0, `${id} empty workGroups.${key}`);
  }
}

console.log("check-wg-tab-to-members: ok");
