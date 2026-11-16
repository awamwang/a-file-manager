// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Link activation policy: double-click a link -> navigate into the directory
 * the link points to (the target itself), not the link path and not the
 * target's parent folder.
 */

import { entryExtension } from "./entryType.ts";

export type LinkTargetEntry = {
  name: string;
  isLink?: boolean | null;
  linkTarget?: string | null;
};

/** Filesystem links (symlink / junction) and Windows `.lnk` shortcuts. */
export function isLinkEntry(entry: LinkTargetEntry): boolean {
  return Boolean(entry.isLink) || entryExtension(entry.name) === "lnk";
}

/** The directory a link points to, when the native listing resolved one. */
export function linkTargetForEntry(entry: LinkTargetEntry): string | null {
  if (!isLinkEntry(entry)) return null;
  return entry.linkTarget?.trim() || null;
}
