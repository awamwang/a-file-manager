// SPDX-License-Identifier: GPL-3.0-or-later
/** Provider DirListing → Navigator listing 映射。 */

import { PROVIDER_ROOT, toProviderUri, type ProviderQueryParam } from "./path.ts";

export type ProviderBackendInfo = {
  id: string;
  name: string;
  description: string;
  healthy: boolean;
  capabilities: string[];
  detail?: string | null;
  interfaceOnly?: boolean;
  source?: string | null;
};

export type ProviderEntryWire = {
  name: string;
  uri: string;
  isDir: boolean;
  isFile: boolean;
  size: number;
  modifiedTime?: number;
  localPath?: string | null;
  extra?: Record<string, unknown>;
  requiredQuery?: ProviderQueryParam[] | null;
};

export type ProviderDirListingWire = {
  uri: string;
  backend: string;
  entries: ProviderEntryWire[];
  error?: string | null;
  requiredQuery?: ProviderQueryParam[] | null;
};

export type ProviderNavigatorEntry = {
  name: string;
  path: string;
  kind: "dir" | "file";
  size: number;
  modifiedMs: number | null;
  /**
   * Semantic hint forwarded from the backend's `extra` bag
   * (`kind: "bookmark"` / `url` / `virtual: true` / `"backend"` root card).
   * Kept so Navigator can give virtual entries a dedicated icon instead of
   * leaving them on the shell-icon loading placeholder forever.
   */
  providerKind?: string | null;
};

export type ProviderNavigatorListing = {
  path: string;
  entries: ProviderNavigatorEntry[];
  dirCount: number;
  fileCount: number;
  requiredQuery?: ProviderQueryParam[] | null;
  error?: string | null;
};

const LEGACY_MISSING_QUERY = /missing\s+query/i;

/**
 * Backends flag non-file entries in `extra`: browser bookmarks carry
 * `kind: "bookmark"` + `url`, knowledge/rclone virtual nodes carry
 * `virtual: true`. Keep only the semantic marker, not the whole bag.
 */
export function providerKindFromExtra(
  extra: Record<string, unknown> | null | undefined,
): string | null {
  if (!extra) return null;
  const rawKind = extra.kind;
  if (typeof rawKind === "string" && rawKind.trim()) {
    return rawKind.trim().toLowerCase();
  }
  if (typeof extra.url === "string" && extra.url.trim()) return "bookmark";
  if (extra.virtual === true) return "virtual";
  return null;
}

/** 待查询软空：无 error，或旧网关 Missing query 文案且已声明 requiredQuery。 */
export function isSoftProviderQueryAwaiting(
  error?: string | null,
  requiredQuery?: ProviderQueryParam[] | null,
): boolean {
  if (!requiredQuery?.length) return false;
  const text = error?.trim() ?? "";
  return text.length === 0 || LEGACY_MISSING_QUERY.test(text);
}

/** 仅在真实失败且没有 requiredQuery 时硬失败；有查询条时保留列表。 */
export function shouldThrowProviderListError(
  listing: Pick<ProviderDirListingWire, "error" | "requiredQuery">,
): boolean {
  const error = listing.error?.trim() ?? "";
  if (!error) return false;
  if (isSoftProviderQueryAwaiting(listing.error, listing.requiredQuery)) {
    return false;
  }
  return !listing.requiredQuery?.length;
}

/** 将 `provider_list` 结果映射为 Navigator 列表形状。 */
export function providerListingFromDir(
  listing: ProviderDirListingWire,
): ProviderNavigatorListing {
  const path = toProviderUri(listing.uri) || PROVIDER_ROOT;
  let dirCount = 0;
  let fileCount = 0;
  const entries = listing.entries.map((entry) => {
    const isDir = Boolean(entry.isDir);
    if (isDir) dirCount += 1;
    else fileCount += 1;
    const modified =
      typeof entry.modifiedTime === "number" && entry.modifiedTime > 0
        ? entry.modifiedTime
        : null;
    return {
      name: entry.name,
      path: entry.uri,
      kind: (isDir ? "dir" : "file") as "dir" | "file",
      size: entry.size ?? 0,
      modifiedMs: modified,
      providerKind: providerKindFromExtra(entry.extra),
    };
  });
  const awaiting = isSoftProviderQueryAwaiting(
    listing.error,
    listing.requiredQuery,
  );
  const errorText = listing.error?.trim() ?? "";
  return {
    path,
    entries,
    dirCount,
    fileCount,
    requiredQuery: listing.requiredQuery ?? null,
    error: awaiting || !errorText ? null : listing.error,
  };
}

/** 后端目录卡片 → 根级 Navigator 条目（Guide / Providers 入口用）。 */
export function providerRootEntriesFromBackends(
  backends: ProviderBackendInfo[],
): ProviderNavigatorEntry[] {
  return backends.map((backend) => ({
    name: backend.name || backend.id,
    path: `awam://${backend.id}`,
    kind: "dir" as const,
    size: 0,
    modifiedMs: null,
    providerKind: "backend",
  }));
}
