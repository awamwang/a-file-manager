// SPDX-License-Identifier: GPL-3.0-or-later
/** Single-step undo for Explorer-like file ops (Ctrl+Z). */

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  deleteEntries,
  renameEntry,
  transferEntries,
  type FileOpsResult,
} from "../fileClipboard.ts";
import { restoreRecyclePaths } from "../recycle/bin.ts";
import {
  dirsAffectedByUndo,
  isContaminated,
  moveUndoRecord,
  popRecord,
  pushRecord,
  renameUndoRecord,
  type Fingerprint,
  type UndoRecord,
} from "./stack.ts";
import { showToast } from "../toast.ts";
import { i18n } from "../../i18n/index.ts";

export type { UndoRecord };
export { dirsAffectedByUndo };

const stack = ref<UndoRecord[]>([]);

function pushUndo(record: UndoRecord): void {
  stack.value = pushRecord(stack.value, record);
}

/** UN-008: capture the record-time baseline so a later Ctrl+Z can refuse when a
 * foreign program touched the target. Fire-and-forget: while the capture is in
 * flight (or when it fails) the record simply has no baseline and undo proceeds. */
function attachFingerprints(record: UndoRecord, paths: string[]): void {
  void invoke<Array<Fingerprint | null>>("file_ops_fingerprints", { paths })
    .then((fingerprints) => {
      if (record.kind === "copy" || record.kind === "create") {
        record.fingerprints = fingerprints;
      }
    })
    .catch(() => undefined);
}

/** UN-008: re-fingerprint now and compare against the record-time baseline. */
async function contaminationCheck(
  record: UndoRecord,
  paths: string[],
): Promise<boolean> {
  if (record.kind !== "copy" && record.kind !== "create") return false;
  const stored =
    record.kind === "copy" || record.kind === "create"
      ? record.fingerprints
      : undefined;
  if (!stored) return false;
  try {
    const current = await invoke<Array<Fingerprint | null>>(
      "file_ops_fingerprints",
      { paths },
    );
    return isContaminated(stored, current);
  } catch {
    return false;
  }
}

/** UN-008 refusal: localized toast, no forced delete, no silent skip. The
 * contaminated record is popped so later Ctrl+Z still reach earlier records
 * (UN-006 LIFO discipline: one poisoned record must not strand the stack). */
function contaminatedRefusal(): FileOpsResult & { undone: UndoRecord | null } {
  showToast(i18n.global.t("fileOps.undoContaminated"), "error");
  return { ok: false, paths: [], error: null, undone: null };
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
      // UN-008: refuse when a foreign program modified the created items since
      // the copy — deleting them would clobber those edits.
      if (await contaminationCheck(record, record.createdPaths)) {
        stack.value = popRecord(stack.value).stack;
        return contaminatedRefusal();
      }
      result = await deleteEntries(record.createdPaths, false);
      break;
    case "create":
      // UN-008: same guard for created files/folders.
      if (await contaminationCheck(record, [record.path])) {
        stack.value = popRecord(stack.value).stack;
        return contaminatedRefusal();
      }
      result = await deleteEntries([record.path], true);
      break;
    case "delete":
      // 批量还原走 restoreRecyclePaths（任务中心 + Rust async worker）。
      result = await restoreRecyclePaths(record.originalPaths);
      // 回收站里已无对应条目（已被还原/清空）：该记录失效，
      // 静默弹出并按“无可撤销”处理，避免报错且不阻塞更早的撤销。
      if (result.ok && !result.paths.length) {
        stack.value = popRecord(stack.value).stack;
        return { ok: false, paths: [], error: "Nothing to undo", undone: null };
      }
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
  const record: UndoRecord = { kind: "copy", createdPaths: [...createdPaths] };
  pushUndo(record);
  attachFingerprints(record, record.createdPaths);
}

export function recordCreateUndo(path: string): void {
  const record: UndoRecord = { kind: "create", path };
  pushUndo(record);
  attachFingerprints(record, [record.path]);
}

export function recordDeleteUndo(originalPaths: string[]): void {
  if (!originalPaths.length) return;
  pushUndo({ kind: "delete", originalPaths: [...originalPaths] });
}
