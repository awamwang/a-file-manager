// SPDX-License-Identifier: GPL-3.0-or-later
/** Single-step undo for Explorer-like file ops (Ctrl+Z). */

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  deleteEntries,
  renameEntry,
  transferEntries,
  type FileOpsResult,
} from "../fileClipboard";
import {
  moveUndoRecord,
  popRecord,
  pushRecord,
  renameUndoRecord,
  type UndoRecord,
} from "./stack";

export type { UndoRecord };

const stack = ref<UndoRecord[]>([]);

function pushUndo(record: UndoRecord): void {
  stack.value = pushRecord(stack.value, record);
}

export async function undoLast(): Promise<FileOpsResult & { undone: UndoRecord | null }> {
  const record = stack.value[stack.value.length - 1];
  if (!record) {
    return { ok: false, paths: [], error: "Nothing to undo", undone: null };
  }

  let result: FileOpsResult;
  switch (record.kind) {
    case "rename":
      result = await renameEntry(record.currentPath, record.previousName);
      break;
    case "move": {
      const moved: string[] = [];
      for (const item of record.items) {
        const one = await transferEntries(
          [item.currentPath],
          item.originalDir,
          "move",
        );
        if (!one.ok) {
          return { ...one, undone: null };
        }
        moved.push(...one.paths);
      }
      result = { ok: true, paths: moved, error: null };
      break;
    }
    case "copy":
      result = await deleteEntries(record.createdPaths, false);
      break;
    case "create":
      result = await deleteEntries([record.path], true);
      break;
    case "delete":
      result = await invoke<FileOpsResult>("file_ops_restore", {
        paths: record.originalPaths,
      });
      break;
  }

  if (result.ok) {
    stack.value = popRecord(stack.value).stack;
    return { ...result, undone: record };
  }
  return { ...result, undone: null };
}

export function recordRenameUndo(fromPath: string, toPath: string): void {
  pushUndo(renameUndoRecord(fromPath, toPath));
}

export function recordMoveUndo(sources: string[], destinations: string[]): void {
  pushUndo(moveUndoRecord(sources, destinations));
}

export function recordCopyUndo(createdPaths: string[]): void {
  if (!createdPaths.length) return;
  pushUndo({ kind: "copy", createdPaths: [...createdPaths] });
}

export function recordCreateUndo(path: string): void {
  pushUndo({ kind: "create", path });
}

export function recordDeleteUndo(originalPaths: string[]): void {
  if (!originalPaths.length) return;
  pushUndo({ kind: "delete", originalPaths: [...originalPaths] });
}
