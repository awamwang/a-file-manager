// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * User shortcut overrides (localStorage, mirroring `search/scope.ts`).
 *
 * Only the chord arrays are stored here; the action table lives in registry.ts.
 * `shortcutOverridesVersion` is the single reactivity signal: runtime rebuilds
 * its dispatch map and every tooltip / settings row re-resolves final chords
 * whenever an override is written.
 */

import { ref } from "vue";
import { chordKey } from "./chord.ts";
import type { ShortcutChord, ShortcutOverrides } from "./types.ts";

export const SHORTCUT_OVERRIDES_STORAGE_KEY = "afm.shortcutOverrides.v1";

/** Guard against a corrupt config file flooding the dispatch map. */
export const MAX_CHORDS_PER_ACTION = 8;

export const shortcutOverridesVersion = ref(0);

function storage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

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

export function loadShortcutOverrides(store: Storage | null = storage()): ShortcutOverrides {
  if (!store) return {};
  try {
    return normalizeShortcutOverrides(
      JSON.parse(store.getItem(SHORTCUT_OVERRIDES_STORAGE_KEY) ?? "{}"),
    );
  } catch {
    return {};
  }
}

let overrides: ShortcutOverrides = loadShortcutOverrides();

export function getShortcutOverrides(): Readonly<ShortcutOverrides> {
  return overrides;
}

function commit(next: ShortcutOverrides): void {
  overrides = next;
  try {
    storage()?.setItem(SHORTCUT_OVERRIDES_STORAGE_KEY, JSON.stringify(next));
  } catch {
    /* 持久化失败不阻断本次会话内的配置。 */
  }
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
