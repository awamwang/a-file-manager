// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Info fallback for entries that have no real, locally readable content.
 *
 * Provider (`awam://…`) items — SFTP / rclone remotes, bookmarks, virtual nodes
 * — and the guide / recycle-bin pseudo paths are not filesystem paths. Piping
 * them into `preview_load` / `navigator_stat` only yields "Not a file" or
 * "Path does not exist" errors. Both the preview and the properties/details
 * panes use this module to show the metadata we already have from the listing
 * instead of a raw error.
 *
 * Pure: no IPC, no Vue runtime. A `t` + time formatter are injected so labels
 * stay localized and the module is trivial to unit-check.
 */

import type { FluentGlyphId } from "../components/icons/glyphs.ts";
import { formatBytes } from "./formatBytes.ts";
import { isGuidePath } from "./guide";
import {
  getProviderQuery,
  isProviderPath,
  providerBackendId,
  providerLeafName,
} from "./provider";
import { isRecyclePath } from "./recycle";

/** The listing fields we consume (NavigatorEntry structurally satisfies this). */
export type VirtualInfoSource = {
  name?: string;
  path?: string;
  kind?: string;
  size?: number;
  modifiedMs?: number | null;
  providerKind?: string | null;
};

export type VirtualInfoRow = {
  key: string;
  label: string;
  value: string;
  mono?: boolean;
  /** Long machine-generated values (e.g. magnet) get a click-to-copy control. */
  copyable?: boolean;
};

export type VirtualEntryInfo = {
  /** Header icon; always a glyph that needs no shell/icon flight. */
  glyph: FluentGlyphId;
  /** Localized type label shown under the header icon. */
  kindLabel: string;
  /** Ordered metadata rows already localized and formatted. */
  rows: VirtualInfoRow[];
  /** Localized banner explaining why there is no live preview. */
  hint: string;
};

export type VirtualInfoDeps = {
  t: (key: string) => string;
  formatTime: (ms: number | null) => string;
};

const PROVIDER_LINK_KINDS = new Set(["bookmark", "url", "link", "shortcut"]);
const MAX_QUERY_ROWS = 8;

/** True when `path` is a virtual/pseudo location with no local file to stat. */
export function isVirtualEntryPath(path: string | null | undefined): boolean {
  if (!path) return false;
  return isProviderPath(path) || isGuidePath(path) || isRecyclePath(path);
}

function lookupName(
  path: string,
  entry: VirtualInfoSource | null | undefined,
  fallback: string,
): string {
  const named = entry?.name?.trim();
  if (named) return named;
  if (isProviderPath(path)) {
    const leaf = providerLeafName(path);
    if (leaf && leaf !== "root") return leaf;
  }
  return fallback;
}

/**
 * Build display info for a virtual path, or `null` for a normal filesystem
 * path (callers then fall back to the native `preview_load` / `navigator_stat`).
 */
export function virtualEntryInfo(
  path: string | null | undefined,
  entry: VirtualInfoSource | null | undefined,
  deps: VirtualInfoDeps,
): VirtualEntryInfo | null {
  const { t, formatTime } = deps;
  const raw = (path ?? "").trim();
  if (!raw) return null;

  if (isGuidePath(raw)) {
    return {
      glyph: "desktop",
      kindLabel: t("virtualInfo.kindGuide"),
      rows: [
        { key: "name", label: t("properties.name"), value: t("guide.title") },
        { key: "path", label: t("properties.path"), value: raw, mono: true },
      ],
      hint: t("virtualInfo.hintSpecial"),
    };
  }

  if (isRecyclePath(raw)) {
    return {
      glyph: "recycle",
      kindLabel: t("virtualInfo.kindRecycle"),
      rows: [
        { key: "name", label: t("properties.name"), value: t("virtualInfo.recycleName") },
        { key: "path", label: t("properties.path"), value: raw, mono: true },
      ],
      hint: t("virtualInfo.hintSpecial"),
    };
  }

  if (!isProviderPath(raw)) return null;

  const providerKind = (entry?.providerKind ?? "").trim().toLowerCase();
  const backend = providerBackendId(raw);
  const isRoot = raw.replace(/\/+$/, "").toLowerCase() === "awam://root";
  const isBackendCard = providerKind === "backend" || (backend !== null && !raw.slice("awam://".length).replace(/^\/+/, "").includes("/"));

  let glyph: FluentGlyphId = "provider";
  let kindLabel = t("virtualInfo.kindRemoteItem");
  if (isRoot) {
    glyph = "cloud";
    kindLabel = t("virtualInfo.kindProviderRoot");
  } else if (PROVIDER_LINK_KINDS.has(providerKind)) {
    glyph = "bookmark";
    kindLabel = t("virtualInfo.kindBookmark");
  } else if (isBackendCard) {
    glyph = "provider";
    kindLabel = t("virtualInfo.kindBackend");
  } else if (entry?.kind === "dir") {
    glyph = "folder";
    kindLabel = t("virtualInfo.kindRemoteFolder");
  } else if (entry?.kind === "file") {
    glyph = "document";
    kindLabel = t("virtualInfo.kindRemoteFile");
  }

  const name = lookupName(
    raw,
    entry,
    isRoot ? t("virtualInfo.remoteServicesName") : backend ?? t("properties.unknown"),
  );
  const rows: VirtualInfoRow[] = [
    { key: "name", label: t("properties.name"), value: name },
    { key: "kind", label: t("properties.kind"), value: kindLabel },
  ];
  if (backend) {
    rows.push({
      key: "backend",
      label: t("virtualInfo.backend"),
      value: backend,
      mono: true,
    });
  }
  if (entry?.kind === "file" && typeof entry.size === "number") {
    rows.push({
      key: "size",
      label: t("properties.size"),
      value: formatBytes(entry.size),
    });
  }
  if (entry?.modifiedMs != null) {
    rows.push({
      key: "modified",
      label: t("properties.modified"),
      value: formatTime(entry.modifiedMs),
    });
  }
  rows.push({ key: "path", label: t("properties.path"), value: raw, mono: true });

  const query = getProviderQuery(raw);
  let shown = 0;
  for (const [key, value] of Object.entries(query)) {
    if (shown >= MAX_QUERY_ROWS) break;
    rows.push({
      key: `query:${key}`,
      label: key,
      value,
      mono: true,
      copyable: key.toLowerCase() === "magnet" && Boolean(value.trim()),
    });
    shown += 1;
  }

  return {
    glyph,
    kindLabel,
    rows,
    hint: t("virtualInfo.hintRemote"),
  };
}
