// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Transfer-source dedup (SEL-007 / SEL-011) — pure, headless-testable.
 * When one copy/move/paste carries both a folder and one of its descendants, the
 * descendant is already inside the ancestor's subtree, so processing it again would
 * double-handle (or self-nest). Drop any source that is a strict descendant of another
 * source. Folder-into-itself *target* is a separate backend refusal (TR-006).
 */

import { toNativeFsPath } from "./pathKey.ts";

function dirKey(path: string): string {
  return toNativeFsPath(path)
    .replace(/\\+$/, "")
    .toLowerCase();
}

function isStrictDescendant(ancestorKey: string, nodeKey: string): boolean {
  if (!ancestorKey) return false;
  return nodeKey.length > ancestorKey.length && nodeKey.startsWith(`${ancestorKey}\\`);
}

/** Collapse exact duplicates, then drop sources contained in another source. */
export function dedupeDescendantSources(sources: string[]): string[] {
  const unique: string[] = [];
  const seen = new Set<string>();
  for (const source of sources) {
    const key = dirKey(source);
    if (!key || seen.has(key)) continue;
    seen.add(key);
    unique.push(source);
  }
  const keys = unique.map((source) => dirKey(source));
  const out: string[] = [];
  unique.forEach((_source, index) => {
    const redundant = keys.some(
      (ancestorKey, other) => other !== index && isStrictDescendant(ancestorKey, keys[index] ?? ""),
    );
    if (!redundant) out.push(unique[index] as string);
  });
  return out;
}
