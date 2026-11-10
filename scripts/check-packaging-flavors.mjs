// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-222: Core vs AI Add-on packaging flavors.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const cargo = read("src-tauri/Cargo.toml");
const lib = read("src-tauri/src/lib.rs");
const flavor = read("src/config/flavor.ts");
const sideNav = read("src/shell/sideNav.ts");
const router = read("src/router/index.ts");
const registry = read("src/commands/registry.ts");
const pkg = JSON.parse(read("package.json"));
const buildFlavor = read("scripts/build-flavor.mjs");

assert.ok(cargo.includes('default = ["ai-addon"]'), "Cargo default enables ai-addon");
assert.ok(cargo.includes("ai-addon = []"), "ai-addon feature declared");
assert.ok(lib.includes('#[cfg(feature = "ai-addon")]'), "lib gates AI module");
assert.ok(lib.includes("cfg!(feature = \"ai-addon\")") || lib.includes("ai_addon:"), "health reports flavor");
// 单一 generate_handler：AI IPC 逐条 cfg 门控，Core 构建自动剔除
assert.ok(
  /#\[cfg\(feature = "ai-addon"\)\]\s*\n\s*ai::/.test(lib),
  "AI IPC entries gated inside shared generate_handler",
);
assert.ok(
  !lib.includes('#[cfg(not(feature = "ai-addon"))]'),
  "no duplicated Core-only generate_handler branch",
);

assert.ok(flavor.includes("VITE_AFM_AI_ADDON"), "Vite flavor env");
assert.ok(sideNav.includes("AI_ADDON_ENABLED"), "side nav gated");
assert.ok(router.includes("AI_ADDON_ENABLED"), "router gated");
assert.ok(registry.includes("AI_ADDON_ENABLED"), "commands gated");

assert.equal(pkg.scripts["build:ai"], "node scripts/build-flavor.mjs ai");
assert.equal(pkg.scripts["build:core"], "node scripts/build-flavor.mjs core");
assert.ok(buildFlavor.includes("--no-default-features"), "core cargo args");

// Compile-check Core flavor (no AI module) + default AI flavor health test.
const coreCheck = spawnSync(
  "cargo",
  [
    "check",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "--lib",
    "--no-default-features",
  ],
  { cwd: root, encoding: "utf8" },
);
if (coreCheck.stdout) process.stdout.write(coreCheck.stdout);
if (coreCheck.stderr) process.stderr.write(coreCheck.stderr);
assert.equal(coreCheck.status, 0, "cargo check --no-default-features failed");

const aiTest = spawnSync(
  "cargo",
  [
    "test",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "--lib",
    "health_get_returns_ok_payload",
    "--",
    "--nocapture",
  ],
  { cwd: root, encoding: "utf8" },
);
if (aiTest.stdout) process.stdout.write(aiTest.stdout);
if (aiTest.stderr) process.stderr.write(aiTest.stderr);
assert.equal(aiTest.status, 0, "default-feature health test failed");

console.log("Packaging flavors OK (AIA-222: Core --no-default-features · AI default).");
