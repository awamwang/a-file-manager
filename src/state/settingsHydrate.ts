// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Single-flight `settings_get` for cold-start (AIA-181 / startup async audit P0).
 * Concurrent callers share one IPC; `settings_set` callers should `rememberSettingsDocument`.
 * AIA-276: schema is static — cache once so Settings remount skips `settings_schema_get`.
 */

import { invoke } from "@tauri-apps/api/core";

export type SettingsDocument = {
  version: number;
  values: Record<string, unknown>;
};

export type SettingsSchemaManifest = {
  version: number;
  tiers: Array<"core" | "power" | "experimental">;
  definitions: Array<{
    key: string;
    tier: "core" | "power" | "experimental";
    title: string;
    description: string;
    valueKind: "boolean" | "string" | "number" | "stringArray" | "enum";
    enumValues?: string[];
    default: unknown;
  }>;
};

let hydrateOnce: Promise<SettingsDocument> | null = null;
let cached: SettingsDocument | null = null;
let schemaOnce: Promise<SettingsSchemaManifest> | null = null;
let schemaCached: SettingsSchemaManifest | null = null;

/** Last hydrated document, if any (sync read for already-warmed chrome). */
export function getCachedSettingsDocument(): SettingsDocument | null {
  return cached;
}

export function getCachedSettingsValues(): Record<string, unknown> | null {
  return cached?.values ?? null;
}

/** Sync schema paint for Settings entry (null until first hydrate). */
export function getCachedSettingsSchema(): SettingsSchemaManifest | null {
  return schemaCached;
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

/**
 * Schema manifest is rebuilt from a static Rust list — fetch once per session.
 * AIA-276: Settings remount must not re-pay `settings_schema_get` IPC.
 */
export function hydrateSettingsSchema(): Promise<SettingsSchemaManifest> {
  if (schemaOnce) return schemaOnce;
  schemaOnce = (async () => {
    try {
      const manifest = await invoke<SettingsSchemaManifest>("settings_schema_get");
      schemaCached = manifest;
      return manifest;
    } catch (err) {
      schemaOnce = null;
      throw err;
    }
  })();
  return schemaOnce;
}

/** Keep cache coherent after `settings_set` (returned document). */
export function rememberSettingsDocument(doc: SettingsDocument): void {
  cached = doc;
  hydrateOnce = Promise.resolve(doc);
}

export function rememberSettingsSchema(manifest: SettingsSchemaManifest): void {
  schemaCached = manifest;
  schemaOnce = Promise.resolve(manifest);
}

export function invalidateSettingsCache(): void {
  cached = null;
  hydrateOnce = null;
}
