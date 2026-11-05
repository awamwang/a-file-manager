// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Single-flight `settings_get` for cold-start (AIA-181 / startup async audit P0).
 * Concurrent callers share one IPC; `settings_set` callers should `rememberSettingsDocument`.
 */

import { invoke } from "@tauri-apps/api/core";

export type SettingsDocument = {
  version: number;
  values: Record<string, unknown>;
};

let hydrateOnce: Promise<SettingsDocument> | null = null;
let cached: SettingsDocument | null = null;

/** Last hydrated document, if any (sync read for already-warmed chrome). */
export function getCachedSettingsDocument(): SettingsDocument | null {
  return cached;
}

export function getCachedSettingsValues(): Record<string, unknown> | null {
  return cached?.values ?? null;
}

/** One in-flight / resolved `settings_get` for the session until invalidated. */
export function hydrateSettings(): Promise<SettingsDocument> {
  if (hydrateOnce) return hydrateOnce;
  hydrateOnce = (async () => {
    try {
      const doc = await invoke<SettingsDocument>("settings_get");
      cached = doc;
      return doc;
    } catch (err) {
      hydrateOnce = null;
      throw err;
    }
  })();
  return hydrateOnce;
}

/** Keep cache coherent after `settings_set` (returned document). */
export function rememberSettingsDocument(doc: SettingsDocument): void {
  cached = doc;
  hydrateOnce = Promise.resolve(doc);
}

export function invalidateSettingsCache(): void {
  cached = null;
  hydrateOnce = null;
}
