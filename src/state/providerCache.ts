// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Provider depth-1 listing cache (AIA-76 / AIA-181).
 *
 * Wire choice: **idle after app start** (`App.vue` → `scheduleIdle`), plus
 * Providers-route first visit as a safety warm. Keeps rclone/knowledge off the
 * first `navigator_list` critical path. Idempotent concurrent callers share one invoke.
 * Server-side TTL: 60s ok / 15s error; `provider_list` reads the same cache.
 */

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type PreloadReport = {
  uris: string[];
  cached: number;
  skipped: string[];
  errors: string[];
};

export const providerPreloadReport = ref<PreloadReport | null>(null);
export const providerPreloadError = ref<string | null>(null);
export const providerPreloadRunning = ref(false);

let preloadOnce: Promise<PreloadReport | null> | null = null;

/** Warm depth-1 cache once per session (idempotent concurrent callers share one invoke). */
export function preloadProviderCache(): Promise<PreloadReport | null> {
  if (preloadOnce) return preloadOnce;
  providerPreloadRunning.value = true;
  providerPreloadError.value = null;
  preloadOnce = (async () => {
    try {
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
    providerPreloadReport.value = null;
  }
  return removed;
}
