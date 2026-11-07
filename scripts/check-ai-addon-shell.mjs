// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-214 / A1: AI Add-on main-menu shell + Core vs AI module boundary.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const rustAi = read("src-tauri/src/ai/mod.rs");
const lib = read("src-tauri/src/lib.rs");
const sections = read("src/ai/sections.ts");
const index = read("src/ai/index.ts");
const view = read("src/views/AiView.vue");
const router = read("src/router/index.ts");
const sideNav = read("src/shell/sideNav.ts");
const registry = read("src/commands/registry.ts");

assert.ok(lib.includes("mod ai;"), "lib.rs must declare AI Add-on module");
assert.ok(lib.includes("ai::ai_addon_status"), "ai_addon_status must be registered");
assert.ok(
  rustAi.includes("AI Add-on") && rustAi.includes("ADR 0007"),
  "Rust AI module must document Core vs Add-on boundary",
);
assert.ok(rustAi.includes('"agents"'), "agents section id");
assert.ok(rustAi.includes('"mcp"'), "mcp section id");
assert.ok(rustAi.includes('"skills"'), "skills section id");
assert.ok(rustAi.includes('"providers"'), "providers section id");
assert.ok(
  rustAi.includes("No Agent registry"),
  "A1 must explicitly defer Agent registry to A2+",
);

assert.ok(index.includes("AI_MENU_SECTIONS"), "src/ai public export");
assert.ok(sections.includes('id: "agents"'));
assert.ok(sections.includes('id: "mcp"'));
assert.ok(sections.includes('id: "skills"'));
assert.ok(sections.includes('id: "providers"'));

assert.ok(view.includes('from "../ai"'), "AiView must import Add-on module");
assert.ok(view.includes("ai_addon_status"));
assert.ok(view.includes('data-ai-section'));
assert.ok(router.includes('path: "/ai"'));
assert.ok(sideNav.includes('id: "ai"'));
assert.ok(registry.includes('id: "view.ai"'));

// Core projects module must stay free of AI Add-on ownership comments claiming Agents/MCP.
const projectsMod = read("src-tauri/src/projects/mod.rs");
assert.ok(
  !projectsMod.includes("ai_addon_status"),
  "Core projects must not host AI Add-on IPC",
);

const cargo = spawnSync(
  "cargo",
  ["test", "--manifest-path", "src-tauri/Cargo.toml", "--lib", "ai::", "--", "--nocapture"],
  { cwd: root, encoding: "utf8" },
);
if (cargo.stdout) process.stdout.write(cargo.stdout);
if (cargo.stderr) process.stderr.write(cargo.stderr);
assert.equal(cargo.status, 0, "cargo test ai:: failed");

console.log("AI Add-on shell checks OK (AIA-214 / A1).");
