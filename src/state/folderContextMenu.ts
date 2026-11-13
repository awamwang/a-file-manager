// SPDX-License-Identifier: GPL-3.0-or-later
/** Request a folder-entry context menu (breadcrumb / background) outside the list rows. */

import { ref } from "vue";
import type { NavigatorEntry } from "./navigatorBrowse.ts";

export type FolderContextMenuRequest = {
  entry: NavigatorEntry;
  x: number;
  y: number;
  /** Bumps so identical coords still retrigger the host watcher. */
  seq: number;
};

export const folderContextMenuRequest = ref<FolderContextMenuRequest | null>(null);

let seq = 0;

/** Synthetic directory entry for a filesystem path (breadcrumb segment / current folder). */
export function dirEntryFromPath(path: string): NavigatorEntry {
  const original = path.trim();
  const display = original.replace(/\//g, "\\");
  if (/^[A-Za-z]:\\?$/.test(display)) {
    const drive = display.slice(0, 2);
    return {
      name: drive,
      path: original || `${drive}\\`,
      kind: "dir",
      size: 0,
    };
  }
  const trimmed = display.replace(/\\+$/, "");
  const parts = trimmed.split("\\").filter(Boolean);
  const name = parts[parts.length - 1] ?? trimmed;
  return {
    name,
    path: original,
    kind: "dir",
    size: 0,
  };
}

/** Open the same folder verbs menu used for a directory list entry. */
export function requestFolderContextMenu(path: string, x: number, y: number): void {
  const trimmed = path.trim();
  if (!trimmed) return;
  folderContextMenuRequest.value = {
    entry: dirEntryFromPath(trimmed),
    x,
    y,
    seq: ++seq,
  };
}
