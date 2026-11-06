// SPDX-License-Identifier: GPL-3.0-or-later
/** Details Type column labels (Explorer-like file type). */

export function entryExtension(name: string): string {
  const base = name.replace(/[/\\]+$/, "");
  const dot = base.lastIndexOf(".");
  if (dot <= 0 || dot === base.length - 1) return "";
  return base.slice(dot + 1).toLowerCase();
}

/** Stable type key for grouping / sorting. */
export function entryTypeKey(kind: string, name: string): string {
  if (kind === "dir") return "dir";
  const ext = entryExtension(name);
  return ext ? `file:${ext}` : "file";
}

/**
 * Human type label. `folderLabel` / `fileLabel` / `typedFile` come from i18n.
 * `typedFile(ext)` receives the uppercased extension (e.g. "TXT").
 */
export function entryTypeLabel(
  kind: string,
  name: string,
  labels: {
    folder: string;
    file: string;
    typedFile: (ext: string) => string;
  },
): string {
  if (kind === "dir") return labels.folder;
  const ext = entryExtension(name);
  if (!ext) return labels.file;
  return labels.typedFile(ext.toUpperCase());
}
