// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Navigator row-icon routing.
 *
 * Some entries can never receive a Windows shell icon (provider `awam://`
 * virtual items, Recycle Bin items pointing at deleted originals). Previously
 * those rows stayed on `shell-icon-slot` forever — the grey loading placeholder
 * looked exactly like "icon failed to load". Others deserve a semantic glyph
 * instead of a generic system icon (`.lnk` shortcuts, symlinks / junctions).
 *
 * Returning `null` keeps `ShellIconImg`: local entries still show the soft slot
 * until their shell icon / thumbnail arrives, with no two-stage glyph swap.
 */

import type { FluentGlyphId } from "../components/icons/glyphs.ts";
import { entryExtension } from "./entryType.ts";
import { isProviderPath } from "./provider/path.ts";
import { isRecyclePath } from "./recycle/path.ts";

export type IconEntry = {
  name: string;
  path: string;
  kind: string;
  /** Semantic marker forwarded from a provider backend's `extra` bag. */
  providerKind?: string | null;
  /** Symlink / junction / reparse point reported by the native listing. */
  isLink?: boolean | null;
};

/** Provider entries that are really links / URL bookmarks, not files. */
const PROVIDER_LINK_KINDS = new Set(["bookmark", "url", "link", "shortcut"]);

/**
 * Dedicated glyph for a special entry, or `null` for a normal local entry.
 *
 * `parentPath` is the pane's current listing path: Recycle Bin / provider items
 * may carry a local-looking `path`, so the containing virtual folder decides.
 */
export function specialEntryIcon(
  entry: IconEntry,
  parentPath = "",
): FluentGlyphId | null {
  const path = entry.path ?? "";
  const providerKind = (entry.providerKind ?? "").trim().toLowerCase();

  // Provider virtual entries have no local shell icon — never enqueue one.
  if (isProviderPath(path)) {
    return PROVIDER_LINK_KINDS.has(providerKind) ? "bookmark" : "provider";
  }
  // Provider child whose path was rewritten to a local path keeps its marker.
  if (isProviderPath(parentPath)) {
    return PROVIDER_LINK_KINDS.has(providerKind) ? "bookmark" : "provider";
  }
  // Recycle Bin items resolve to deleted originals; shell icons cannot load.
  if (isRecyclePath(parentPath)) return "recycle";
  // Symlinks / junctions: the shell often shows a blank page for these, and
  // the entry path may already point at the target rather than the link.
  if (entry.isLink) return "link";
  // Windows shortcut files are regular .lnk files, not filesystem links.
  if (entry.kind !== "dir" && entryExtension(entry.name) === "lnk") {
    return "shortcut";
  }
  return null;
}
