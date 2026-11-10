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
const skills = read("src-tauri/src/ai/skills.rs");
const lib = read("src-tauri/src/lib.rs");
const sections = read("src/ai/sections.ts");
const index = read("src/ai/index.ts");
const view = read("src/views/AiView.vue");
const panel = read("src/ai/AgentTargetsPanel.vue");
const skillsPanel = read("src/ai/SkillsLibraryPanel.vue");
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
assert.ok(lib.includes("ai::ai_skills_library_get"), "skills library IPC");
assert.ok(lib.includes("ai::ai_skills_import"), "skills import IPC");
assert.ok(lib.includes("ai::ai_skills_remove"), "skills remove IPC");
assert.ok(lib.includes("ai::ai_skills_library_set_root"), "skills root IPC");
assert.ok(lib.includes("ai::ai_skills_deploy"), "skills deploy IPC");
assert.ok(lib.includes("ai::ai_skills_undeploy"), "skills undeploy IPC");
assert.ok(lib.includes("ai::ai_skills_search_project"), "skills project search IPC");
assert.ok(lib.includes("ai::ai_mcp_catalog_list"), "mcp catalog list IPC");
assert.ok(lib.includes("ai::ai_mcp_catalog_upsert"), "mcp catalog upsert IPC");
assert.ok(lib.includes("ai::ai_mcp_deploy"), "mcp merge deploy IPC");
assert.ok(lib.includes("ai::ai_mcp_undeploy"), "mcp undeploy IPC");
assert.ok(lib.includes("ai::ai_harness_init"), "harness init IPC");
assert.ok(lib.includes("ai::ai_providers_list"), "providers list IPC");
assert.ok(lib.includes("ai::ai_providers_upsert_compatible"), "providers compatible IPC");
assert.ok(lib.includes("ai::ai_project_open_editor"), "project open editor IPC");

assert.ok(
  rustAi.includes("AI Add-on") && rustAi.includes("ADR 0007"),
  "Rust AI module must document Core vs Add-on boundary",
);
assert.ok(rustAi.includes("mod harness"), "harness module");
assert.ok(rustAi.includes("mod providers"), "providers module");
assert.ok(rustAi.includes("mod open_editor"), "open_editor module");
assert.ok(rustAi.includes('"agents"'), "agents section id");
assert.ok(agents.includes("ai-config.json"), "APPDATA AI config file");
assert.ok(agents.includes("mcp_usage_prompt"), "MCP usage prompt field");
assert.ok(
  agents.includes("placeholder") || agents.includes("Placeholder"),
  "MCP prompt must be documented as placeholder",
);
assert.ok(agents.includes("global_skills_dir"), "global skills dir");
assert.ok(skills.includes("SKILL.md"), "SKILL.md gate");
assert.ok(skills.includes("ERR_SKILL_MANIFEST_MISSING"), "manifest missing error");
assert.ok(skills.includes(String.raw`.agents\skills`) || skills.includes(".agents"), "default library path");
const deploy = read("src-tauri/src/ai/deploy.rs");
assert.ok(deploy.includes("mklink"), "A4 Junction deploy via mklink /J");
assert.ok(deploy.includes("search_project_skills"), "project skill search");

assert.ok(index.includes("AI_MENU_SECTIONS"), "src/ai public export");
assert.ok(index.includes("aiAddonStatus"), "typed client re-exported from index");
assert.ok(sections.includes('id: "providers"'));
assert.ok(sections.includes('id: "mcp"'));
assert.ok(sections.includes('id: "skills"'));
assert.ok(sections.includes('id: "agents"'));
assert.ok(sections.includes('id: "harness"'));
assert.ok(
  rustAi.includes('"providers"') &&
    rustAi.includes('"mcp"') &&
    rustAi.includes('"skills"') &&
    rustAi.includes('"agents"') &&
    rustAi.includes('"harness"'),
  "Rust AI section ids include Providers/MCP/Skills/Agents/Harness",
);
const client = read("src/ai/client.ts");
assert.ok(client.includes('"ai_addon_status"'), "typed client: addon status");
assert.ok(client.includes('"ai_agent_targets_list"'), "typed client: agent list");
assert.ok(client.includes('"ai_skills_import"'), "typed client: skills import");
assert.ok(client.includes('"ai_skills_deploy"'), "typed client: skills deploy");
assert.ok(client.includes('"ai_skills_search_project"'), "typed client: skills search");
assert.ok(client.includes('"ai_mcp_deploy"'), "typed client: mcp deploy");
assert.ok(client.includes('"ai_harness_init"'), "typed client: harness");
assert.ok(client.includes('"ai_providers_list"'), "typed client: providers list");
assert.ok(client.includes('"ai_providers_upsert_compatible"'), "typed client: providers compatible");
assert.ok(client.includes('"ai_project_open_editor"'), "typed client: open editor");
// AIA-263: left nav + Fluent selected UI (not purple-on-white / outline-only)
assert.ok(view.includes('role="tablist"'), "AI settings left nav tablist");
assert.ok(view.includes("ai-nav-item--active"), "section nav selected class");
assert.ok(view.includes("fluent-color-fill-selected"), "Fluent selected fill on nav");
assert.ok(view.includes("data-ai-nav"), "nav items expose section ids");
assert.ok(view.includes("AgentTargetsPanel"));
assert.ok(view.includes("SkillsLibraryPanel"));
assert.ok(view.includes("aiAddonStatus"), "AiView uses typed client");
assert.ok(panel.includes("aiAgentTargetsList"));
assert.ok(panel.includes("mcpUsagePrompt"));
assert.ok(panel.includes("agents-list-item--active"), "agent row selected class");
assert.ok(panel.includes("fluent-color-fill-selected"), "agent selected uses Fluent fill");
assert.ok(skillsPanel.includes("aiSkillsImport"));
assert.ok(skillsPanel.includes("aiSkillsDeploy"));
assert.ok(skillsPanel.includes("aiSkillsSearchProject"));
assert.ok(skillsPanel.includes("skills-list-item--active"), "skill row selected class");
assert.ok(skillsPanel.includes("fluent-color-fill-selected"), "skill selected uses Fluent fill");
const mcpRs = read("src-tauri/src/ai/mcp.rs");
assert.ok(mcpRs.includes("afm__"), "managed MCP key prefix");
assert.ok(mcpRs.includes("mcpServers"), "mcpServers merge target");
assert.ok(view.includes("McpCatalogPanel"));
const mcpPanel = read("src/ai/McpCatalogPanel.vue");
assert.ok(mcpPanel.includes("aiMcpDeploy"));
assert.ok(mcpPanel.includes("mcp-list-item--active"), "mcp row selected class");
assert.ok(mcpPanel.includes("fluent-color-fill-selected"), "mcp selected uses Fluent fill");
assert.ok(view.includes("HarnessInitPanel"));
const harness = read("src-tauri/src/ai/harness.rs");
assert.ok(harness.includes("AGENTS.md"), "writes AGENTS.md");
assert.ok(harness.includes("include_sdd"), "SDD option");
assert.ok(harness.includes("ERR_HARNESS_WORKSPACE"), "workspace gate");
const harnessPanel = read("src/ai/HarnessInitPanel.vue");
assert.ok(harnessPanel.includes("aiHarnessInit"));
assert.ok(harnessPanel.includes("includeSdd"));
assert.ok(view.includes("ProvidersPanel"));
const providersRs = read("src-tauri/src/ai/providers.rs");
assert.ok(providersRs.includes("compatible"), "Compatible kind");
assert.ok(providersRs.includes("openai"), "OpenAI preset");
assert.ok(providersRs.includes("No billing"), "documents no billing panel");
const providersPanel = read("src/ai/ProvidersPanel.vue");
assert.ok(providersPanel.includes("aiProvidersList"));
assert.ok(providersPanel.includes("aiProvidersUpsertCompatible"));
assert.ok(providersPanel.includes("row--active"), "provider row selected class");
assert.ok(
  providersPanel.includes("fluent-color-fill-selected"),
  "provider selected uses Fluent fill",
);
const openEditor = read("src-tauri/src/ai/open_editor.rs");
assert.ok(openEditor.includes("CONTEXT.md"), "Qoder opens CONTEXT.md");
assert.ok(openEditor.includes("ide_open_path"), "reuses ide_launch");
const projectsView = read("src/views/ProjectsView.vue");
assert.ok(projectsView.includes("aiProjectOpenEditor"));
assert.ok(projectsView.includes("openInEditor"));
assert.ok(router.includes('path: "/ai"'));
assert.ok(sideNav.includes('id: "ai"'));
assert.ok(registry.includes('id: "view.ai"'));
const enLocale = read("src/i18n/locales/en.json");
assert.ok(enLocale.includes('"harness"'), "en locale has harness section");
assert.ok(
  !view.includes("#7c3aed") && !view.includes("purple"),
  "AI settings must not use purple-on-white accents",
);

const projectsMod = read("src-tauri/src/projects/mod.rs");
assert.ok(
  !projectsMod.includes("ai_agent_targets") &&
    !projectsMod.includes("ai_skills_") &&
    !projectsMod.includes("ai_harness_") &&
    !projectsMod.includes("ai_providers_") &&
    !projectsMod.includes("ai_project_open_editor"),
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

console.log(
  "AI Add-on checks OK (A1–A8 + AIA-263: shell · left-nav sections · selected Fluent UI · agents · skills · junction · MCP · harness · providers · open editor).",
);
