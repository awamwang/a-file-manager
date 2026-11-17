// SPDX-License-Identifier: GPL-3.0-or-later
/** Provider 列表 / 解析 / 后端发现桥接。 */

import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  ensureProviderSession,
  providerBackends,
} from "../providerCache.ts";
import {
  providerListingFromDir,
  shouldThrowProviderListError,
  type ProviderBackendInfo,
  type ProviderDirListingWire,
  type ProviderNavigatorListing,
} from "./listing.ts";
import { getProviderQuery, PROVIDER_ROOT, toProviderUri } from "./path.ts";

export type ProviderResolveResult = {
  uri: string;
  localPath?: string | null;
  action?: string | null;
  actionArgs?: Record<string, unknown>;
  error?: string | null;
};

/** 网关 `/api/download` 结果（Rust 侧映射为 camelCase）。 */
export type ProviderDownloadResult = {
  uri: string;
  localPath?: string | null;
  backend?: string | null;
  size?: number;
  ok?: boolean;
  truncated?: boolean;
  action?: string | null;
  error?: string | null;
};

export type { ProviderBackendInfo };

/** 复用会话发现缓存；预加载完成后几乎不再打 `/api/backends`。 */
export async function listProviderBackends(): Promise<ProviderBackendInfo[]> {
  await ensureProviderSession();
  return providerBackends.value as ProviderBackendInfo[];
}

export async function loadProviderListing(
  uri: string,
): Promise<ProviderNavigatorListing> {
  const target = toProviderUri(uri) || PROVIDER_ROOT;
  // depth-1（协议根）命中 Rust 列表缓存；更深路径仍即时拉取。
  const listing = await invoke<ProviderDirListingWire>("provider_list", {
    uri: target,
  });
  // 待查询软空不抛；真实失败无 requiredQuery 才硬失败。有查询条时带上 error 保留列表。
  if (shouldThrowProviderListError(listing)) {
    throw new Error(listing.error ?? "provider_list failed");
  }
  return providerListingFromDir(listing);
}

/** 在应用内 Navigator 定位并选中本机路径（动态 import 避免静态环）。 */
async function revealInNavigator(localPath: string): Promise<void> {
  const { revealItemsInNavigator } = await import("../revealInNavigator.ts");
  await revealItemsInNavigator([localPath]);
}

/**
 * 下载虚拟远程文件到本机。通用下载能力在 awam-file 网关侧（rclone copyto），
 * AFM 仅通过 provider_download 代理调用，拿到 localPath 后在 Navigator 中定位。
 */
export async function downloadProviderEntry(uri: string): Promise<string> {
  const result = await invoke<ProviderDownloadResult>("provider_download", {
    uri,
  });
  if (result.error) {
    throw new Error(result.error);
  }
  const local = result.localPath?.trim();
  if (!result.ok || !local) {
    throw new Error(result.error ?? "Download did not return a local path");
  }
  return local;
}

/**
 * 打开 Provider 非目录条目，统一流程：
 * 1. resolve 已有 localPath（本地后端）→ 直接在 Navigator 定位；
 * 2. resolve 返回 download 动作（rclone/SFTP 等虚拟文件）→ 网关下载后定位；
 * 3. resolve 返回 open_url（浏览器书签）→ 浏览器打开；
 * 4. resolve 返回 push_to_downloads（Prowlarr 结果）→ 真 magnet / 网关取回的 .torrent 字节
 *    交给系统默认程序（Prowlarr 自己的 downloadUrl 需要 API Key，不能直接开）。
 */
export async function openProviderEntry(uri: string): Promise<void> {
  const resolved = await invoke<ProviderResolveResult>("provider_resolve", {
    uri,
  });
  const local = resolved.localPath?.trim();
  if (local) {
    await revealInNavigator(local);
    return;
  }
  if (resolved.action === "download") {
    const downloaded = await downloadProviderEntry(uri);
    await revealInNavigator(downloaded);
    return;
  }
  if (resolved.action === "open_url") {
    const url = resolved.actionArgs?.url;
    if (typeof url === "string" && url) {
      await openUrl(url);
      return;
    }
  }
  if (resolved.action === "push_to_downloads") {
    const args = resolved.actionArgs ?? {};
    const magnet = typeof args.magnetUrl === "string" ? args.magnetUrl.trim() : "";
    const torrentB64 =
      typeof args.torrentB64 === "string" ? args.torrentB64.trim() : "";
    const downloadUrl =
      typeof args.downloadUrl === "string" ? args.downloadUrl.trim() : "";
    const title = getProviderQuery(uri).title || null;
    // 网关（awam-file）已经把 Prowlarr 的代理地址解析成真 magnet 或 .torrent 字节：
    // 优先用它，避免把需要 API Key 的下载地址直接丢给浏览器。
    if (magnet.startsWith("magnet:")) {
      const result = await invoke<{ success: boolean; error: string | null }>(
        "open_torrent_target",
        { url: magnet, title },
      );
      if (!result.success) {
        throw new Error(
          result.error ?? "Failed to open magnet with the system default app",
        );
      }
      return;
    }
    if (torrentB64) {
      const name =
        typeof args.torrentName === "string" && args.torrentName
          ? args.torrentName
          : title;
      const result = await invoke<{ success: boolean; error: string | null }>(
        "open_torrent_data",
        { name, data: torrentB64 },
      );
      if (!result.success) {
        throw new Error(
          result.error ?? "Failed to open .torrent with the system default app",
        );
      }
      return;
    }
    // 最后退路：直链种子地址（少数索引器不校验登录）。
    if (/^https?:\/\//i.test(downloadUrl)) {
      const result = await invoke<{ success: boolean; error: string | null }>(
        "open_torrent_target",
        { url: downloadUrl, title },
      );
      if (result.success) return;
      throw new Error(result.error ?? "Failed to open torrent");
    }
    throw new Error(
      "This search result has no usable magnet or .torrent (the indexer may require login or an API key)",
    );
  }
  if (resolved.error) {
    throw new Error(resolved.error);
  }
  throw new Error("This provider item cannot be opened yet.");
}
