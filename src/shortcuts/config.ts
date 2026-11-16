// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * User shortcut overrides — persisted to `{app_config_dir}/shortcut-overrides.json`
 * via the Rust backend. The old localStorage key
 * `afm.shortcutOverrides.v1` is a one-time migration source only.
 *
 * Only the chord arrays are stored here; the action table lives in registry.ts.
 * `shortcutOverridesVersion` is the single reactivity signal: runtime rebuilds
 * its dispatch map and every tooltip / settings row re-resolves final chords
 * whenever an override is written.
 */

import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";
import { chordKey } from "./chord.ts";
import type { ShortcutChord, ShortcutOverrides } from "./types.ts";

/** 旧存储键（localStorage）：仅用于一次性迁移到 shortcut-overrides.json。 */
export const SHORTCUT_OVERRIDES_STORAGE_KEY = "afm.shortcutOverrides.v1";

/** Guard against a corrupt config file flooding the dispatch map. */
export const MAX_CHORDS_PER_ACTION = 8;

export const shortcutOverridesVersion = ref(0);

/** 内存中的覆盖表：启动时由 shortcut-overrides.json 种入，写入时同步更新。 */
let overrides: ShortcutOverrides = {};
/** 串行化后端写请求，保证落盘顺序；失败不阻断会话内配置。 */
let overridesWriteChain: Promise<unknown> = Promise.resolve();
/** 上一次已排队的 wire JSON，跳过无变化写入。 */
let lastQueuedJson: string | null = null;

function isChord(value: unknown): value is ShortcutChord {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const raw = value as Record<string, unknown>;
  if (typeof raw.key !== "string" || raw.key.length === 0 || raw.key.length > 24) {
    return false;
  }
  for (const modifier of ["ctrl", "shift", "alt", "meta"] as const) {
    if (raw[modifier] !== undefined && typeof raw[modifier] !== "boolean") return false;
  }
  return true;
}

/** Sanitize persisted JSON: unknown action ids are kept but filtered by registry.ts. */
export function normalizeShortcutOverrides(raw: unknown): ShortcutOverrides {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return {};
  const result: ShortcutOverrides = {};
  for (const [actionId, chords] of Object.entries(raw as Record<string, unknown>)) {
    if (!Array.isArray(chords)) continue;
    const seen = new Set<string>();
    const clean: ShortcutChord[] = [];
    for (const candidate of chords) {
      if (!isChord(candidate)) continue;
      const key = chordKey(candidate);
      if (seen.has(key)) continue;
      seen.add(key);
      clean.push({
        ...(candidate.ctrl ? { ctrl: true } : {}),
        ...(candidate.shift ? { shift: true } : {}),
        ...(candidate.alt ? { alt: true } : {}),
        ...(candidate.meta ? { meta: true } : {}),
        key: candidate.key,
      });
      if (clean.length >= MAX_CHORDS_PER_ACTION) break;
    }
    // An empty array is a legitimate override: the function is unbound.
    result[actionId] = clean;
  }
  return result;
}

/** 一次性迁移源：localStorage 旧覆盖表。 */
function migrateLegacyOverrides(): ShortcutOverrides {
  try {
    const raw = globalThis.localStorage?.getItem(SHORTCUT_OVERRIDES_STORAGE_KEY);
    if (!raw) return {};
    return normalizeShortcutOverrides(JSON.parse(raw));
  } catch {
    return {};
  }
}

function clearLegacyOverrides(): void {
  try {
    globalThis.localStorage?.removeItem(SHORTCUT_OVERRIDES_STORAGE_KEY);
  } catch {
    /* quota / private mode */
  }
}

/** 把整张覆盖表串行写入后端 shortcut-overrides.json（跳过无变化写入）。 */
function queueOverridesWrite(map: ShortcutOverrides): void {
  const json = JSON.stringify(map);
  if (json === lastQueuedJson) return;
  lastQueuedJson = json;
  overridesWriteChain = overridesWriteChain
    .then(() => invoke("shortcuts_overrides_set", { overrides: map }))
    .catch(() => undefined);
}

/** 等待已排队的覆盖写全部落盘（退出/隐藏 flush 前调用）。 */
export function flushShortcutOverridesPersistence(): Promise<unknown> {
  return overridesWriteChain;
}

/**
 * 冷启动种子：`shortcuts_overrides_get` 读入内存。
 * 后端为空且 localStorage 有旧数据时执行一次性迁移。
 */
export async function hydrateShortcutOverridesFromBackend(): Promise<void> {
  const doc = await invoke<{ overrides: unknown } | null>("shortcuts_overrides_get");
  const fromDoc = normalizeShortcutOverrides(doc?.overrides);
  if (Object.keys(fromDoc).length > 0) {
    overrides = fromDoc;
    lastQueuedJson = JSON.stringify(fromDoc);
    clearLegacyOverrides();
  } else {
    const legacy = migrateLegacyOverrides();
    if (Object.keys(legacy).length > 0) {
      overrides = legacy;
      queueOverridesWrite(legacy);
      void flushShortcutOverridesPersistence().then(clearLegacyOverrides);
    } else {
      overrides = {};
      lastQueuedJson = JSON.stringify({});
    }
  }
  shortcutOverridesVersion.value += 1;
}

export function getShortcutOverrides(): Readonly<ShortcutOverrides> {
  return overrides;
}

function commit(next: ShortcutOverrides): void {
  overrides = next;
  queueOverridesWrite(next);
  shortcutOverridesVersion.value += 1;
}

/** Replace every chord of one action (one-to-many). */
export function setActionChords(
  actionId: string,
  chords: readonly ShortcutChord[],
): void {
  const clean: ShortcutChord[] = [];
  const seen = new Set<string>();
  for (const chord of chords) {
    const key = chordKey(chord);
    if (seen.has(key)) continue;
    seen.add(key);
    clean.push(chord);
    if (clean.length >= MAX_CHORDS_PER_ACTION) break;
  }
  commit(normalizeShortcutOverrides({ ...overrides, [actionId]: clean }));
}

export function resetActionChords(actionId: string): void {
  if (overrides[actionId] === undefined) return;
  const next = { ...overrides };
  delete next[actionId];
  commit(next);
}

export function resetAllShortcutOverrides(): void {
  commit({});
}
