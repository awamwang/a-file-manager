// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-81: Navigator Work Group dropdown (silent switch + hard save).
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const chrome = readFileSync(join(root, "src/state/workGroupChrome.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

for (const needle of [
  "navigator-work-group-menu",
  "silentSwitchWorkGroup",
  "saveActiveOpenStateHard",
  "saveActiveMembersFromTabsHard",
  "chrome.workGroupMenu",
  "workGroups.saveOpenState",
]) {
  if (!shell.includes(needle)) {
    console.error(`AppShell missing ${needle}`);
    process.exit(1);
  }
}

for (const needle of [
  'work_groups_silent_switch',
  "work_groups_save_open_state",
  "work_groups_save_members_from_open_state",
  "restoreWorkGroupOpenState",
]) {
  if (!chrome.includes(needle)) {
    console.error(`workGroupChrome missing ${needle}`);
    process.exit(1);
  }
}

if (en.chrome.workGroupMenu !== "Work Group") {
  console.error("en.chrome.workGroupMenu mismatch");
  process.exit(1);
}

console.log("navigator-wg-chrome: ok");
