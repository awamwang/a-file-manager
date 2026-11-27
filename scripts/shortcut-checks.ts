// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  chordFromEvent,
  chordKey,
  eventMatchesChord,
  formatChordLabel,
  hasDomTextSelection,
} from "../src/shortcuts/chord.ts";
import { NEUTRAL_PLATFORM, platformAriaKeys } from "../src/config/platform.ts";
import {
  FIXED_CHORD_KEYS,
  SHORTCUT_ACTIONS,
  BUILTIN_SHORTCUTS,
  actionIdFor,
  effectiveChordLabelsForId,
  effectiveShortcutDefinitions,
  findShortcutConflicts,
  listShortcutActionsForDisplay,
  listShortcutsForDisplay,
  resolveDispatchMap,
  validateNewChord,
} from "../src/shortcuts/registry.ts";
import {
  flushShortcutOverridesPersistence,
  normalizeShortcutOverrides,
  resetAllShortcutOverrides,
  setActionChords,
} from "../src/shortcuts/config.ts";
import {
  dispatchShortcutEvent,
  ownerForChordKey,
  registerShortcutHandler,
  unregisterShortcutHandler,
} from "../src/shortcuts/runtime.ts";
import { isToggleCommandPanelShortcut } from "../src/state/commandPanel.ts";
import { isGlobalSearchShortcut } from "../src/state/globalSearch.ts";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

assert.ok(BUILTIN_SHORTCUTS.length >= 6);
assert.equal(findShortcutConflicts().length, 0, "builtins must not conflict");

const winners = resolveDispatchMap();
assert.equal(
  winners.get(chordKey({ ctrl: true, shift: true, key: "p" }))?.id,
  "commandPanel.toggle",
);
assert.equal(
  winners.get(chordKey({ ctrl: true, shift: true, key: "f" }))?.id,
  "discovery.globalSearch",
);
assert.equal(
  winners.get(chordKey({ ctrl: true, key: "e" }))?.id,
  "navigator.folderSearch",
);
assert.equal(
  winners.get(chordKey({ key: "f3" }))?.id,
  "navigator.folderSearchF3",
);
assert.equal(
  winners.get(chordKey({ alt: true, key: "arrowleft" }))?.id,
  "navigator.back",
);
assert.equal(
  winners.get(chordKey({ ctrl: true, alt: true, key: "arrowleft" }))?.id,
  "navigator.backCtrlAlt",
);
assert.equal(
  winners.get(chordKey({ ctrl: true, alt: true, key: "arrowright" }))?.id,
  "navigator.forwardCtrlAlt",
);
assert.equal(
  winners.get(chordKey({ key: "browserback" }))?.id,
  "navigator.backBrowser",
);
assert.equal(
  winners.get(chordKey({ key: "browserforward" }))?.id,
  "navigator.forwardBrowser",
);
assert.equal(
  winners.get(chordKey({ ctrl: true, key: "tab" }))?.id,
  "navigator.nextTabGroup",
);
assert.equal(
  winners.get(chordKey({ ctrl: true, shift: true, key: "tab" }))?.id,
  "navigator.prevTabGroup",
);
assert.equal(
  winners.get(chordKey({ alt: true, key: "n" }))?.id,
  "view.work-groups",
);
assert.equal(
  winners.get(chordKey({ alt: true, key: "1" }))?.id,
  "workGroup.switch1",
);
assert.equal(
  winners.get(chordKey({ alt: true, key: "`" }))?.id,
  "workGroup.switch0",
  "default work group is Alt+backquote",
);
assert.equal(
  winners.get(chordKey({ alt: true, key: "9" }))?.id,
  "workGroup.switch9",
);
assert.equal(
  winners.get(chordKey({ key: "space" }))?.id,
  "navigator.previewBridge",
);
assert.equal(
  winners.get(chordKey({ alt: true, key: "enter" }))?.id,
  "navigator.properties",
);
assert.equal(
  winners.get(chordKey({ alt: true, key: "p" }))?.id,
  "navigator.previewPane",
);
assert.equal(
  winners.get(chordKey({ alt: true, shift: true, key: "p" }))?.id,
  "navigator.detailsPane",
  "AIA-235: Alt+Shift+P toggles details pane",
);
assert.equal(
  winners.get(chordKey({ ctrl: true, key: "z" }))?.id,
  "navigator.undo",
);
assert.equal(
  winners.get(chordKey({ alt: true, key: "d" }))?.id,
  "chrome.focusAddressAltD",
);
assert.equal(
  winners.get(chordKey({ key: "f4" }))?.id,
  "chrome.focusAddressF4",
);
assert.equal(
  winners.get(chordKey({ key: "f6" }))?.id,
  "chrome.focusCycle",
);
assert.equal(
  winners.get(chordKey({ shift: true, key: "f6" }))?.id,
  "chrome.focusCycleReverse",
);
assert.equal(
  winners.get(chordKey({ ctrl: true, key: "d" }))?.id,
  "navigator.delete",
);
assert.equal(
  winners.get(chordKey({ key: "delete" }))?.id,
  "navigator.deleteKey",
);
assert.equal(
  winners.get(chordKey({ ctrl: true, key: "w" }))?.id,
  "navigator.closeTab",
);
assert.equal(
  winners.get(chordKey({ ctrl: true, key: "1" }))?.id,
  "navigator.jumpTab1",
);
assert.equal(
  winners.get(chordKey({ ctrl: true, key: "9" }))?.id,
  "navigator.jumpTab9",
);
assert.equal(
  eventMatchesChord(keyEvent({ key: " " }), { key: "space" }),
  true,
);
assert.equal(
  eventMatchesChord(keyEvent({ altKey: true, key: "n" }), {
    alt: true,
    key: "n",
  }),
  true,
);
assert.equal(
  eventMatchesChord(
    keyEvent({ ctrlKey: true, altKey: true, key: "n" }),
    { alt: true, key: "n" },
  ),
  false,
  "Ctrl+Alt+N (new file) must not match Alt+N (work groups)",
);
assert.equal(
  eventMatchesChord(keyEvent({ ctrlKey: true, key: "Tab" }), {
    ctrl: true,
    key: "tab",
  }),
  true,
);
assert.equal(
  eventMatchesChord(
    keyEvent({ ctrlKey: true, shiftKey: true, key: "Tab" }),
    { ctrl: true, shift: true, key: "tab" },
  ),
  true,
);
assert.equal(
  eventMatchesChord(keyEvent({ altKey: true, key: "ArrowLeft" }), {
    alt: true,
    key: "arrowleft",
  }),
  true,
);
assert.equal(
  eventMatchesChord(
    keyEvent({ ctrlKey: true, altKey: true, key: "ArrowLeft" }),
    { alt: true, key: "arrowleft" },
  ),
  false,
  "Ctrl+Alt+Left must not match Alt+Left (exact modifiers)",
);
assert.equal(
  eventMatchesChord(
    keyEvent({ ctrlKey: true, altKey: true, key: "ArrowLeft" }),
    { ctrl: true, alt: true, key: "arrowleft" },
  ),
  true,
);

const conflicted = findShortcutConflicts([
  ...BUILTIN_SHORTCUTS,
  {
    id: "dup.panel",
    labelKey: "x",
    chord: { ctrl: true, shift: true, key: "p" },
    scope: "global",
  },
]);
assert.equal(conflicted.length, 1);
assert.deepEqual(conflicted[0].ids.sort(), ["commandPanel.toggle", "dup.panel"]);

const display = listShortcutsForDisplay([
  ...BUILTIN_SHORTCUTS,
  {
    id: "dup.panel",
    labelKey: "x",
    chord: { ctrl: true, shift: true, key: "p" },
    scope: "global",
  },
]);
assert.equal(display.find((r) => r.id === "dup.panel")?.blockedBy, "commandPanel.toggle");

function keyEvent(partial: Partial<KeyboardEvent>): KeyboardEvent {
  return {
    key: "P",
    code: "KeyP",
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    isComposing: false,
    ...partial,
  } as KeyboardEvent;
}

// macOS Option+digit emits a glyph in `key` (⌥0 → `, ⌥1 → ¡) while `code`
// still names the physical DigitN key. The Alt+1…9 work-group chords must
// match by code; all non-digit chords keep matching by key.
assert.equal(
  eventMatchesChord(
    keyEvent({ key: "`", code: "Digit0", altKey: true }),
    { alt: true, key: "0" },
  ),
  true,
);
assert.equal(
  eventMatchesChord(
    keyEvent({ key: "¡", code: "Digit1", altKey: true }),
    { alt: true, key: "1" },
  ),
  true,
);
assert.equal(
  eventMatchesChord(
    keyEvent({ key: "¡", code: "Digit1", altKey: true }),
    { alt: true, key: "2" },
  ),
  false,
);
assert.equal(
  eventMatchesChord(keyEvent({ key: "0", code: "Digit0", altKey: true }), {
    alt: true,
    key: "0",
  }),
  true,
);
// A non-digit chord must not be captured by the code branch.
assert.equal(
  eventMatchesChord(keyEvent({ key: "n", code: "Digit1", altKey: true }), {
    alt: true,
    key: "n",
  }),
  true,
);
// Alt+` (default work group): macOS ⌥+` is a dead key, so matching and
// recording must use the physical Backquote code.
assert.equal(
  eventMatchesChord(keyEvent({ key: "Dead", code: "Backquote", altKey: true }), {
    alt: true,
    key: "`",
  }),
  true,
);
assert.equal(
  eventMatchesChord(keyEvent({ key: "`", code: "Backquote", altKey: true }), {
    alt: true,
    key: "`",
  }),
  true,
);
assert.equal(
  eventMatchesChord(keyEvent({ key: "`", code: "Backquote", altKey: true }), {
    alt: true,
    key: "0",
  }),
  false,
);
assert.equal(
  chordFromEvent(keyEvent({ key: "¡", code: "Digit1", altKey: true }))?.key,
  "1",
);

assert.equal(
  isToggleCommandPanelShortcut(
    keyEvent({ ctrlKey: true, shiftKey: true, key: "P" }),
  ),
  true,
);
assert.equal(
  isGlobalSearchShortcut(keyEvent({ ctrlKey: true, shiftKey: true, key: "f" })),
  true,
);
assert.equal(
  eventMatchesChord(keyEvent({ ctrlKey: true, key: "l" }), {
    ctrl: true,
    key: "l",
  }),
  true,
);

assert.equal(typeof hasDomTextSelection, "function");
assert.equal(
  hasDomTextSelection(),
  false,
  "Node test env has no DOM selection",
);

let copyHandled = false;
registerShortcutHandler("navigator.copy", () => {
  copyHandled = true;
});
assert.equal(
  dispatchShortcutEvent(
    keyEvent({ ctrlKey: true, key: "c" }) as KeyboardEvent,
  ),
  true,
  "Ctrl+C without text selection still runs navigator.copy",
);
assert.equal(copyHandled, true);
unregisterShortcutHandler("navigator.copy");

const runtimeSrc = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "../src/shortcuts/runtime.ts"),
  "utf8",
);
assert.ok(
  runtimeSrc.includes("hasDomTextSelection"),
  "runtime must yield Ctrl+C/X when DOM text is selected",
);
assert.ok(runtimeSrc.includes("YIELD_TO_DOM_TEXT_SELECTION"));
assert.ok(
  runtimeSrc.includes("hasChordModifier"),
  "bare keys (backtick) must not fire from editable targets",
);

/* —— 反引号触发 + 功能/多键模型 + 用户覆盖层 —— */

assert.equal(
  winners.get(chordKey({ key: "`" }))?.id,
  "commandPanel.toggleBacktick",
  "bare backtick must toggle the Search Palette",
);
assert.equal(actionIdFor("commandPanel.toggleBacktick"), "commandPanel.toggle");
assert.deepEqual(effectiveChordLabelsForId("commandPanel.toggle"), [
  "Ctrl+Shift+P",
  "`",
]);
assert.equal(
  eventMatchesChord(keyEvent({ key: "`" }), { key: "`" }),
  true,
);
assert.deepEqual(
  chordFromEvent(keyEvent({ key: "`", ctrlKey: true, shiftKey: true })),
  { ctrl: true, shift: true, key: "`" },
);
assert.equal(
  chordFromEvent(keyEvent({ key: "Shift", shiftKey: true })),
  null,
  "modifier-only keydown must not be recorded",
);

const panelAction = SHORTCUT_ACTIONS.find((a) => a.id === "commandPanel.toggle");
assert.equal(panelAction?.configurable, true);
assert.deepEqual(panelAction?.aliases, ["commandPanel.toggleBacktick"]);
assert.equal(
  SHORTCUT_ACTIONS.find((a) => a.id === "navigator.jumpTab1")?.configurable,
  false,
  "Ctrl+1…9 is a fixed x+N group",
);
assert.equal(
  SHORTCUT_ACTIONS.find((a) => a.id === "workGroup.switch9")?.configurable,
  false,
  "Alt+1…9 is a fixed x+N group",
);
assert.ok(FIXED_CHORD_KEYS.has(chordKey({ alt: true, key: "1" })));
assert.ok(FIXED_CHORD_KEYS.has(chordKey({ ctrl: true, key: "9" })));
assert.equal(effectiveChordLabelsForId("navigator.back").length, 3);

// One function → many chords, and every chord reaches the canonical handler.
let backHandled = 0;
registerShortcutHandler("navigator.back", () => {
  backHandled += 1;
});
setActionChords("navigator.back", [{ key: "f7" }, { ctrl: true, key: "k" }]);
assert.deepEqual(effectiveChordLabelsForId("navigator.back"), [
  "F7",
  "Ctrl+K",
]);
assert.equal(ownerForChordKey("F7")?.id, "navigator.back");
assert.equal(dispatchShortcutEvent(keyEvent({ key: "F7" })), true);
assert.equal(dispatchShortcutEvent(keyEvent({ ctrlKey: true, key: "k" })), true);
assert.equal(backHandled, 2);
assert.equal(
  dispatchShortcutEvent(keyEvent({ altKey: true, key: "ArrowLeft" })),
  false,
  "an overridden action must stop firing its old default chords",
);
assert.equal(
  effectiveShortcutDefinitions().some((def) => def.id === "navigator.backBrowser"),
  false,
  "removed alias chords must disappear from the dispatch table",
);

assert.equal(validateNewChord("navigator.back", { alt: true, key: "1" }).reason, "reserved");
assert.equal(validateNewChord("navigator.back", { key: "escape" }).reason, "reserved");
assert.equal(validateNewChord("navigator.back", { key: "f7" }).reason, "duplicate");
assert.equal(
  validateNewChord("navigator.back", { ctrl: true, key: "c" }).reason,
  "conflict",
  "Ctrl+C belongs to navigator.copy",
);
assert.equal(
  validateNewChord("navigator.jumpTab1", { key: "f8" }).reason,
  "unknown",
);
assert.deepEqual(
  normalizeShortcutOverrides({
    "navigator.back": [{ key: "f7" }, { key: "f7" }, "bad"],
  }),
  { "navigator.back": [{ key: "f7" }] },
);

// A built-in bare key (Space = preview) must be re-bindable after removal.
setActionChords("navigator.previewBridge", []);
assert.equal(
  validateNewChord("navigator.previewBridge", { key: "space" }).ok,
  true,
);

const actionRows = listShortcutActionsForDisplay();
const backRow = actionRows.find((row) => row.id === "navigator.back");
assert.equal(backRow?.bindings.length, 2);
assert.equal(backRow?.isOverridden, true);
assert.equal(
  actionRows.find((row) => row.id === "navigator.jumpTab1")?.configurable,
  false,
);

resetAllShortcutOverrides();
assert.deepEqual(effectiveChordLabelsForId("navigator.back"), [
  "Alt+←",
  "Ctrl+Alt+←",
  "BrowserBack",
]);
unregisterShortcutHandler("navigator.back");

/**
 * macOS port: the registry stays Windows-first (`ctrl: true`), but a Cmd keydown
 * must match it and labels must read ⌘. Simulate the WKWebView user agent;
 * `platform_capabilities` has not answered in this Node process, so the
 * synchronous UA fallback is what we are testing here.
 */
const savedNavigator = Object.getOwnPropertyDescriptor(globalThis, "navigator");
Object.defineProperty(globalThis, "navigator", {
  value: {
    userAgent:
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_8_3) AppleWebKit/605.1.15 (KHTML, like Gecko)",
  },
  configurable: true,
});
try {
  assert.equal(
    eventMatchesChord(keyEvent({ metaKey: true, key: "c" }), {
      ctrl: true,
      key: "c",
    }),
    true,
    "Cmd+C must match a ctrl-registered copy chord on macOS",
  );
  assert.equal(
    eventMatchesChord(keyEvent({ ctrlKey: true, key: "c" }), {
      ctrl: true,
      key: "c",
    }),
    false,
    "physical Ctrl must not fire a Cmd chord on macOS",
  );
  assert.equal(
    eventMatchesChord(keyEvent({ metaKey: true, shiftKey: true, key: "P" }), {
      ctrl: true,
      shift: true,
      key: "p",
    }),
    true,
    "Cmd+Shift+P must still open the Command Panel on macOS",
  );
  assert.equal(formatChordLabel({ ctrl: true, shift: true, key: "p" }), "⌘+⇧+P");
  assert.equal(platformAriaKeys("Control+Shift+P"), "Meta+Shift+P");
  // chordKey must stay platform-neutral: dispatch maps and conflict checks
  // are keyed by it and must not change when the host switches to ⌘.
  assert.equal(chordKey({ ctrl: true, shift: true, key: "p" }), "Ctrl+Shift+P");
} finally {
  if (savedNavigator) {
    Object.defineProperty(globalThis, "navigator", savedNavigator);
  } else {
    delete (globalThis as { navigator?: unknown }).navigator;
  }
}

/**
 * Contract sync: `PlatformCapabilities` is serialized with camelCase, so the
 * TS shape must have exactly the fields declared in the Rust struct. This is
 * the lightweight stand-in for a generated IPC schema (see research doc 6.5).
 */
const platformSrc = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "../src-tauri/src/platform/mod.rs"),
  "utf8",
);
const structBody = platformSrc
  .split("pub struct PlatformCapabilities {")[1]
  ?.split("\n}")[0];
assert.ok(structBody, "Rust PlatformCapabilities struct not found");
const rustFields = [...structBody.matchAll(/pub (\w+):/g)]
  .map((match) => match[1].replace(/_([a-z])/g, (_, c: string) => c.toUpperCase()))
  .sort();
assert.deepEqual(
  Object.keys(NEUTRAL_PLATFORM).sort(),
  rustFields,
  "TS PlatformCapabilities must mirror the Rust struct exactly",
);

// 无 Tauri 后端（Node）时写链静默失败，flush 不抛错、不阻断会话内配置。
await flushShortcutOverridesPersistence();

console.log(
  `Shortcut registry checks OK (${BUILTIN_SHORTCUTS.length} builtins, ${SHORTCUT_ACTIONS.length} actions, config + backtick + conflict + match, macOS primary modifier, capability contract).`,
);
