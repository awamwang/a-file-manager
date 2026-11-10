// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Shared folder-drop apply path for list / tree / tabs / places (AIA-240 + AIA-242 + AIA-250).
 * Keeps volume + modifier rules and undo/refresh side effects in one place.
 */

import {
  dataTransferLooksExternal,
  pathsFromDataTransfer,
} from "./paths";
import { dropTransferMode, type DropMode } from "./mode";
import { transferEntries } from "../fileClipboard";
import { recordCopyUndo, recordMoveUndo } from "../fileUndo";
import { clearSelection, selectedPaths } from "../navigatorSelection";
import { refreshActiveListing } from "../navigatorBrowse";

export type FolderDropCommit = {
  ok: boolean;
  mode?: DropMode;
  count?: number;
  error?: string;
};

/** Resolve copy|move|link for dragover / drop (empty external sources → modifiers only). */
export function previewDropMode(
  event: DragEvent,
  destPath: string,
  sourcePaths: string[],
): DropMode {
  const external = dataTransferLooksExternal(event.dataTransfer);
  if (external && sourcePaths.length === 0) {
    if (event.altKey) return "link";
    if (event.shiftKey) return "move";
    return "copy";
  }
  return dropTransferMode(sourcePaths, destPath, {
    ctrl: event.ctrlKey,
    shift: event.shiftKey,
    alt: event.altKey,
  });
}

/** Paths used for dragover cursor preview (in-app selection; empty for external). */
export function previewSourcePaths(event: DragEvent): string[] {
  if (dataTransferLooksExternal(event.dataTransfer)) return [];
  return [...selectedPaths.value];
}

/**
 * Accept a folder drop target: preventDefault, set dropEffect, return mode.
 * Returns null when the destination must not accept (e.g. self-drop on selected row).
 */
export function acceptFolderDragOver(
  event: DragEvent,
  destPath: string,
  options: { rejectIfSelected?: boolean } = {},
): DropMode | null {
  if (options.rejectIfSelected && selectedPaths.value.includes(destPath)) {
    return null;
  }
  event.preventDefault();
  event.stopPropagation();
  const mode = previewDropMode(event, destPath, previewSourcePaths(event));
  if (event.dataTransfer) event.dataTransfer.dropEffect = mode;
  return mode;
}

/** Apply inbound / in-app drop into `destPath`; refresh listing + undo bookkeeping. */
export async function commitFolderDrop(
  event: DragEvent,
  destPath: string,
): Promise<FolderDropCommit> {
  event.preventDefault();
  event.stopPropagation();
  const paths = pathsFromDataTransfer(event.dataTransfer).filter((p) => p !== destPath);
  if (!paths.length) return { ok: false, error: "empty" };

  try {
    const mode = previewDropMode(event, destPath, paths);
    const result = await transferEntries(paths, destPath, mode);
    if (!result.ok) {
      return { ok: false, mode, error: result.error ?? "failed" };
    }
    if (mode === "copy") recordCopyUndo(result.paths);
    else if (mode === "move") recordMoveUndo(paths, result.paths);
    clearSelection();
    refreshActiveListing();
    return { ok: true, mode, count: result.paths.length };
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

export function noticeKeyForDropMode(mode: DropMode): string {
  if (mode === "copy") return "navigator.copiedInto";
  if (mode === "move") return "navigator.movedInto";
  return "navigator.linkedInto";
}
