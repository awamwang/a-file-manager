// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Platform copy overrides (macOS port, 7.22).
 *
 * The locale catalogs keep their Windows-first wording as the base, so Windows
 * copy cannot regress. Each catalog additionally carries a `platform.macos`
 * section: a flat map from dotted message keys to macOS wording (Finder / Trash
 * / This Mac / ⌘-based shortcut labels / `~/Library/Application Support`).
 *
 * This module merges that section into the catalog *before* vue-i18n sees it.
 * No component needs an `isMacPlatform()` branch and no `t()` call site
 * changes. Host detection is delegated to `src/config/platform.ts`, the same
 * capability contract the rest of the UI uses; an unknown host keeps the base
 * wording (the neutral sentinel is Windows-first by design).
 *
 * Static contract: `scripts/check-platform-terms.mjs`.
 */

export type MessageCatalog = Record<string, unknown>;

const SECTION = "platform";
const VARIANT = "macos";

/** Flat `dotted.key -> macOS copy` map; keys are validated by the contract. */
export function macosOverrides(catalog: MessageCatalog): Record<string, string> {
  const section = catalog[SECTION];
  if (!section || typeof section !== "object" || Array.isArray(section)) return {};
  const variant = (section as MessageCatalog)[VARIANT];
  if (!variant || typeof variant !== "object" || Array.isArray(variant)) return {};
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(variant as MessageCatalog)) {
    if (typeof value === "string") out[key] = value;
  }
  return out;
}

/** Overwrite an existing leaf only; a broken key must never create new copy. */
function setAtPath(target: MessageCatalog, dotted: string, value: string): void {
  const parts = dotted.split(".");
  let node = target;
  for (const part of parts.slice(0, -1)) {
    const next = node[part];
    if (!next || typeof next !== "object" || Array.isArray(next)) return;
    node = next as MessageCatalog;
  }
  const leaf = parts[parts.length - 1];
  if (typeof node[leaf] === "string") node[leaf] = value;
}

/**
 * Return a catalog with the internal `platform` section removed and, on macOS,
 * the overrides merged in. The imported JSON module is never mutated, so the
 * base catalog stays available when capabilities resolve later.
 */
export function localizeCatalog(catalog: MessageCatalog, mac: boolean): MessageCatalog {
  const overrides = mac ? macosOverrides(catalog) : {};
  const out = JSON.parse(JSON.stringify(catalog)) as MessageCatalog;
  delete out[SECTION];
  for (const [key, value] of Object.entries(overrides)) {
    setAtPath(out, key, value);
  }
  return out;
}
