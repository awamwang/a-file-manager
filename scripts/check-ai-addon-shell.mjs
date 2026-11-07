// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-214 / A1 + AIA-215 / A2: AI Add-on shell + Agent Target registry boundary.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const rustAi = read("src-tauri/src/ai/mod.rs");
const agents = read("src-tauri/src/ai/agents.rs");
const lib = read("src-tauri/src/lib.rs");
const sections = read("src/ai/sections.ts");
const index = read("src/ai/index.ts");
const view = read("src/views/AiView.vue");
const panel = read("src/ai/AgentTargetsPanel.vue");
const router = read("src/router/index.ts");
const sideNav = read("src/shell/sideNav.ts");
const registry = read("src/commands/registry.ts");

assert.ok(lib.includes("mod ai;"), "lib.rs must declare AI Add-on module");
assert.ok(lib.includes("ai::ai_addon_status"), "ai_addon_status must be registered");
assert.ok(lib.includes("ai::ai_agent_targets_list"), "agent list IPC");
assert.ok(lib.includes("ai::ai_agent_targets_set_enabled"), "agent enable IPC");
assert.ok(lib.includes("ai::ai_agent_targets_upsert_custom"), "agent custom IPC");
assert.ok(lib.includes("ai::ai_agent_targets_update"), "agent update IPC");
assert.ok(lib.includes("ai::ai_agent_targets_remove"), "agent remove IPC");
assert.ok(lib.includes("ai::ai_agent_targets_discover"), "agent discover IPC");

assert.ok(
  rustAi.includes("AI Add-on") && rustAi.includes("ADR 0007"),
  "Rust AI module must document Core vs Add-on boundary",
);
assert.ok(rustAi.includes('"agents"'), "agents section id");
assert.ok(agents.includes("ai-config.json"), "APPDATA AI config file");
assert.ok(agents.includes("mcp_usage_prompt"), "MCP usage prompt field");
assert.ok(
  agents.includes("placeholder") || agents.includes("Placeholder"),
  "MCP prompt must be documented as placeholder",
);
assert.ok(agents.includes("global_skills_dir"), "global skills dir");
assert.ok(agents.includes("project_skills_dir"), "project skills dir");
assert.ok(agents.includes("global_mcp_path"), "global mcp path");
assert.ok(agents.includes("project_mcp_path"), "project mcp path");
assert.ok(!agents.includes("merge_deploy") && !agents.includes("junction deploy"), "A2 must not deploy");

assert.ok(index.includes("AI_MENU_SECTIONS"), "src/ai public export");
assert.ok(sections.includes('id: "agents"'));
assert.ok(view.includes("AgentTargetsPanel"));
assert.ok(panel.includes("ai_agent_targets_list"));
assert.ok(panel.includes("mcpUsagePrompt"));
assert.ok(router.includes('path: "/ai"'));
assert.ok(sideNav.includes('id: "ai"'));
assert.ok(registry.includes('id: "view.ai"'));

const projectsMod = read("src-tauri/src/projects/mod.rs");
assert.ok(
  !projectsMod.includes("ai_agent_targets"),
  "Core projects must not host Agent Target IPC",
);

const cargo = spawnSync(
  "cargo",
  ["test", "--manifest-path", "src-tauri/Cargo.toml", "--lib", "ai::", "--", "--nocapture"],
  { cwd: root, encoding: "utf8" },
);
if (cargo.stdout) process.stdout.write(cargo.stdout);
if (cargo.stderr) process.stderr.write(cargo.stderr);
assert.equal(cargo.status, 0, "cargo test ai:: failed");

console.log("AI Add-on shell + Agent Target checks OK (AIA-214/A1 · AIA-215/A2).");
