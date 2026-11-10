// SPDX-License-Identifier: GPL-3.0-or-later
/** Provider 列表 / 解析 / 后端发现桥接。 */

import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  providerListingFromDir,
  type ProviderBackendInfo,
  type ProviderDirListingWire,
  type ProviderNavigatorListing,
} from "./listing";
import { PROVIDER_ROOT, toProviderUri } from "./path";

export type ProviderResolveResult = {
  uri: string;
  localPath?: string | null;
  action?: string | null;
  actionArgs?: Record<string, unknown>;
  error?: string | null;
};

export type { ProviderBackendInfo };

export async function listProviderBackends(): Promise<ProviderBackendInfo[]> {
  return invoke<ProviderBackendInfo[]>("provider_backends");
}

export async function loadProviderListing(
  uri: string,
): Promise<ProviderNavigatorListing> {
  const target = toProviderUri(uri) || PROVIDER_ROOT;
  const listing = await invoke<ProviderDirListingWire>("provider_list", {
    uri: target,
  });
  if (listing.error) {
    throw new Error(listing.error);
  }
  return providerListingFromDir(listing);
}

/** 打开 Provider 非目录条目：优先本地路径，其次 open_url 动作。 */
export async function openProviderEntry(uri: string): Promise<void> {
  const resolved = await invoke<ProviderResolveResult>("provider_resolve", {
    uri,
  });
  if (resolved.error) {
    throw new Error(resolved.error);
  }
  const local = resolved.localPath?.trim();
  if (local) {
    const result = await invoke<{ success: boolean; error: string | null }>(
      "open_path_default",
      { path: local },
    );
    if (!result.success) {
      throw new Error(result.error ?? "Failed to open local path");
    }
    return;
  }
  if (resolved.action === "open_url") {
    const url = resolved.actionArgs?.url;
    if (typeof url === "string" && url) {
      await openUrl(url);
      return;
    }
  }
  throw new Error("This provider item cannot be opened yet.");
}
