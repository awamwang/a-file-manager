// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Open-tool extension allow-list helpers. Pure (no imports) so the normalize
 * rule is unit-checkable headlessly and shared by the Settings custom-tool
 * input.
 */

/**
 * Normalize a comma-separated extension input into the persisted allow-list:
 * strip `*.` / leading dots, lowercase, drop empties. Empty input = unscoped
 * (empty list). A lone `*` (any file, never a directory) survives untouched.
 */
export function normalizeOpenToolExtensions(raw: string): string[] {
  return raw
    .split(/[,，\s]+/)
    .map((token) => token.trim().replace(/^\*+\./, "").replace(/^\.+/, "").toLowerCase())
    .filter(Boolean);
}
