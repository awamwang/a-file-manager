// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Provider depth-1 listing cache + 会话态发现缓存（AIA-76 / AIA-181）。
 *
 * Wire choice: **idle after app start** (`App.vue` → `scheduleIdle`), plus
 * Providers-route first visit as a safety warm. Keeps rclone/knowledge off the
 * first `navigator_list` critical path. Knowledge root warm is gated by
 * `power.providerPreloadKnowledge` (default on). Idempotent concurrent callers
 * share one invoke. Server-side TTL: 600s ok / 30s error；`provider_list` 与
 * `provider_backends` 共用会话缓存，进入 Providers 页或路径栏访问协议根时不再
 * 重复打网关，除非手动刷新。
 */

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { PROVIDER_ROOT } from "./provider/path";

export type PreloadReport = {
  uris: string[];
  cached: number;
  skipped: string[];
  errors: string[];
};

export type ProviderGatewayStatus = {
  online: boolean;
  baseUrl: string;
  detail?: string | null;
  backendCount?: number | null;
};

export type ProviderBackendSession = {
  id: string;
  name: string;
  description: string;
  healthy: boolean;
  capabilities: string[];
  detail?: string | null;
  interfaceOnly?: boolean;
  source?: string | null;
};

export type ProviderDirListingSession = {
  uri: string;
  backend: string;
  entries: Array<{
    name: string;
    uri: string;
    isDir: boolean;
    isFile: boolean;
    size: number;
    localPath?: string | null;
    extra?: Record<string, unknown>;
  }>;
  error?: string | null;
};

export const providerPreloadReport = ref<PreloadReport | null>(null);
export const providerPreloadError = ref<string | null>(null);
export const providerPreloadRunning = ref(false);

/** 会话内复用的网关状态 / 后端列表 / 根目录列表，避免 Providers 页每次 remount 重拉。 */
export const providerGateway = ref<ProviderGatewayStatus | null>(null);
export const providerBackends = ref<ProviderBackendSession[]>([]);
export const providerRootListing = ref<ProviderDirListingSession | null>(null);
export const providerSessionError = ref<string | null>(null);
export const providerSessionHydrating = ref(false);
export const providerSessionReady = ref(false);

let preloadOnce: Promise<PreloadReport | null> | null = null;
let sessionOnce: Promise<void> | null = null;

/** 拉取网关状态与后端列表（命中 Rust 会话缓存时几乎瞬时）。 */
export async function hydrateProviderDiscovery(): Promise<void> {
  const [status, list] = await Promise.all([
    invoke<ProviderGatewayStatus>("provider_gateway_status"),
    invoke<ProviderBackendSession[]>("provider_backends"),
  ]);
  providerGateway.value = status;
  providerBackends.value = list;
}

/** 拉取 Provider 根目录列表（depth-1 缓存命中时几乎瞬时）。 */
export async function hydrateProviderRootListing(): Promise<void> {
  const listing = await invoke<ProviderDirListingSession>("provider_list", {
    uri: PROVIDER_ROOT,
  });
  providerRootListing.value = listing;
}

/**
 * 确保会话已预热：并发调用共享同一 Promise；已就绪且非 force 时直接复用。
 */
export function ensureProviderSession(options?: {
  force?: boolean;
}): Promise<void> {
  const force = options?.force === true;
  if (!force && sessionOnce) {
    return sessionOnce;
  }
  providerSessionHydrating.value = true;
  providerSessionError.value = null;
  sessionOnce = (async () => {
    try {
      await hydrateProviderDiscovery();
      await hydrateProviderRootListing();
      providerSessionReady.value = true;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      providerSessionError.value = message;
      providerSessionReady.value = false;
      sessionOnce = null;
      throw err;
    } finally {
      providerSessionHydrating.value = false;
    }
  })();
  return sessionOnce;
}

/** Warm depth-1 cache once per session (idempotent concurrent callers share one invoke). */
export function preloadProviderCache(): Promise<PreloadReport | null> {
  if (preloadOnce) return preloadOnce;
  providerPreloadRunning.value = true;
  providerPreloadError.value = null;
  preloadOnce = (async () => {
    try {
      // 先预热发现结果，再 warm 各协议根；两者均写入 Rust 会话缓存。
      await ensureProviderSession().catch(() => undefined);
      const report = await invoke<PreloadReport>("provider_preload");
      providerPreloadReport.value = report;
      return report;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      providerPreloadError.value = message;
      // Allow a later retry (e.g. after backend becomes available).
      preloadOnce = null;
      return null;
    } finally {
      providerPreloadRunning.value = false;
    }
  })();
  return preloadOnce;
}

export async function invalidateProviderCache(uri?: string): Promise<number> {
  const removed = await invoke<number>("provider_cache_invalidate", {
    uri: uri ?? null,
  });
  // Next preload should re-warm after a full clear.
  if (!uri) {
    preloadOnce = null;
    sessionOnce = null;
    providerPreloadReport.value = null;
    providerSessionReady.value = false;
    providerBackends.value = [];
    providerGateway.value = null;
    providerRootListing.value = null;
  }
  return removed;
}

/** 手动刷新：失效会话缓存后重新发现并列出根目录。 */
export async function refreshProviderSession(): Promise<void> {
  await invalidateProviderCache();
  await ensureProviderSession({ force: true });
  // 异步再 warm 各协议根，不阻塞 UI。
  void preloadProviderCache();
}
