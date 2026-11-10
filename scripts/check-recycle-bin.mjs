// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
const guide = readFileSync(join(root, "src/components/navigator/NavigatorGuide.vue"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const model = readFileSync(join(root, "src/components/navigator/contextMenuModel.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(browse.includes("isRecyclePath"));
assert.ok(browse.includes("loadRecycleListing"));
assert.ok(guide.includes("RECYCLE_PATH"));
assert.ok(shell.includes("openPlacesPath(RECYCLE_PATH)"), "places entry opens recycle");
assert.ok(model.includes("recycleMode"));
assert.ok(model.includes('id: "restore"'));
assert.equal(en.recycleBin.tabName, "Recycle Bin");

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/recycle-bin-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
assert.equal(result.status, 0);

console.log("Recycle Bin shell checks OK (AIA-193).");
