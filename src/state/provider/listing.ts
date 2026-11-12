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
};

export type ProviderNavigatorListing = {
  path: string;
  entries: ProviderNavigatorEntry[];
  dirCount: number;
  fileCount: number;
  requiredQuery?: ProviderQueryParam[] | null;
};

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
    };
  });
  return {
    path,
    entries,
    dirCount,
    fileCount,
    requiredQuery: listing.requiredQuery ?? null,
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
  }));
}
