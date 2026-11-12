// SPDX-License-Identifier: GPL-3.0-or-later

import {
  chordKey,
  eventMatchesChord,
  hasDomTextSelection,
  isEditableTarget,
} from "./chord.ts";
import { BUILTIN_SHORTCUTS, resolveDispatchMap } from "./registry.ts";
import type { ShortcutHandler } from "./types.ts";

const handlers = new Map<string, ShortcutHandler>();
const dispatchByChord = resolveDispatchMap(BUILTIN_SHORTCUTS);

/** 有文字选区时让出给浏览器原生剪贴板，便于复制报错/路径等展示文案。 */
const YIELD_TO_DOM_TEXT_SELECTION = new Set([
  "navigator.copy",
  "navigator.cut",
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
  for (const def of dispatchByChord.values()) {
    if (!eventMatchesChord(event, def.chord)) continue;
    if (!def.allowInEditable && isEditableTarget(event.target)) {
      // Allow Ctrl+Shift+P / F even from inputs so overlays stay reachable.
      const always =
        def.id === "commandPanel.toggle" || def.id === "discovery.globalSearch";
      if (!always) continue;
    }
    if (
      YIELD_TO_DOM_TEXT_SELECTION.has(def.id) &&
      hasDomTextSelection()
    ) {
      continue;
    }
    const handler = handlers.get(def.id);
    if (!handler) continue;
    void handler(event);
    return true;
  }
  return false;
}

/** Test helper: which definition currently owns a chord label key. */
export function ownerForChordKey(key: string) {
  return dispatchByChord.get(key) ?? null;
}

export function chordKeyForId(id: string): string | null {
  const def = BUILTIN_SHORTCUTS.find((s) => s.id === id);
  return def ? chordKey(def.chord) : null;
}
