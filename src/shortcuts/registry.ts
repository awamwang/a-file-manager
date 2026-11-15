// SPDX-License-Identifier: GPL-3.0-or-later

import {
  chordKey,
  chordToAria,
  formatChordLabel,
  isReservedBareKey,
} from "./chord.ts";
import { getShortcutOverrides, shortcutOverridesVersion } from "./config.ts";
import type {
  ShortcutAction,
  ShortcutActionRow,
  ShortcutChord,
  ShortcutConflict,
  ShortcutDefinition,
  ShortcutOverrides,
  ShortcutValidationResult,
} from "./types.ts";

/**
 * Day-1 built-in global shortcuts (beyond ad-hoc Command Panel / Discovery helpers).
 * Chord conflicts are detected at load; first definition wins for dispatch.
 */
export const BUILTIN_SHORTCUTS: ShortcutDefinition[] = [
  {
    id: "commandPanel.toggle",
    labelKey: "shortcuts.items.commandPanelToggle",
    chord: { ctrl: true, shift: true, key: "p" },
    scope: "global",
  },
  /** Alias: bare backtick also toggles the Search Palette (one action → many chords). */
  {
    id: "commandPanel.toggleBacktick",
    labelKey: "shortcuts.items.commandPanelToggle",
    chord: { key: "`" },
    scope: "global",
  },
  {
    id: "discovery.globalSearch",
    labelKey: "shortcuts.items.discoveryGlobalSearch",
    chord: { ctrl: true, shift: true, key: "f" },
    scope: "global",
  },
  {
    id: "navigator.folderSearch",
    labelKey: "shortcuts.items.navigatorFolderSearch",
    chord: { ctrl: true, key: "e" },
    scope: "global",
  },
  /** Alias: F3 focuses the same current-folder search box. */
  {
    id: "navigator.folderSearchF3",
    labelKey: "shortcuts.items.navigatorFolderSearch",
    chord: { key: "f3" },
    scope: "global",
  },
  {
    id: "navigator.goUp",
    labelKey: "shortcuts.items.navigatorGoUp",
    chord: { alt: true, key: "arrowup" },
    scope: "global",
  },
  {
    id: "navigator.back",
    labelKey: "shortcuts.items.navigatorBack",
    chord: { alt: true, key: "arrowleft" },
    scope: "global",
  },
  {
    id: "navigator.forward",
    labelKey: "shortcuts.items.navigatorForward",
    chord: { alt: true, key: "arrowright" },
    scope: "global",
  },
  /** Alias: Ctrl+Alt+←/→ (exact chord match; Alt-only does not fire when Ctrl is held). */
  {
    id: "navigator.backCtrlAlt",
    labelKey: "shortcuts.items.navigatorBack",
    chord: { ctrl: true, alt: true, key: "arrowleft" },
    scope: "global",
  },
  {
    id: "navigator.forwardCtrlAlt",
    labelKey: "shortcuts.items.navigatorForward",
    chord: { ctrl: true, alt: true, key: "arrowright" },
    scope: "global",
  },
  /** Alias: 键鼠 BrowserBack / BrowserForward（AppCommand），绑定 Navigator 历史而非 WebView。 */
  {
    id: "navigator.backBrowser",
    labelKey: "shortcuts.items.navigatorBack",
    chord: { key: "browserback" },
    scope: "global",
  },
  {
    id: "navigator.forwardBrowser",
    labelKey: "shortcuts.items.navigatorForward",
    chord: { key: "browserforward" },
    scope: "global",
  },
  {
    id: "navigator.newTab",
    labelKey: "shortcuts.items.navigatorNewTab",
    chord: { ctrl: true, key: "t" },
    scope: "global",
  },
  {
    id: "navigator.nextTabGroup",
    labelKey: "shortcuts.items.navigatorNextTabGroup",
    chord: { ctrl: true, key: "tab" },
    scope: "global",
  },
  {
    id: "navigator.prevTabGroup",
    labelKey: "shortcuts.items.navigatorPrevTabGroup",
    chord: { ctrl: true, shift: true, key: "tab" },
    scope: "global",
  },
  {
    id: "navigator.closeTab",
    labelKey: "shortcuts.items.navigatorCloseTab",
    chord: { ctrl: true, key: "w" },
    scope: "navigator",
  },
  {
    id: "navigator.copy",
    labelKey: "shortcuts.items.navigatorCopy",
    chord: { ctrl: true, key: "c" },
    scope: "global",
  },
  {
    id: "navigator.cut",
    labelKey: "shortcuts.items.navigatorCut",
    chord: { ctrl: true, key: "x" },
    scope: "global",
  },
  {
    id: "navigator.paste",
    labelKey: "shortcuts.items.navigatorPaste",
    chord: { ctrl: true, key: "v" },
    scope: "global",
  },
  {
    id: "navigator.rename",
    labelKey: "shortcuts.items.navigatorRename",
    chord: { key: "f2" },
    scope: "global",
  },
  {
    id: "navigator.copyPath",
    labelKey: "shortcuts.items.navigatorCopyPath",
    chord: { ctrl: true, shift: true, key: "c" },
    scope: "global",
  },
  {
    id: "navigator.newFolder",
    labelKey: "shortcuts.items.navigatorNewFolder",
    chord: { ctrl: true, shift: true, key: "n" },
    scope: "global",
  },
  {
    id: "navigator.newFile",
    labelKey: "shortcuts.items.navigatorNewFile",
    chord: { ctrl: true, alt: true, key: "n" },
    scope: "global",
  },
  {
    id: "navigator.refresh",
    labelKey: "shortcuts.items.navigatorRefresh",
    chord: { key: "f5" },
    scope: "global",
  },
  {
    id: "navigator.delete",
    labelKey: "shortcuts.items.navigatorDelete",
    chord: { ctrl: true, key: "d" },
    scope: "global",
  },
  /** Alias: Delete key (exact chord; keeps OS-familiar delete). */
  {
    id: "navigator.deleteKey",
    labelKey: "shortcuts.items.navigatorDelete",
    chord: { key: "delete" },
    scope: "global",
  },
  {
    id: "navigator.deletePermanent",
    labelKey: "shortcuts.items.navigatorDeletePermanent",
    chord: { shift: true, key: "delete" },
    scope: "global",
  },
  {
    id: "navigator.selectAll",
    labelKey: "shortcuts.items.navigatorSelectAll",
    chord: { ctrl: true, key: "a" },
    scope: "global",
  },
  {
    id: "navigator.undo",
    labelKey: "shortcuts.items.navigatorUndo",
    chord: { ctrl: true, key: "z" },
    scope: "global",
  },
  /**
   * AIA Preview Bridge: Space → selectedPaths → Peek CLI / QuickLook pipe /
   * built-in pane. Editable targets (address / rename) are skipped by runtime.
   */
  {
    id: "navigator.previewBridge",
    labelKey: "shortcuts.items.navigatorPreviewBridge",
    chord: { key: "space" },
    scope: "navigator",
  },
  {
    id: "navigator.properties",
    labelKey: "shortcuts.items.navigatorProperties",
    chord: { alt: true, key: "enter" },
    scope: "navigator",
  },
  {
    id: "navigator.previewPane",
    labelKey: "shortcuts.items.navigatorPreviewPane",
    chord: { alt: true, key: "p" },
    scope: "navigator",
  },
  {
    /** AIA-235: Details pane (not Properties Alt+Enter). */
    id: "navigator.detailsPane",
    labelKey: "shortcuts.items.navigatorDetailsPane",
    chord: { alt: true, shift: true, key: "p" },
    scope: "navigator",
  },
  {
    id: "chrome.focusAddress",
    labelKey: "shortcuts.items.focusAddress",
    chord: { ctrl: true, key: "l" },
    scope: "global",
  },
  {
    id: "chrome.focusAddressAltD",
    labelKey: "shortcuts.items.focusAddress",
    chord: { alt: true, key: "d" },
    scope: "global",
  },
  {
    id: "chrome.focusAddressF4",
    labelKey: "shortcuts.items.focusAddress",
    chord: { key: "f4" },
    scope: "global",
  },
  {
    id: "chrome.focusCycle",
    labelKey: "shortcuts.items.focusCycle",
    chord: { key: "f6" },
    scope: "global",
  },
  {
    id: "chrome.focusCycleReverse",
    labelKey: "shortcuts.items.focusCycleReverse",
    chord: { shift: true, key: "f6" },
    scope: "global",
  },
  {
    id: "view.settings",
    labelKey: "shortcuts.items.openSettings",
    chord: { ctrl: true, key: "," },
    scope: "global",
  },
  {
    id: "view.shortcuts",
    labelKey: "shortcuts.items.openShortcutList",
    chord: { ctrl: true, shift: true, key: "/" },
    scope: "global",
  },
  /**
   * Open/focus Work Groups page (same id as Command Panel `view.work-groups`).
   * Listed in Settings → Shortcuts; remap when user overrides land.
   */
  {
    id: "view.work-groups",
    labelKey: "shortcuts.items.openWorkGroups",
    chord: { alt: true, key: "n" },
    scope: "global",
  },
];

for (let tabNumber = 1; tabNumber <= 9; tabNumber += 1) {
  BUILTIN_SHORTCUTS.push({
    id: `navigator.jumpTab${tabNumber}`,
    labelKey: `shortcuts.items.navigatorJumpTab${tabNumber}`,
    chord: { ctrl: true, key: String(tabNumber) },
    scope: "navigator",
  });
}

/** Alt+1…9：按当前排序切换到对应工作组（超出列表长度时忽略）。 */
for (let workGroupNumber = 1; workGroupNumber <= 9; workGroupNumber += 1) {
  BUILTIN_SHORTCUTS.push({
    id: `workGroup.switch${workGroupNumber}`,
    labelKey: `shortcuts.items.workGroupSwitch${workGroupNumber}`,
    chord: { alt: true, key: String(workGroupNumber) },
    scope: "global",
  });
}

export function findShortcutConflicts(
  defs: ShortcutDefinition[] = BUILTIN_SHORTCUTS,
): ShortcutConflict[] {
  const byChord = new Map<string, string[]>();
  for (const def of defs) {
    const key = chordKey(def.chord);
    const list = byChord.get(key) ?? [];
    list.push(def.id);
    byChord.set(key, list);
  }
  const conflicts: ShortcutConflict[] = [];
  for (const [key, ids] of byChord) {
    if (ids.length > 1) {
      conflicts.push({ chordLabel: key, ids: [...ids] });
    }
  }
  return conflicts;
}

/** First registered id wins for a chord. */
export function resolveDispatchMap(
  defs: ShortcutDefinition[] = BUILTIN_SHORTCUTS,
): Map<string, ShortcutDefinition> {
  const map = new Map<string, ShortcutDefinition>();
  for (const def of defs) {
    const key = chordKey(def.chord);
    if (!map.has(key)) {
      map.set(key, def);
    }
  }
  return map;
}

export function listShortcutsForDisplay(
  defs: ShortcutDefinition[] = BUILTIN_SHORTCUTS,
): Array<ShortcutDefinition & { chordLabel: string; blockedBy?: string }> {
  const winners = resolveDispatchMap(defs);
  return defs.map((def) => {
    const key = chordKey(def.chord);
    const winner = winners.get(key);
    return {
      ...def,
      chordLabel: formatChordLabel(def.chord),
      blockedBy: winner && winner.id !== def.id ? winner.id : undefined,
    };
  });
}

/* -------------------------------------------------------------------------- *
 * 功能（action）模型：一个功能 → 1..N 个组合键
 * -------------------------------------------------------------------------- */

/** x+N 键组固定不可配置：Ctrl+1…9 切标签、Alt+1…9 切工作组。 */
const FIXED_ACTION_ID = /^(?:navigator\.jumpTab[1-9]|workGroup\.switch[1-9])$/;

/**
 * Legacy alias definitions share a `labelKey`; fold them into one action so the
 * Settings page and tooltips speak in terms of functions, not chord ids.
 */
function buildActions(defs: ShortcutDefinition[]): ShortcutAction[] {
  const byLabel = new Map<string, ShortcutDefinition[]>();
  for (const def of defs) {
    const group = byLabel.get(def.labelKey) ?? [];
    group.push(def);
    byLabel.set(def.labelKey, group);
  }
  return [...byLabel.values()].map((group) => {
    const primary = group[0];
    return {
      id: primary.id,
      labelKey: primary.labelKey,
      aliases: group.slice(1).map((def) => def.id),
      scope: primary.scope,
      allowInEditable: primary.allowInEditable,
      configurable: !FIXED_ACTION_ID.test(primary.id),
    };
  });
}

export const SHORTCUT_ACTIONS: ShortcutAction[] = buildActions(BUILTIN_SHORTCUTS);

/** Every legacy alias id resolves back to its canonical action (and handler) id. */
const ACTION_ID_BY_DEFINITION_ID = new Map<string, string>();
for (const action of SHORTCUT_ACTIONS) {
  ACTION_ID_BY_DEFINITION_ID.set(action.id, action.id);
  for (const alias of action.aliases) {
    ACTION_ID_BY_DEFINITION_ID.set(alias, action.id);
  }
}

export function actionIdFor(id: string): string {
  return ACTION_ID_BY_DEFINITION_ID.get(id) ?? id;
}

export function findShortcutAction(actionId: string): ShortcutAction | null {
  return SHORTCUT_ACTIONS.find((action) => action.id === actionId) ?? null;
}

export function defaultChordsForAction(actionId: string): ShortcutChord[] {
  const action = findShortcutAction(actionId);
  if (!action) return [];
  const ids = new Set([action.id, ...action.aliases]);
  return BUILTIN_SHORTCUTS.filter((def) => ids.has(def.id)).map((def) => def.chord);
}

/** Chords protected from remapping (the fixed x+N groups). */
export const FIXED_CHORD_KEYS: ReadonlySet<string> = new Set(
  SHORTCUT_ACTIONS.filter((action) => !action.configurable).flatMap((action) =>
    defaultChordsForAction(action.id).map(chordKey),
  ),
);

/** The *final* chords of a function, after applying user overrides. */
export function effectiveChordsForId(
  id: string,
  overrides: Readonly<ShortcutOverrides> = getShortcutOverrides(),
): ShortcutChord[] {
  // Reading the version ref makes every render-time caller (aria attributes,
  // tooltips, settings rows) reactive to a remap without extra plumbing.
  void shortcutOverridesVersion.value;
  const actionId = actionIdFor(id);
  const action = findShortcutAction(actionId);
  if (!action) return [];
  if (!action.configurable) return defaultChordsForAction(actionId);
  const stored = overrides[actionId];
  return stored === undefined ? defaultChordsForAction(actionId) : stored;
}

/** Final, formatted chord labels — the only source tooltips should display. */
export function effectiveChordLabelsForId(id: string): string[] {
  return effectiveChordsForId(id).map(formatChordLabel);
}

/** ARIA-friendly final chords (space separated, Control+Shift+P grammar). */
export function ariaKeyshortcutsForId(id: string): string {
  return effectiveChordsForId(id).map(chordToAria).join(" ");
}

/**
 * Flatten actions back into dispatch definitions. With no overrides this is the
 * untouched builtin table (keeps legacy ids, tests and first-wins behaviour).
 * Overrides replace *all* chords of an action, so removed aliases stop firing.
 */
export function effectiveShortcutDefinitions(
  overrides: Readonly<ShortcutOverrides> = getShortcutOverrides(),
): ShortcutDefinition[] {
  if (Object.keys(overrides).length === 0) return BUILTIN_SHORTCUTS;
  const result: ShortcutDefinition[] = [];
  for (const action of SHORTCUT_ACTIONS) {
    const stored = action.configurable ? overrides[action.id] : undefined;
    const chords = stored === undefined ? defaultChordsForAction(action.id) : stored;
    chords.forEach((chord, index) => {
      result.push({
        id:
          index === 0
            ? action.id
            : (action.aliases[index - 1] ?? `${action.id}#${index}`),
        labelKey: action.labelKey,
        chord,
        scope: action.scope,
        allowInEditable: index === 0 ? action.allowInEditable : undefined,
      });
    });
  }
  return result;
}

/** One row per function, with every final binding and its conflict owner. */
export function listShortcutActionsForDisplay(
  overrides: Readonly<ShortcutOverrides> = getShortcutOverrides(),
): ShortcutActionRow[] {
  const winners = resolveDispatchMap(effectiveShortcutDefinitions(overrides));
  return SHORTCUT_ACTIONS.map((action) => {
    const stored = action.configurable ? overrides[action.id] : undefined;
    const chords =
      stored === undefined ? defaultChordsForAction(action.id) : stored;
    return {
      id: action.id,
      labelKey: action.labelKey,
      configurable: action.configurable,
      isOverridden: action.configurable && stored !== undefined,
      bindings: chords.map((chord) => {
        const label = formatChordLabel(chord);
        const winner = winners.get(label);
        const ownerId = winner ? actionIdFor(winner.id) : undefined;
        return {
          chord,
          label,
          blockedBy: ownerId && ownerId !== action.id ? ownerId : undefined,
        };
      }),
    };
  });
}

/** Reject reserved / duplicate / conflicting chords before persisting them. */
export function validateNewChord(
  actionId: string,
  chord: ShortcutChord,
  overrides: Readonly<ShortcutOverrides> = getShortcutOverrides(),
): ShortcutValidationResult {
  const action = findShortcutAction(actionId);
  if (!action || !action.configurable) return { ok: false, reason: "unknown" };
  const key = chordKey(chord);
  if (FIXED_CHORD_KEYS.has(key) || isReservedBareKey(chord)) {
    return { ok: false, reason: "reserved" };
  }
  if (effectiveChordsForId(actionId, overrides).some((c) => chordKey(c) === key)) {
    return { ok: false, reason: "duplicate" };
  }
  const winner = resolveDispatchMap(effectiveShortcutDefinitions(overrides)).get(key);
  if (winner) {
    const ownerId = actionIdFor(winner.id);
    if (ownerId !== actionId) return { ok: false, reason: "conflict", ownerId };
  }
  return { ok: true };
}
