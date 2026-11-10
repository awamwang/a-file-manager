// SPDX-License-Identifier: GPL-3.0-or-later
/** Grill #1：Navigator 会话行为钉 + 编排归属接线守卫。 */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");
const registerUrl = pathToFileURL(
  join(root, "scripts/register-ts-resolve.mjs"),
).href;

const result = spawnSync(
  process.execPath,
  [
    "--import",
    registerUrl,
    "--experimental-strip-types",
    join(root, "scripts/navigator-session-checks.ts"),
  ],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0, "navigator-session-checks failed");

const browse = read("src/state/navigatorBrowse.ts");
const chrome = read("src/state/workGroupChrome.ts");
const shell = read("src/components/shell/AppShell.vue");

for (const token of [
  "export async function silentSwitchWorkGroup",
  "export async function restoreWorkGroupOpenState",
  "export async function navigateTo",
  "export async function newNavigatorTab",
  "export async function closeNavigatorTab",
  "export async function switchNavigatorTab",
  "export async function openOrFocusNavigatorTab",
  "export async function applyProjectSilentSwitch",
]) {
  assert.ok(browse.includes(token), `navigatorBrowse missing ${token}`);
}

// 静默切换编排下沉到会话；chrome 只再导出 / 管 busy·error·notice
assert.ok(
  browse.includes("work_groups_silent_switch"),
  "session must own work_groups_silent_switch invoke",
);
assert.ok(
  !chrome.includes('invoke<SilentSwitchResult>("work_groups_silent_switch"'),
  "workGroupChrome must not own silent_switch invoke",
);
assert.ok(
  chrome.includes("silentSwitchWorkGroup"),
  "chrome may re-export or wrap silentSwitchWorkGroup",
);

assert.ok(
  /openPlacesPath[\s\S]*?openOrFocusNavigatorTab/.test(shell),
  "AppShell places must open via session openOrFocusNavigatorTab",
);
assert.ok(
  shell.includes("onNewNavigatorTab") && shell.includes("newNavigatorTab"),
  "AppShell new tab must call session",
);
assert.ok(
  shell.includes("closeNavigatorTab") && shell.includes("switchNavigatorTab"),
  "AppShell tab strip must call session",
);

console.log("check-navigator-session: ok");
