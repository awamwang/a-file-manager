// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-225: tab-group switch must paint strip/path first; listing + disk persist deferred.
// AIA-259: WG silent switch paints from local cache before disk IPC; side tree deferred.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
const chrome = readFileSync(join(root, "src/state/workGroupChrome.ts"), "utf8");
const idle = readFileSync(join(root, "src/state/scheduleIdle.ts"), "utf8");
const branch = readFileSync(
  join(root, "src/components/shell/FolderTreeBranch.vue"),
  "utf8",
);
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");

assert.ok(
  browse.includes("revealPathNonBlocking"),
  "navigatorBrowse must define revealPathNonBlocking",
);
assert.ok(
  browse.includes("browseEpoch"),
  "rapid switches must drop stale navigator_list via browseEpoch",
);
assert.ok(
  browse.includes("cancelOpenStatePersist"),
  "AIA-259: must cancel debounced persist before silent switch owns the write",
);
assert.ok(
  /listing\.value\s*=\s*null/.test(
    browse.slice(
      browse.indexOf("function revealPathNonBlocking"),
      browse.indexOf("\nexport ", browse.indexOf("function revealPathNonBlocking")),
    ),
  ),
  "AIA-259: revealPathNonBlocking must clear stale listing on path change",
);

function fnBody(name) {
  const start = browse.indexOf(`export async function ${name}`);
  assert.ok(start >= 0, `missing ${name}`);
  const next = browse.indexOf("\nexport ", start + 1);
  return browse.slice(start, next > start ? next : browse.length);
}

for (const name of [
  "switchNavigatorTab",
  "switchNavigatorTabAdjacent",
  "restoreWorkGroupOpenState",
  "closeNavigatorTab",
  "openOrFocusNavigatorTab",
]) {
  const body = fnBody(name);
  assert.ok(
    body.includes("revealPathNonBlocking"),
    `${name} must reveal path without blocking on listing`,
  );
  assert.ok(
    !/await\s+navigateTo\(/.test(body),
    `${name} must not await navigateTo (blocks tab-group UI)`,
  );
  if (name === "switchNavigatorTab" || name === "switchNavigatorTabAdjacent") {
    assert.ok(
      /schedulePersist\(\s*false\s*\)/.test(body),
      `${name} must debounce OpenState persist (not immediate)`,
    );
    assert.ok(
      !/schedulePersist\(\s*true\s*\)/.test(body),
      `${name} must not force immediate persist on switch`,
    );
  }
}

const modelCheck = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/open-state-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (modelCheck.stdout) process.stdout.write(modelCheck.stdout);
if (modelCheck.stderr) process.stderr.write(modelCheck.stderr);
assert.equal(modelCheck.status, 0, "open-state-checks must pass");

// AIA-259: optimistic WG silent switch + deferred side work（编排在会话模块）。
{
  const switchStart = browse.indexOf("export async function silentSwitchWorkGroup");
  assert.ok(switchStart >= 0, "session missing silentSwitchWorkGroup");
  const switchBody = browse.slice(
    switchStart,
    browse.indexOf("\nexport ", switchStart + 1),
  );
  assert.ok(
    switchBody.includes("cancelOpenStatePersist"),
    "silentSwitchWorkGroup must cancel pending OpenState persist",
  );
  assert.ok(
    switchBody.indexOf("restoreWorkGroupOpenState") <
      switchBody.indexOf("work_groups_silent_switch"),
    "AIA-259: optimistic restore must run before awaiting silent_switch IPC",
  );
  assert.ok(
    /reveal:\s*!paintedOptimistic/.test(switchBody),
    "reconcile restore must skip duplicate list when optimistic paint succeeded",
  );
  assert.ok(
    chrome.includes("runSilentWorkGroupSwitch") ||
      chrome.includes("silentSwitchWorkGroup"),
    "chrome must wrap session silent switch for busy/error",
  );
  assert.ok(
    !chrome.includes('invoke<SilentSwitchResult>("work_groups_silent_switch"'),
    "chrome must not own silent_switch invoke",
  );
}

assert.ok(
  idle.includes("scheduleAfterPaint"),
  "scheduleIdle must export scheduleAfterPaint",
);
assert.ok(
  branch.includes("scheduleAfterPaint"),
  "FolderTreeBranch auto-expand must defer past first paint",
);
assert.ok(
  shell.includes("scheduleAfterPaint") && shell.includes("refreshFreeDisk"),
  "free-disk status IPC must defer past first paint",
);

console.log("tab-group switch async checks OK (AIA-225/259).");
