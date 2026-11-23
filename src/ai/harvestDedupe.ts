// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Pure helpers for multi-agent harvest: merge candidates coming from several
 * Agent Targets, and explain *why* a candidate looks like a duplicate.
 *
 * No Vue / Tauri imports — this module is exercised headlessly by
 * scripts/harvest-dedupe-checks.ts via `node --experimental-strip-types`.
 *
 * Backend (harvest.rs) still owns the final skip-on-import guard; these helpers
 * only shape what the user sees and what gets submitted.
 */
import type {
  HarvestMcpCandidate,
  HarvestMcpScanReport,
  HarvestSkillCandidate,
  HarvestSkillScanReport,
  LibrarySkill,
  McpCatalogEntry,
} from "./types.ts";

export type DuplicateReason =
  | "none"
  | "existing-id"
  | "existing-server"
  | "existing-name"
  | "name-conflict";

export type HarvestMcpCandidateView = HarvestMcpCandidate & {
  sources: string[];
  duplicateReason: DuplicateReason;
  duplicateOf?: string;
};

export type HarvestSkillCandidateView = HarvestSkillCandidate & {
  sources: string[];
  sourcePaths: Record<string, string>;
  duplicateReason: DuplicateReason;
};

/** Fold case / scope / punctuation so `Context7`, `@scope/context7` agree. */
export function normalizeMcpKey(id: string): string {
  return id
    .trim()
    .toLowerCase()
    .replace(/^@/, "")
    .replace(/^.*\//, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function canonical(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === "object") {
    const source = value as Record<string, unknown>;
    const out: Record<string, unknown> = {};
    for (const key of Object.keys(source).sort()) out[key] = canonical(source[key]);
    return out;
  }
  return value;
}

/** Key order and whitespace must not change a server's identity. */
export function normalizeServerJson(json: string): string {
  try {
    return JSON.stringify(canonical(JSON.parse(json)));
  } catch {
    return json.trim();
  }
}

/** FNV-1a — stable across runs, adequate for candidate comparison (not security). */
export function serverFingerprint(json: string): string {
  const text = normalizeServerJson(json);
  let hash = 2166136261;
  for (let i = 0; i < text.length; i += 1) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16);
}

/**
 * Merge MCP candidates from several scan reports.
 * - same normalized id + same server config → one row, sources accumulated
 * - same normalized id + different config → kept apart, marked `name-conflict`
 * - catalog matches are labelled so the UI can explain instead of just disabling
 */
export function mergeMcpCandidates(
  reports: readonly HarvestMcpScanReport[],
  catalog: readonly McpCatalogEntry[],
): HarvestMcpCandidateView[] {
  const catalogIds = new Set(catalog.map((entry) => entry.id.trim().toLowerCase()));
  const catalogByFingerprint = new Map<string, string>();
  for (const entry of catalog) {
    const fingerprint = serverFingerprint(entry.serverJson);
    if (!catalogByFingerprint.has(fingerprint)) {
      catalogByFingerprint.set(fingerprint, entry.id);
    }
  }

  const byKey = new Map<string, Map<string, HarvestMcpCandidateView>>();
  for (const report of reports) {
    for (const candidate of report.candidates) {
      const baseKey = normalizeMcpKey(candidate.id);
      if (!baseKey) continue;
      const fingerprint = serverFingerprint(candidate.serverJson);
      let group = byKey.get(baseKey);
      if (!group) {
        group = new Map();
        byKey.set(baseKey, group);
      }
      const existing = group.get(fingerprint);
      if (existing) {
        if (!existing.sources.includes(candidate.sourceAgentId)) {
          existing.sources.push(candidate.sourceAgentId);
        }
        continue;
      }
      let duplicateReason: DuplicateReason = "none";
      let duplicateOf: string | undefined;
      if (catalogIds.has(candidate.id.trim().toLowerCase())) {
        duplicateReason = "existing-id";
        duplicateOf = candidate.id;
      } else if (catalogByFingerprint.has(fingerprint)) {
        duplicateReason = "existing-server";
        duplicateOf = catalogByFingerprint.get(fingerprint);
      }
      group.set(fingerprint, {
        ...candidate,
        sources: [candidate.sourceAgentId],
        duplicateReason,
        duplicateOf,
      });
    }
  }

  const views: HarvestMcpCandidateView[] = [];
  for (const group of byKey.values()) {
    const conflicting = group.size > 1;
    for (const view of group.values()) {
      if (conflicting) {
        view.duplicateReason = "name-conflict";
        view.duplicateOf = undefined;
      }
      views.push(view);
    }
  }
  return views.sort((a, b) => a.id.localeCompare(b.id));
}

/**
 * Merge Skill candidates from several scan reports by folder name.
 * sourcePaths is kept per agent so a later revision can offer "use this copy".
 */
export function mergeSkillCandidates(
  reports: readonly HarvestSkillScanReport[],
  library: readonly LibrarySkill[],
): HarvestSkillCandidateView[] {
  const existingNames = new Set(library.map((skill) => skill.name.toLowerCase()));
  const byName = new Map<string, HarvestSkillCandidateView>();
  for (const report of reports) {
    for (const candidate of report.candidates) {
      const key = candidate.name.trim().toLowerCase();
      if (!key) continue;
      const existing = byName.get(key);
      if (existing) {
        if (!existing.sources.includes(candidate.sourceAgentId)) {
          existing.sources.push(candidate.sourceAgentId);
        }
        existing.sourcePaths[candidate.sourceAgentId] = candidate.path;
        continue;
      }
      byName.set(key, {
        ...candidate,
        sources: [candidate.sourceAgentId],
        sourcePaths: { [candidate.sourceAgentId]: candidate.path },
        duplicateReason: existingNames.has(key) ? "existing-name" : "none",
      });
    }
  }
  return [...byName.values()].sort((a, b) => a.name.localeCompare(b.name));
}

/** Strip view-only fields; optionally rename before submitting to the backend. */
export function toMcpCandidate(
  view: HarvestMcpCandidateView,
  newId?: string,
): HarvestMcpCandidate {
  const id = newId?.trim().toLowerCase() || view.id;
  return {
    id,
    displayName: newId?.trim() || view.displayName,
    serverJson: view.serverJson,
    alreadyInCatalog: view.alreadyInCatalog,
    sourceAgentId: view.sources[0] ?? view.sourceAgentId,
  };
}

export function toSkillCandidate(
  view: HarvestSkillCandidateView,
  newName?: string,
): HarvestSkillCandidate {
  return {
    name: newName?.trim() || view.name,
    path: view.sourcePaths[view.sources[0] ?? view.sourceAgentId] ?? view.path,
    alreadyInLibrary: view.alreadyInLibrary,
    sourceAgentId: view.sources[0] ?? view.sourceAgentId,
  };
}
