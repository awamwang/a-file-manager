// SPDX-License-Identifier: GPL-3.0-or-later

import type { ShortcutChord } from "./types.ts";
import { isMacPlatform, platformShortcutLabel } from "../config/platform.ts";

/**
 * Stable string for conflict maps / equality.
 *
 * Deliberately platform-independent: `resolveDispatchMap`, conflict detection
 * and `chordKeyForId` all use this key, so it must not change when the host
 * switches from Ctrl to ⌘. Only display (`formatChordLabel`) and event
 * matching (`eventMatchesChord`) are platform-aware.
 */
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
  // Registry chords stay Windows-first (`ctrl: true`); on macOS the same chord
  // is ⌘, and the label must say so.
  return platformShortcutLabel(chordKey(chord));
}

/**
 * The registry declares chords with the Windows primary modifier. On macOS the
 * primary modifier is ⌘ (`metaKey`), so `ctrl` and `meta` swap for matching.
 * Alt/Shift stay literal (⌥/⇧ are the same physical keys).
 */
function effectiveCtrlMeta(chord: ShortcutChord): { ctrl: boolean; meta: boolean } {
  if (!isMacPlatform()) {
    return { ctrl: Boolean(chord.ctrl), meta: Boolean(chord.meta) };
  }
  return { ctrl: Boolean(chord.meta), meta: Boolean(chord.ctrl) };
}

export function normalizeEventKey(key: string): string {
  if (key === " ") return "space";
  if (key.length === 1) return key.toLowerCase();
  return key.toLowerCase();
}

export function eventMatchesChord(event: KeyboardEvent, chord: ShortcutChord): boolean {
  if (event.isComposing) return false;
  const primary = effectiveCtrlMeta(chord);
  if (primary.ctrl !== event.ctrlKey) return false;
  if (Boolean(chord.shift) !== event.shiftKey) return false;
  if (Boolean(chord.alt) !== event.altKey) return false;
  if (primary.meta !== event.metaKey) return false;
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
    case "enter":
      return "Enter";
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
  if (typeof HTMLElement === "undefined") return false;
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  return Boolean(target.closest("[contenteditable='true']"));
}

/** 页面上有非空文字选区时，应让出 Ctrl+C/X 给系统复制，而不是走文件剪贴板。 */
export function hasDomTextSelection(): boolean {
  if (typeof window === "undefined" || typeof window.getSelection !== "function") {
    return false;
  }
  const selection = window.getSelection();
  if (!selection || selection.rangeCount === 0 || selection.isCollapsed) {
    return false;
  }
  return selection.toString().length > 0;
}
