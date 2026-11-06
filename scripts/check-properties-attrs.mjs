// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const panel = readFileSync(
  join(root, "src/components/navigator/PropertiesPanel.vue"),
  "utf8",
);
const rust = readFileSync(join(root, "src-tauri/src/navigator/attrs.rs"), "utf8");
const lib = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const stat = readFileSync(join(root, "src-tauri/src/navigator/stat.rs"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(panel.includes("navigator_set_attributes"));
assert.ok(panel.includes("draftReadonly"));
assert.ok(panel.includes("draftHidden"));
assert.ok(panel.includes("applyAttributes"));
assert.ok(rust.includes("SetFileAttributesW"));
assert.ok(rust.includes("merge_attribute_bits"));
assert.ok(lib.includes("navigator_set_attributes"));
assert.ok(stat.includes("hidden:"));
assert.equal(typeof en.properties.hidden, "string");
assert.equal(typeof en.properties.applyAttributes, "string");

const cargo = spawnSync(
  "cargo",
  [
    "test",
    "--manifest-path",
    join(root, "src-tauri/Cargo.toml"),
    "attrs::",
    "--",
    "--nocapture",
  ],
  { cwd: root, encoding: "utf8" },
);
if (cargo.stdout) process.stdout.write(cargo.stdout);
if (cargo.stderr) process.stderr.write(cargo.stderr);
assert.equal(cargo.status, 0);

console.log("Properties attributes shell checks OK (AIA-198).");
