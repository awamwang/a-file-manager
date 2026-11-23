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

/**
 * Physical digit for a keyboard event, or `null` when the code is not a digit.
 *
 * On macOS, Option+digit produces a glyph (`⌥0` → `` ` ``, `⌥1` → `¡`), so
 * `event.key` is no longer `"0"`…`"9"` and matching by key silently breaks the
 * Alt+0…9 work-group chords. `event.code` names the physical key and stays
 * stable across layouts and Option. Used only for single-digit chords so no
 * other binding changes behavior.
 */
export function digitKeyFromEventCode(code: string): string | null {
  const match = /^(?:Digit|Numpad)([0-9])$/.exec(code);
  return match ? match[1] : null;
}

function eventKeyForChord(event: KeyboardEvent, chord: ShortcutChord): string {
  if (/^[0-9]$/.test(chord.key)) {
    const fromCode = digitKeyFromEventCode(event.code ?? "");
    if (fromCode !== null) return fromCode;
  }
  return normalizeEventKey(event.key);
}

export function eventMatchesChord(event: KeyboardEvent, chord: ShortcutChord): boolean {
  if (event.isComposing) return false;
  const primary = effectiveCtrlMeta(chord);
  if (primary.ctrl !== event.ctrlKey) return false;
  if (Boolean(chord.shift) !== event.shiftKey) return false;
  if (Boolean(chord.alt) !== event.altKey) return false;
  if (primary.meta !== event.metaKey) return false;
  return eventKeyForChord(event, chord) === chord.key;
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
    case "delete":
      return "Delete";
    case "backspace":
      return "Backspace";
    case "insert":
      return "Insert";
    case "home":
      return "Home";
    case "end":
      return "End";
    case "pageup":
      return "PageUp";
    case "pagedown":
      return "PageDown";
    case "browserback":
      return "BrowserBack";
    case "browserforward":
      return "BrowserForward";
    default:
      if (/^f\d{1,2}$/.test(key)) return key.toUpperCase();
      return key.length === 1 ? key.toUpperCase() : key;
  }
}

const MODIFIER_KEYS = new Set([
  "control",
  "ctrl",
  "shift",
  "alt",
  "altgraph",
  "meta",
  "os",
  "capslock",
  "numlock",
]);

/**
 * Bare keys that would break editing / overlays even without a chord conflict.
 * Space is deliberately NOT here: it is the built-in preview chord and must be
 * re-bindable after the user removes it.
 */
const RESERVED_BARE_KEYS = new Set([
  "escape",
  "enter",
  "tab",
  "backspace",
]);

export function hasChordModifier(chord: ShortcutChord): boolean {
  return Boolean(chord.ctrl || chord.shift || chord.alt || chord.meta);
}

export function isModifierKey(key: string): boolean {
  return MODIFIER_KEYS.has(key.toLowerCase());
}

export function isReservedBareKey(chord: ShortcutChord): boolean {
  return !hasChordModifier(chord) && RESERVED_BARE_KEYS.has(chord.key);
}

/** Build a chord from a live keydown (Settings recorder). Null = IME / modifier-only. */
export function chordFromEvent(event: KeyboardEvent): ShortcutChord | null {
  if (event.isComposing) return null;
  // Option+digit on macOS emits a glyph in `event.key`; record the physical
  // digit so the Alt+0…9 group chords can be rebound like any other chord.
  const key = digitKeyFromEventCode(event.code ?? "") ?? normalizeEventKey(event.key);
  if (!key || isModifierKey(key)) return null;
  return {
    ...(event.ctrlKey ? { ctrl: true } : {}),
    ...(event.shiftKey ? { shift: true } : {}),
    ...(event.altKey ? { alt: true } : {}),
    ...(event.metaKey ? { meta: true } : {}),
    key,
  };
}

/** ARIA `aria-keyshortcuts` grammar (Control+Shift+P rather than Ctrl+Shift+P). */
export function chordToAria(chord: ShortcutChord): string {
  const parts: string[] = [];
  if (chord.ctrl) parts.push("Control");
  if (chord.shift) parts.push("Shift");
  if (chord.alt) parts.push("Alt");
  if (chord.meta) parts.push("Meta");
  switch (chord.key) {
    case "arrowup":
      parts.push("ArrowUp");
      break;
    case "arrowdown":
      parts.push("ArrowDown");
      break;
    case "arrowleft":
      parts.push("ArrowLeft");
      break;
    case "arrowright":
      parts.push("ArrowRight");
      break;
    case "escape":
      parts.push("Escape");
      break;
    case "enter":
      parts.push("Enter");
      break;
    case "tab":
      parts.push("Tab");
      break;
    case "space":
      parts.push("Space");
      break;
    default:
      parts.push(/^f\d{1,2}$/.test(chord.key) ? chord.key.toUpperCase() : chord.key);
  }
  return parts.join("+");
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
