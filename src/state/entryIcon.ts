// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Navigator row-icon routing.
 *
 * Some entries can never receive a Windows shell icon (provider `awam://`
 * virtual items, Recycle Bin items pointing at deleted originals); those keep
 * a semantic glyph. Local links (`.lnk` shortcuts, symlinks / junctions) now
 * ALSO return `null`: they render through `ShellIconImg` so they show their
 * target's real icon (an app's own image for `.lnk`) with a Windows-style
 * shortcut-arrow overlay drawn by `ShellIconImg` via the `isLink` flag.
 *
 * Returning `null` keeps `ShellIconImg`: local entries still show the soft slot
 * until their shell icon / thumbnail arrives, with no two-stage glyph swap.
 */

import type { FluentGlyphId } from "../components/icons/glyphs.ts";
import { isProviderPath } from "./provider/path.ts";
import { isCloudFileEntry, providerFileGlyph } from "./providerFileIcon.ts";
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

  // Ordinary cloud files show their real type glyph (document / picture / …)
  // and get a cloud badge in the view; virtual / bookmark / backend / root
  // cards keep their dedicated semantic glyph.
  if (isProviderPath(path) || isProviderPath(parentPath)) {
    if (isCloudFileEntry(entry, parentPath)) return providerFileGlyph(entry.name, entry.kind);
    return PROVIDER_LINK_KINDS.has(providerKind) ? "bookmark" : "provider";
  }
  // Recycle Bin items resolve to deleted originals; shell icons cannot load.
  if (isRecyclePath(parentPath)) return "recycle";
  // Local links (symlinks / junctions / .lnk) deliberately fall through to
  // `ShellIconImg`: the native side resolves the target so the real icon (an
  // app's own image for a .lnk) is shown, with an arrow overlay from the view.
  return null;
}
