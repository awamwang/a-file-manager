// SPDX-License-Identifier: GPL-3.0-or-later

/** Modifier + main key chord (Windows-first). */
export type ShortcutChord = {
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  meta?: boolean;
  /** Normalized lowercase key (`p`, `f`, `t`, `l`, `,`, `arrowup`, `escape`, …). */
  key: string;
};

export type ShortcutScope = "global" | "navigator";

export type ShortcutDefinition = {
  id: string;
  /** i18n key under `shortcuts.items.<id>` or full path. */
  labelKey: string;
  chord: ShortcutChord;
  scope: ShortcutScope;
  /** When true, still fire while focus is in an editable field (rare). */
  allowInEditable?: boolean;
};

export type ShortcutConflict = {
  chordLabel: string;
  ids: string[];
};

export type ShortcutHandler = (event: KeyboardEvent) => void | Promise<void>;
