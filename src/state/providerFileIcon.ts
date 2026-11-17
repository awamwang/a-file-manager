// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Cloud (provider `awam://`) row-icon routing.
 *
 * Ordinary cloud files have no Windows shell icon, but they are real files —
 * they deserve the same type glyph a local file would get (document / picture
 * / music / video / folder) plus a small cloud badge to mark their origin.
 * Virtual / bookmark / backend / root cards are NOT files: they keep their
 * dedicated semantic glyph and never receive the badge.
 */

import type { FluentGlyphId } from "../components/icons/glyphs.ts";
import { entryExtension } from "./entryType.ts";
import { isProviderPath, PROVIDER_ROOT } from "./provider/path.ts";

export type CloudIconEntry = {
  name: string;
  path: string;
  kind: string;
  providerKind?: string | null;
};

/** Bookmark / URL / link markers are links, not cloud files. */
const PROVIDER_LINK_KINDS = new Set(["bookmark", "url", "link", "shortcut"]);
/** Virtual collections and backend cards are containers, not cloud files. */
const PROVIDER_CONTAINER_KINDS = new Set(["virtual", "backend"]);

const IMAGE_EXTS = new Set([
  "png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "ico", "tif", "tiff", "heic",
]);
const AUDIO_EXTS = new Set([
  "mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "aiff", "opus",
]);
const VIDEO_EXTS = new Set([
  "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg",
]);

/**
 * Type glyph for an ordinary cloud file / folder.
 * Unknown extensions (`.md`, `.pdf`, `.txt`, …) fall back to `document`.
 */
export function providerFileGlyph(name: string, kind: string): FluentGlyphId {
  if (kind === "dir") return "folder";
  const ext = entryExtension(name);
  if (IMAGE_EXTS.has(ext)) return "picture";
  if (AUDIO_EXTS.has(ext)) return "musicNote";
  if (VIDEO_EXTS.has(ext)) return "video";
  return "document";
}

/**
 * Whether a listing row is an ordinary cloud file / folder that should show
 * its type glyph with a cloud badge. Everything else (local rows, bookmarks,
 * virtual collections, backend cards, the provider root) returns false.
 */
export function isCloudFileEntry(entry: CloudIconEntry, parentPath = ""): boolean {
  const path = (entry.path ?? "").trim();
  const inProvider = isProviderPath(path) || isProviderPath(parentPath);
  if (!inProvider) return false;
  if (entry.kind !== "file" && entry.kind !== "dir") return false;

  const providerKind = (entry.providerKind ?? "").trim().toLowerCase();
  if (PROVIDER_LINK_KINDS.has(providerKind)) return false;
  if (PROVIDER_CONTAINER_KINDS.has(providerKind)) return false;

  const normalized = path.replace(/\/+$/, "").toLowerCase();
  if (!normalized || normalized === PROVIDER_ROOT) return false;
  // A single-segment backend URI (awam://knowledge) is a backend card, not a file.
  if (isProviderPath(path)) {
    const rest = normalized.slice("awam://".length).split("?")[0] ?? "";
    if (!rest.includes("/")) return false;
  }
  return true;
}
