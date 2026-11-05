// SPDX-License-Identifier: GPL-3.0-or-later
// Navigator Open State / multi-tab pure model smoke.
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const runner = join(root, "scripts", "open-state-checks.ts");

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", runner],
  { cwd: root, encoding: "utf8" },
);

if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
if (shell.includes("newTabLater") && shell.includes("disabled")) {
  // Still a stub somewhere — fail if New tab remains disabled with later title.
  if (/tab--add[\s\S]*?disabled[\s\S]*?newTabLater/.test(shell) || /disabled[\s\S]*?newTabLater/.test(shell)) {
    console.error("AppShell still disables New tab stub");
    process.exit(1);
  }
}
if (!shell.includes("onNewNavigatorTab") || !shell.includes("onCloseNavigatorTab")) {
  console.error("AppShell missing New/Close tab wiring");
  process.exit(1);
}
if (!shell.includes("bootstrapNavigatorOpenState")) {
  console.error("AppShell missing Open State bootstrap");
  process.exit(1);
}

const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
for (const token of [
  "work_groups_save_open_state",
  "restoreWorkGroupOpenState",
  "bootstrapNavigatorOpenState",
  "newNavigatorTab",
  "closeNavigatorTab",
  "switchNavigatorTab",
  "switchNavigatorTabAdjacent",
  "openOrFocusNavigatorTab",
  "openMembersIntoOpenState",
]) {
  if (!browse.includes(token)) {
    console.error(`navigatorBrowse missing ${token}`);
    process.exit(1);
  }
}

// Directory open entries must open-or-focus (new tab), not replace via navigateTo.
if (!/openEntry[\s\S]*?openOrFocusNavigatorTab/.test(browse)) {
  console.error("openEntry must openOrFocusNavigatorTab for directories");
  process.exit(1);
}

const discovery = readFileSync(join(root, "src/views/DiscoveryView.vue"), "utf8");
if (!discovery.includes("openOrFocusNavigatorTab") || /openHit[\s\S]*?navigateTo\(/.test(discovery)) {
  console.error("Discovery openHit must use openOrFocusNavigatorTab");
  process.exit(1);
}

if (
  !shell.includes("openOrFocusNavigatorTab") ||
  !/openPlacesPath[\s\S]*?openOrFocusNavigatorTab/.test(shell) ||
  !/goHome[\s\S]*?openOrFocusNavigatorTab/.test(shell)
) {
  console.error("AppShell places/home must openOrFocusNavigatorTab");
  process.exit(1);
}

if (
  !shell.includes("switchNavigatorTabAdjacent") ||
  !shell.includes("navigator.nextTabGroup") ||
  !shell.includes("navigator.prevTabGroup")
) {
  console.error("AppShell missing page-group adjacent switch wiring");
  process.exit(1);
}

const model = readFileSync(join(root, "src/state/openStateModel.ts"), "utf8");
for (const token of [
  "openStateAfterSwitchAdjacent",
  "openStateOpenOrFocusPath",
  "openStateEnsureMemberPaths",
  "findTabGroupIndexByPath",
]) {
  if (!model.includes(token)) {
    console.error(`openStateModel missing ${token}`);
    process.exit(1);
  }
}

const wg = readFileSync(join(root, "src/views/WorkGroupsView.vue"), "utf8");
if (
  !wg.includes("restoreWorkGroupOpenState") ||
  !wg.includes("snapshotLiveOpenState") ||
  !wg.includes("openMembersIntoOpenState") ||
  !wg.includes("openMembersIntoRuntime")
) {
  console.error("WorkGroupsView not wired to shared Open State / open-members");
  process.exit(1);
}

const projects = readFileSync(join(root, "src/views/ProjectsView.vue"), "utf8");
if (
  !projects.includes("openMembersIntoOpenState") ||
  !projects.includes("openBoundMembersIntoRuntime")
) {
  console.error("ProjectsView missing open-members into Open State");
  process.exit(1);
}

console.log("check-open-state: wiring ok");
