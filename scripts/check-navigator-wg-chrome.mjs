// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-81 / AIA-90: Navigator Work Group dropdown — silent switch; no explicit OpenState save.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const chrome = readFileSync(join(root, "src/state/workGroup/chrome.ts"), "utf8");
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
  "work_groups_save_members_from_open_state",
  "openActiveMembersIntoOpenState",
  "openMembersIntoOpenState",
  "silentSwitchWorkGroup",
]) {
  if (!chrome.includes(needle)) {
    console.error(`workGroupChrome missing ${needle}`);
    process.exit(1);
  }
}

const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
if (!browse.includes("work_groups_silent_switch")) {
  console.error("navigatorBrowse must own work_groups_silent_switch (session)");
  process.exit(1);
}
if (!browse.includes("restoreWorkGroupOpenState")) {
  console.error("navigatorBrowse missing restoreWorkGroupOpenState");
  process.exit(1);
}
if (chrome.includes('invoke<SilentSwitchResult>("work_groups_silent_switch"')) {
  console.error("workGroupChrome must not own silent_switch invoke");
  process.exit(1);
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

// AIA-147: Work Groups chip leads the navigator toolbar and is visually distinct.
{
  const chromeStart = shell.indexOf('class="navigator-chrome"');
  // Toolbar chrome follows the tab strip; end at content main (AIA-276 host wrapper).
  const chromeEnd = shell.indexOf('class="content"');
  if (chromeStart < 0 || chromeEnd <= chromeStart) {
    console.error("cannot locate navigator-chrome block");
    process.exit(1);
  }
  const chrome = shell.slice(chromeStart, chromeEnd);
  const wgIdx = chrome.indexOf("navigator-work-group-menu");
  const backIdx = Math.max(
    chrome.indexOf('t("chrome.back")'),
    chrome.indexOf("t('chrome.back')"),
  );
  if (wgIdx < 0 || backIdx < 0 || wgIdx > backIdx) {
    console.error("Work Groups menu must be first in navigator toolbar (before Back)");
    process.exit(1);
  }
  if (!chrome.includes("wg-trigger") || !chrome.includes("wg-trigger-label")) {
    console.error("Work Groups trigger must use distinct text+chevron chip (wg-trigger)");
    process.exit(1);
  }
  if (!shell.includes(".wg-trigger {") && !shell.includes(".wg-trigger{")) {
    console.error("AppShell missing .wg-trigger distinct chip styles");
    process.exit(1);
  }
}

console.log("navigator-wg-chrome: ok");
