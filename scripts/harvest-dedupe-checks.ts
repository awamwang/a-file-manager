// SPDX-License-Identifier: GPL-3.0-or-later
// Behavioral unit for multi-agent harvest merging and duplicate explanation.
// Pinned by scripts/check-harvest-dedupe.mjs.
import assert from "node:assert/strict";
import type {
  HarvestMcpScanReport,
  HarvestSkillScanReport,
  McpCatalogEntry,
  LibrarySkill,
} from "../src/ai/types.ts";
import {
  mergeMcpCandidates,
  mergeSkillCandidates,
  normalizeMcpKey,
  serverFingerprint,
  toMcpCandidate,
  toSkillCandidate,
} from "../src/ai/harvestDedupe.ts";

assert.equal(normalizeMcpKey("  Context7 "), "context7");
assert.equal(normalizeMcpKey("@Scope/Context7"), "context7");
assert.equal(normalizeMcpKey("@scope/context7-mcp"), "context7-mcp");
assert.equal(normalizeMcpKey(""), "");

assert.equal(
  serverFingerprint('{"command":"npx","args":["-y","a"]}'),
  serverFingerprint('{"args":["-y","a"], "command":"npx"}'),
  "key order must not change the server identity",
);
assert.notEqual(
  serverFingerprint('{"command":"npx"}'),
  serverFingerprint('{"command":"node"}'),
);

const serverJson = '{"command":"npx","args":["-y","@ctx/mcp"]}';
const reportA: HarvestMcpScanReport = {
  agentId: "claude-code",
  sourcePath: "C:/a/mcp.json",
  candidates: [
    {
      id: "context7",
      displayName: "context7",
      serverJson,
      alreadyInCatalog: false,
      sourceAgentId: "claude-code",
    },
  ],
};
const reportB: HarvestMcpScanReport = {
  agentId: "cursor",
  sourcePath: "C:/b/mcp.json",
  candidates: [
    {
      id: "@scope/context7",
      displayName: "@scope/context7",
      serverJson,
      alreadyInCatalog: false,
      sourceAgentId: "cursor",
    },
  ],
};

const merged = mergeMcpCandidates([reportA, reportB], []);
assert.equal(merged.length, 1, "same server from two agents must collapse to one row");
assert.deepEqual([...merged[0].sources].sort(), ["claude-code", "cursor"]);
assert.equal(merged[0].duplicateReason, "none");
assert.equal(toMcpCandidate(merged[0]).sourceAgentId, "claude-code");
assert.equal(toMcpCandidate(merged[0], "my-ctx").id, "my-ctx");
assert.equal(toMcpCandidate(merged[0], "My-Ctx").displayName, "My-Ctx");

const withId: McpCatalogEntry = {
  id: "context7",
  displayName: "context7",
  serverJson,
};
assert.equal(
  mergeMcpCandidates([reportA], [withId])[0].duplicateReason,
  "existing-id",
);

const renamedInCatalog: McpCatalogEntry = {
  id: "ctx-tool",
  displayName: "ctx-tool",
  serverJson,
};
assert.equal(
  mergeMcpCandidates([reportA], [renamedInCatalog])[0].duplicateReason,
  "existing-server",
  "same server under a different id must be detected",
);

const conflicting: HarvestMcpScanReport = {
  ...reportB,
  candidates: [
    {
      ...reportB.candidates[0],
      id: "context7",
      serverJson: '{"command":"node","args":["other.js"]}',
    },
  ],
};
const conflicted = mergeMcpCandidates([reportA, conflicting], []);
assert.equal(conflicted.length, 2, "same id with different config must stay separate");
assert.ok(conflicted.every((c) => c.duplicateReason === "name-conflict"));

const skillsA: HarvestSkillScanReport = {
  agentId: "claude-code",
  sourcePath: "C:/a/skills",
  candidates: [
    {
      name: "pdf-toolkit",
      path: "C:/a/skills/pdf-toolkit",
      alreadyInLibrary: false,
      sourceAgentId: "claude-code",
    },
  ],
};
const skillsB: HarvestSkillScanReport = {
  agentId: "cursor",
  sourcePath: "C:/b/skills",
  candidates: [
    {
      name: "PDF-Toolkit",
      path: "C:/b/skills/PDF-Toolkit",
      alreadyInLibrary: false,
      sourceAgentId: "cursor",
    },
  ],
};
const mergedSkills = mergeSkillCandidates([skillsA, skillsB], []);
assert.equal(mergedSkills.length, 1, "same skill name from two agents must collapse");
assert.equal(mergedSkills[0].sources.length, 2);
assert.equal(Object.keys(mergedSkills[0].sourcePaths).length, 2);
assert.equal(toSkillCandidate(mergedSkills[0]).path, "C:/a/skills/pdf-toolkit");
assert.equal(toSkillCandidate(mergedSkills[0], "pdf-toolkit-v2").name, "pdf-toolkit-v2");

const library: LibrarySkill[] = [
  { name: "pdf-toolkit", path: "C:/lib/pdf-toolkit", manifestPath: "C:/lib/pdf-toolkit/SKILL.md" },
];
assert.equal(
  mergeSkillCandidates([skillsA], library)[0].duplicateReason,
  "existing-name",
);

console.log("harvest-dedupe checks OK (multi-agent merge, fingerprints, duplicate reasons).");
