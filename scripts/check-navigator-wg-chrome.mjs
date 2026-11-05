// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-81 / AIA-90: Navigator Work Group dropdown — silent switch; no explicit OpenState save.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const chrome = readFileSync(join(root, "src/state/workGroupChrome.ts"), "utf8");
const wgView = readFileSync(join(root, "src/views/WorkGroupsView.vue"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

for (const needle of [
  "navigator-work-group-menu",
  "silentSwitchWorkGroup",
  "saveActiveMembersFromTabsHard",
  "openActiveMembersIntoOpenState",
  "onOpenMembersIntoOpenState",
  "chrome.workGroupMenu",
]) {
  if (!shell.includes(needle)) {
    console.error(`AppShell missing ${needle}`);
    process.exit(1);
  }
}

// AIA-90: OpenState is silent on switch — no explicit save affordance in chrome / WG page.
for (const [label, text] of [
  ["AppShell", shell],
  ["WorkGroupsView", wgView],
]) {
  if (text.includes("workGroups.saveOpenState") || text.includes("saveOpenState")) {
    console.error(`${label} still exposes explicit OpenState save UI`);
    process.exit(1);
  }
}
if (chrome.includes("saveActiveOpenStateHard") || chrome.includes("work_groups_save_open_state")) {
  console.error("workGroupChrome must not wrap explicit OpenState save (silent via silent_switch / browse persist)");
  process.exit(1);
}

for (const needle of [
  "work_groups_silent_switch",
  "work_groups_save_members_from_open_state",
  "restoreWorkGroupOpenState",
  "openActiveMembersIntoOpenState",
  "openMembersIntoOpenState",
]) {
  if (!chrome.includes(needle)) {
    console.error(`workGroupChrome missing ${needle}`);
    process.exit(1);
  }
}

if (!wgView.includes("openMembersIntoRuntime") || !wgView.includes("openMembersIntoOpenState")) {
  console.error("WorkGroupsView missing open-members into Open State");
  process.exit(1);
}

if (!en.workGroups?.openMembersIntoOpenState) {
  console.error("en.workGroups.openMembersIntoOpenState missing");
  process.exit(1);
}

if (en.chrome.workGroupMenu !== "Work Group") {
  console.error("en.chrome.workGroupMenu mismatch");
  process.exit(1);
}

console.log("navigator-wg-chrome: ok");
