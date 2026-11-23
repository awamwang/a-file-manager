// SPDX-License-Identifier: GPL-3.0-or-later
// Guard for multi-agent MCP/Skill harvest: FE↔BE command parity, the pure merge
// module being actually used by both panels, and headless behavior checks.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const client = read("src/ai/client.ts");
const lib = read("src-tauri/src/lib.rs");
const mcpPanel = read("src/ai/McpCatalogPanel.vue");
const skillsPanel = read("src/ai/SkillsLibraryPanel.vue");
const dedupe = read("src/ai/harvestDedupe.ts");

// FE↔BE name parity: renaming any harvest command is a silent break.
for (const name of [
  "ai_harvest_mcp_scan",
  "ai_harvest_mcp_import",
  "ai_harvest_skills_scan",
  "ai_harvest_skills_import",
]) {
  assert.ok(client.includes(`"${name}"`), `client.ts missing command ${name}`);
  assert.ok(lib.includes(name), `lib.rs missing registration ${name}`);
}

// Multi-agent selection must be live in both panels, not just in the helper.
for (const [label, source] of [
  ["MCP", mcpPanel],
  ["Skills", skillsPanel],
]) {
  assert.ok(source.includes("harvestAgentIds"), `${label} panel missing multi-agent selection`);
  assert.ok(source.includes("source: \"agent\""), `${label} panel missing agent source segment`);
  assert.ok(source.includes("source: \"file\""), `${label} panel missing file source segment`);
  assert.ok(source.includes("duplicateReason"), `${label} panel must explain duplicate reasons`);
  assert.ok(source.includes("scanHarvest"), `${label} panel missing scan action`);
}

assert.ok(mcpPanel.includes("mergeMcpCandidates"), "MCP panel must use the merge helper");
assert.ok(skillsPanel.includes("mergeSkillCandidates"), "Skills panel must use the merge helper");
assert.ok(dedupe.includes("export function serverFingerprint"));
assert.ok(dedupe.includes("export function mergeMcpCandidates"));
assert.ok(dedupe.includes("export function mergeSkillCandidates"));

// One accent per task region: the tools area may not use three blue buttons.
assert.ok(
  !mcpPanel.includes("fluent-control--accent\" @click=\"scanHarvest"),
  "scan is a subordinate action, not a primary button",
);
assert.ok(
  !skillsPanel.includes("fluent-control--accent\" @click=\"saveRoot"),
  "save-root is a setting, not a primary button",
);

const unit = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/harvest-dedupe-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (unit.stdout) process.stdout.write(unit.stdout);
if (unit.stderr) process.stderr.write(unit.stderr);
assert.equal(unit.status, 0, "harvest-dedupe behavior checks failed");

console.log("harvest-dedupe checks OK (multi-agent import, dedupe reasons, command parity).");
