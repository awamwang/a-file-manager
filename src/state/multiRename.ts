// SPDX-License-Identifier: GPL-3.0-or-later
/** Explorer-like multi-select rename with sequence numbers. */

export type RenamePlanItem = {
  path: string;
  newName: string;
};

function splitName(name: string): { stem: string; ext: string } {
  const dot = name.lastIndexOf(".");
  if (dot <= 0 || dot === name.length - 1) {
    return { stem: name, ext: "" };
  }
  return { stem: name.slice(0, dot), ext: name.slice(dot) };
}

/**
 * Build rename targets for a multi-selection.
 * First item uses `baseName` (or `baseName` + first ext if base has no ext).
 * Later items become `stem (n).ext` with n starting at 2.
 */
export function planMultiRename(
  paths: string[],
  baseName: string,
): RenamePlanItem[] {
  const trimmed = baseName.trim();
  if (!paths.length || !trimmed) return [];
  const base = splitName(trimmed);
  return paths.map((path, index) => {
    const current = path.replace(/[/\\]+$/, "");
    const slash = Math.max(current.lastIndexOf("\\"), current.lastIndexOf("/"));
    const oldName = slash >= 0 ? current.slice(slash + 1) : current;
    const old = splitName(oldName);
    const ext = base.ext || old.ext;
    const stem = base.stem || old.stem;
    const newName =
      index === 0 ? `${stem}${ext}` : `${stem} (${index + 1})${ext}`;
    return { path, newName };
  });
}
