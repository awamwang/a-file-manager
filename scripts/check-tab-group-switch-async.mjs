// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-225: tab-group switch must paint strip/path first; listing + disk persist deferred.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");

assert.ok(
  browse.includes("revealPathNonBlocking"),
  "navigatorBrowse must define revealPathNonBlocking",
);
assert.ok(
  browse.includes("browseEpoch"),
  "rapid switches must drop stale navigator_list via browseEpoch",
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

console.log("tab-group switch async checks OK (AIA-225).");
