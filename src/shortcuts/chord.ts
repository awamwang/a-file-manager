// SPDX-License-Identifier: GPL-3.0-or-later

import type { ShortcutChord } from "./types.ts";

/** Stable string for conflict maps / equality. */
export function chordKey(chord: ShortcutChord): string {
  const parts: string[] = [];
  if (chord.ctrl) parts.push("Ctrl");
  if (chord.shift) parts.push("Shift");
  if (chord.alt) parts.push("Alt");
  if (chord.meta) parts.push("Meta");
  parts.push(displayMainKey(chord.key));
  return parts.join("+");
}

export function formatChordLabel(chord: ShortcutChord): string {
  return chordKey(chord);
}

export function normalizeEventKey(key: string): string {
  if (key === " ") return "space";
  if (key.length === 1) return key.toLowerCase();
  return key.toLowerCase();
}

export function eventMatchesChord(event: KeyboardEvent, chord: ShortcutChord): boolean {
  if (event.isComposing) return false;
  if (Boolean(chord.ctrl) !== event.ctrlKey) return false;
  if (Boolean(chord.shift) !== event.shiftKey) return false;
  if (Boolean(chord.alt) !== event.altKey) return false;
  if (Boolean(chord.meta) !== event.metaKey) return false;
  return normalizeEventKey(event.key) === chord.key;
}

function displayMainKey(key: string): string {
  switch (key) {
    case "arrowup":
      return "↑";
    case "arrowdown":
      return "↓";
    case "arrowleft":
      return "←";
    case "arrowright":
      return "→";
    case "escape":
      return "Esc";
    case " ":
    case "space":
      return "Space";
    case "tab":
      return "Tab";
    case ",":
      return ",";
    default:
      if (/^f\d{1,2}$/.test(key)) return key.toUpperCase();
      return key.length === 1 ? key.toUpperCase() : key;
  }
}

export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  return Boolean(target.closest("[contenteditable='true']"));
}
