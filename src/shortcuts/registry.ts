// SPDX-License-Identifier: GPL-3.0-or-later

import { chordKey, formatChordLabel } from "./chord.ts";
import type { ShortcutConflict, ShortcutDefinition } from "./types.ts";

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
    id: "chrome.focusAddress",
    labelKey: "shortcuts.items.focusAddress",
    chord: { ctrl: true, key: "l" },
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
