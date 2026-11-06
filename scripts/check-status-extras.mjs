// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const rust = readFileSync(join(root, "src-tauri/src/navigator/disk_space.rs"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(shell.includes("selectionStatus"));
assert.ok(shell.includes("freeSpace"));
assert.ok(shell.includes("navigator_disk_space"));
assert.ok(rust.includes("GetDiskFreeSpaceExW"));
assert.equal(en.chrome.selectionStatus.includes("{count}"), true);

const logic = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/status-extras-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (logic.stdout) process.stdout.write(logic.stdout);
if (logic.stderr) process.stderr.write(logic.stderr);
assert.equal(logic.status, 0);

const cargo = spawnSync(
  "cargo",
  [
    "test",
    "--manifest-path",
    join(root, "src-tauri/Cargo.toml"),
    "disk_space::",
    "--",
    "--nocapture",
  ],
  { cwd: root, encoding: "utf8" },
);
if (cargo.stdout) process.stdout.write(cargo.stdout);
if (cargo.stderr) process.stderr.write(cargo.stderr);
assert.equal(cargo.status, 0);

console.log("Status extras shell checks OK (AIA-195).");
