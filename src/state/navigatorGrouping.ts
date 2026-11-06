// SPDX-License-Identifier: GPL-3.0-or-later
/** Optional Explorer-like grouping for Navigator listings. */

import { entryTypeKey, entryTypeLabel } from "./entryType.ts";

export type GroupableEntry = {
  name: string;
  path: string;
  kind: string;
};

export type NavigatorGroup<T extends GroupableEntry = GroupableEntry> = {
  key: string;
  label: string;
  entries: T[];
};

export type GroupLabelFns = {
  folder: string;
  file: string;
  typedFile: (ext: string) => string;
};

export function groupEntriesByType<T extends GroupableEntry>(
  entries: T[],
  labels: GroupLabelFns,
): NavigatorGroup<T>[] {
  const buckets = new Map<string, NavigatorGroup<T>>();
  for (const entry of entries) {
    const key = entryTypeKey(entry.kind, entry.name);
    let group = buckets.get(key);
    if (!group) {
      group = {
        key,
        label: entryTypeLabel(entry.kind, entry.name, labels),
        entries: [],
      };
      buckets.set(key, group);
    }
    group.entries.push(entry);
  }
  return [...buckets.values()].sort((a, b) => {
    if (a.key === "dir") return -1;
    if (b.key === "dir") return 1;
    return a.label.localeCompare(b.label, undefined, { sensitivity: "base" });
  });
}
