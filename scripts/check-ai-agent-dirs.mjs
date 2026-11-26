// SPDX-License-Identifier: GPL-3.0-or-later
// ADR 0028: Agent Target sessions/rules path slots + AI directory jump layer.
// Backend schema v3 + open commands, FE wiring, and six-locale i18n parity.
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const agentsRs = read("src-tauri/src/ai/agents.rs");
const modRs = read("src-tauri/src/ai/mod.rs");
const projectAiRs = read("src-tauri/src/ai/project_ai.rs");
const openWithRs = read("src-tauri/src/open_with/mod.rs");
const libRs = read("src-tauri/src/lib.rs");
const client = read("src/ai/client.ts");
const types = read("src/ai/types.ts");
const panel = read("src/ai/AgentTargetsPanel.vue");
const projectPanel = read("src/ai/ProjectAiPanel.vue");

// --- Backend: schema v4 + extra path slots (ADR 0028 + macOS display normalization). ---
assert.ok(agentsRs.includes("pub const AI_CONFIG_SCHEMA_VERSION: u32 = 4"), "schema v4");
assert.ok(
  agentsRs.includes("migrate_display_paths") && agentsRs.includes("fn display_path"),
  "one-shot v3→v4 migration normalizes Windows preset paths on Unix hosts",
);
assert.ok(agentsRs.includes("BUILTIN_EXTRA_PATHS"), "builtin extras table separate from defs");
assert.ok(agentsRs.includes("global_sessions_dir"), "AgentTarget.global_sessions_dir");
assert.ok(agentsRs.includes("global_rules_path"), "AgentTarget.global_rules_path");
assert.ok(agentsRs.includes("project_rules_dir"), "AgentTarget.project_rules_dir");
assert.ok(agentsRs.includes("migrate_extra_paths"), "one-shot v2→v3 migration seeding");
assert.ok(
  agentsRs.includes("pub fn resolve_global_slot_path"),
  "slot resolution with token expansion",
);
for (const code of [
  "ERR_AGENT_SLOT_UNKNOWN",
  "ERR_AGENT_PATH_EMPTY",
  "ERR_AGENT_PATH_NOT_FOUND",
]) {
  assert.ok(agentsRs.includes(`pub const ${code}`), `error code ${code}`);
}
// Migration tests: user-cleared values must not be re-seeded.
assert.ok(
  agentsRs.includes("fn migrates_v2_seeds_extra_paths_once_and_keeps_user_values"),
  "v2→v3 migration cargo test",
);
assert.ok(
  agentsRs.includes("fn resolves_global_slot_paths_with_expansion_and_errors"),
  "slot resolution cargo test",
);

// --- Backend: read-only project AI dir detection (no load_or_init side effect). ---
assert.ok(projectAiRs.includes("pub struct ProjectAiDir"), "ProjectAiDir model");
assert.ok(projectAiRs.includes("pub fn project_ai_dirs"), "project_ai_dirs detection");
assert.ok(
  projectAiRs.includes("enabled_agent_ids(workspace, central)"),
  "dir detection is read-only (enabled_agent_ids, not load_or_init)",
);
assert.ok(
  projectAiRs.includes("fn project_ai_dirs_lists_existing_dirs_readonly"),
  "project_ai_dirs cargo test",
);

// --- Backend: open pathway reuses the shared open_with default-opener. ---
assert.ok(
  openWithRs.includes("pub(crate) fn open_existing_path"),
  "open_with exposes shared open_existing_path",
);

// --- IPC: command names registered and FE↔BE name parity. ---
for (const cmd of ["ai_agent_target_open_path", "ai_project_ai_dirs"]) {
  assert.ok(modRs.includes(`pub async fn ${cmd}`) || modRs.includes(`pub fn ${cmd}`), `command ${cmd}`);
  assert.ok(libRs.includes(`ai::${cmd},`), `lib.rs registers ${cmd}`);
  assert.ok(client.includes(`"${cmd}"`), `typed client invokes ${cmd}`);
}

// --- FE: types + panels. ---
assert.ok(types.includes("globalSessionsDir"), "AgentTarget.globalSessionsDir type");
assert.ok(types.includes("globalRulesPath"), "AgentTarget.globalRulesPath type");
assert.ok(types.includes("projectRulesDir"), "AgentTarget.projectRulesDir type");
assert.ok(types.includes("AgentGlobalPathSlot"), "slot union type");
assert.ok(types.includes("ProjectAiDir"), "ProjectAiDir type");
assert.ok(panel.includes("aiAgentTargetOpenPath"), "panel opens slot via typed client");
assert.ok(panel.includes("openSlot('globalSkillsDir')"), "open skills slot wired");
assert.ok(panel.includes("openSlot('globalSessionsDir')"), "open sessions slot wired");
assert.ok(panel.includes("openSlot('globalRulesPath')"), "open rules slot wired");
assert.ok(panel.includes("openSlot('globalMcpPath')"), "open mcp slot wired");
assert.ok(panel.includes("AI_AGENT_PATH_NOT_FOUND"), "path errors mapped to i18n");
assert.ok(projectPanel.includes("aiProjectAiDirs"), "project panel lists AI dirs");
assert.ok(projectPanel.includes("openAiDir"), "project panel opens AI dir");
assert.ok(projectPanel.includes('"open_path_default"'), "project dir open uses shared opener");

// --- ADR + coverage ledger. ---
assert.ok(
  existsSync(join(root, "docs/adr/0028-agent-target-path-slots-and-dir-jump.md")),
  "ADR 0028 exists",
);
const adr = read("docs/adr/0028-agent-target-path-slots-and-dir-jump.md");
assert.ok(adr.includes("sessions") && adr.includes("rules"), "ADR 0028 scope");
const ledger = read("docs/external-behavior-baseline-coverage.md");
assert.ok(ledger.includes("check-ai-agent-dirs"), "coverage ledger row");

// --- i18n: six locales carry the new keys. ---
const locales = ["en", "zh-CN", "zh-TW", "ja", "de", "fr"];
const agentKeys = [
  "fieldGlobalSessions",
  "fieldGlobalRules",
  "fieldProjectRules",
  "openGlobalSkills",
  "openGlobalSessions",
  "openGlobalRules",
  "openGlobalMcp",
  "noticeOpened",
  "errorSlotUnknown",
  "errorPathEmpty",
  "errorPathNotFound",
];
const projectKeys = ["dirsTitle", "dirsHint", "noticeDirOpened", "errorDirOpen"];
for (const locale of locales) {
  const doc = JSON.parse(readFileSync(join(root, `src/i18n/locales/${locale}.json`), "utf8"));
  for (const key of agentKeys) {
    assert.equal(typeof doc.ai.agents[key], "string", `${locale}.ai.agents.${key}`);
    assert.ok(doc.ai.agents[key].length > 0, `${locale}.ai.agents.${key} empty`);
  }
  for (const key of projectKeys) {
    assert.equal(typeof doc.ai.project[key], "string", `${locale}.ai.project.${key}`);
    assert.ok(doc.ai.project[key].length > 0, `${locale}.ai.project.${key} empty`);
  }
}

console.log("check-ai-agent-dirs: ok");
