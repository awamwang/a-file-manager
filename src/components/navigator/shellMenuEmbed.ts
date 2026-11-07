// SPDX-License-Identifier: GPL-3.0-or-later
/** Flatten Windows shell context-menu trees for AFM's single-level submenu (AIA-239). */

export type ShellContextMenuItem = {
  id: number;
  name: string;
  verb: string | null;
  children: ShellContextMenuItem[] | null;
};

export type FlattenedShellItem = {
  commandId: number;
  verb: string | null;
  label: string;
};

/** Recursively flatten nested shell parents into "Parent › Child" leaf rows. */
export function flattenShellItems(
  items: ShellContextMenuItem[],
  prefix = "",
): FlattenedShellItem[] {
  const out: FlattenedShellItem[] = [];
  for (const item of items) {
    const label = prefix ? `${prefix} › ${item.name}` : item.name;
    if (item.children?.length) {
      out.push(...flattenShellItems(item.children, label));
      continue;
    }
    if (!item.id && !item.verb) continue;
    out.push({
      commandId: item.id,
      verb: item.verb,
      label,
    });
  }
  return out;
}
