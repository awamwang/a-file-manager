// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const registry = readFileSync(join(root, "src/shortcuts/registry.ts"), "utf8");
const rust = readFileSync(join(root, "src-tauri/src/navigator/env_path.rs"), "utf8");

assert.ok(shell.includes("navigator_expand_path"));
assert.ok(shell.includes("chrome.focusAddressAltD"));
assert.ok(shell.includes("chrome.focusAddressF4"));
assert.ok(registry.includes('id: "chrome.focusAddressAltD"'));
assert.ok(registry.includes('id: "chrome.focusAddressF4"'));
assert.ok(rust.includes("ExpandEnvironmentStringsW"));

const logic = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/address-env-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (logic.stdout) process.stdout.write(logic.stdout);
if (logic.stderr) process.stderr.write(logic.stderr);
assert.equal(logic.status, 0);

const cargo = spawnSync(
  "cargo",
  ["test", "--manifest-path", join(root, "src-tauri/Cargo.toml"), "env_path::", "--", "--nocapture"],
  { cwd: root, encoding: "utf8" },
);
if (cargo.stdout) process.stdout.write(cargo.stdout);
if (cargo.stderr) process.stderr.write(cargo.stderr);
assert.equal(cargo.status, 0);

console.log("Address env shell checks OK (AIA-196).");
