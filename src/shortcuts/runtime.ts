// SPDX-License-Identifier: GPL-3.0-or-later

import {
  eventMatchesChord,
  hasChordModifier,
  hasDomTextSelection,
  isEditableTarget,
} from "./chord.ts";
import { shortcutOverridesVersion } from "./config.ts";
import {
  actionIdFor,
  effectiveChordLabelsForId,
  effectiveShortcutDefinitions,
  resolveDispatchMap,
} from "./registry.ts";
import type { ShortcutDefinition, ShortcutHandler } from "./types.ts";

const handlers = new Map<string, ShortcutHandler>();

/** Rebuilt whenever an override is written, so dispatch always uses final chords. */
let dispatchCache: { version: number; map: Map<string, ShortcutDefinition> } | null = null;

function currentDispatchMap(): Map<string, ShortcutDefinition> {
  const version = shortcutOverridesVersion.value;
  if (!dispatchCache || dispatchCache.version !== version) {
    dispatchCache = {
      version,
      map: resolveDispatchMap(effectiveShortcutDefinitions()),
    };
  }
  return dispatchCache.map;
}

/** 有文字选区时让出给浏览器原生剪贴板，便于复制报错/路径等展示文案。 */
const YIELD_TO_DOM_TEXT_SELECTION = new Set([
  "navigator.copy",
  "navigator.cut",
]);

/** Overlay toggles stay reachable from inputs — but only with modified chords. */
const ALWAYS_IN_EDITABLE_ACTIONS = new Set([
  "commandPanel.toggle",
  "discovery.globalSearch",
]);

export function registerShortcutHandler(id: string, handler: ShortcutHandler): void {
  handlers.set(id, handler);
}

export function unregisterShortcutHandler(id: string): void {
  handlers.delete(id);
}

/**
 * Try to run a registered shortcut for this keydown.
 * Returns true when a handler ran (caller should typically preventDefault).
 */
export function dispatchShortcutEvent(event: KeyboardEvent): boolean {
  for (const def of currentDispatchMap().values()) {
    if (!eventMatchesChord(event, def.chord)) continue;
    const actionId = actionIdFor(def.id);
    if (!def.allowInEditable && isEditableTarget(event.target)) {
      // A bare key such as ` must keep typing into an input; only modified
      // overlay toggles (Ctrl+Shift+P / Ctrl+Shift+F) bypass editable targets.
      const always =
        ALWAYS_IN_EDITABLE_ACTIONS.has(actionId) && hasChordModifier(def.chord);
      if (!always) continue;
    }
    if (
      YIELD_TO_DOM_TEXT_SELECTION.has(actionId) &&
      hasDomTextSelection()
    ) {
      continue;
    }
    // Alias definitions (and any extra user chord) share the canonical handler.
    const handler = handlers.get(actionId) ?? handlers.get(def.id);
    if (!handler) continue;
    void handler(event);
    return true;
  }
  return false;
}

/** Test helper: which definition currently owns a chord label key. */
export function ownerForChordKey(key: string) {
  return currentDispatchMap().get(key) ?? null;
}

/** Final chord label for an id (null when the function is unbound). */
export function chordKeyForId(id: string): string | null {
  return effectiveChordLabelsForId(id)[0] ?? null;
}
