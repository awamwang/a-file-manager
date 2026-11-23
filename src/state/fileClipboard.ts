// SPDX-License-Identifier: GPL-3.0-or-later
/** Shell file clipboard (CF_HDROP) so copy/cut/paste works with File Explorer. */

import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { resetNameConflictBatch, resolveNameConflictChoice } from "./nameConflict.ts";
import {
  isRetryableTransferErrorClass,
  resolveTransferErrorChoice,
  type TransferErrorClass,
} from "./transferError.ts";
import { normalizePathKey } from "./pathKey.ts";
import { dedupeDescendantSources } from "./transferDedup.ts";
import { beginLongTask, finishLongTask, type LongTaskKind } from "./taskCenter.ts";

export type ClipboardMode = "copy" | "cut";

export type FileOpsResult = {
  ok: boolean;
  paths: string[];
  error: string | null;
  /** LC-006/LC-007/LC-008 per-item error class from the backend
   *  (`file_ops/mod.rs::classify_transfer_error`): `in-use` / `access-denied` /
   *  `disk-full` for a retryable single-item failure, else null/undefined for
   *  successes and hard refusals. */
  errorClass?: string | null;
  /** LC-005: true when the user cancelled a tracked copy/move. Partial work was
   *  rolled back (plain copy delete / plain move back); distinct from a failure. */
  cancelled?: boolean;
};

export const clipboardPaths = ref<string[]>([]);
export const clipboardMode = ref<ClipboardMode | null>(null);

export const hasClipboard = computed(
  () => clipboardPaths.value.length > 0 && clipboardMode.value !== null,
);

/** 剪切态路径键集合；非 cut 时为 null，供列表项半透明判断复用。 */
const cutPathKeys = computed(() => {
  if (clipboardMode.value !== "cut" || !clipboardPaths.value.length) return null;
  return new Set(
    clipboardPaths.value.map((path) => normalizePathKey(path)).filter(Boolean),
  );
});

/** 该项是否处于待剪切（cut clipboard）状态。 */
export function isClipboardCutPath(path: string): boolean {
  const keys = cutPathKeys.value;
  if (!keys) return false;
  const key = normalizePathKey(path);
  return Boolean(key) && keys.has(key);
}

export function setClipboard(paths: string[], mode: ClipboardMode): void {
  clipboardPaths.value = [...paths];
  clipboardMode.value = mode;
  void invoke("file_ops_clipboard_set", { paths, mode }).catch(() => undefined);
}

export function clearClipboard(): void {
  clipboardPaths.value = [];
  clipboardMode.value = null;
  void invoke("file_ops_clipboard_clear").catch(() => undefined);
}

/** Read CF_HDROP so paste works for copies made in File Explorer. */
export async function refreshClipboardFromSystem(): Promise<void> {
  try {
    const result = await invoke<{ paths: string[]; mode: ClipboardMode | null }>(
      "file_ops_clipboard_get",
    );
    clipboardPaths.value = result.paths ?? [];
    clipboardMode.value =
      result.mode === "copy" || result.mode === "cut" ? result.mode : null;
    if (!clipboardPaths.value.length) clipboardMode.value = null;
  } catch {
    /* keep the last local snapshot if the shell clipboard is busy */
  }
}

function samePath(left: string, right: string): boolean {
  const norm = (value: string) =>
    value.trim().replace(/\//g, "\\").replace(/\\+$/, "").toLowerCase();
  return norm(left) === norm(right);
}

function baseName(path: string): string {
  const norm = path.replace(/\//g, "\\").replace(/\\+$/, "");
  const idx = norm.lastIndexOf("\\");
  return idx >= 0 ? norm.slice(idx + 1) : norm;
}

export async function pasteClipboard(destDir: string): Promise<FileOpsResult> {
  await refreshClipboardFromSystem();
  const paths = [...clipboardPaths.value];
  const mode = clipboardMode.value;
  if (!paths.length || !mode) {
    return { ok: false, paths: [], error: "Clipboard is empty" };
  }
  const result = await transferEntries(paths, destDir, mode === "copy" ? "copy" : "move");
  if (result.ok && mode === "cut") clearClipboard();
  return result;
}

export async function transferEntries(
  sources: string[],
  destDir: string,
  mode: "copy" | "move" | "link",
): Promise<FileOpsResult> {
  // CF-003 / TR-014: a checked "Apply to all" choice lives only for this batch.
  // Reset at the top so the previous paste's remembered choice never leaks here.
  resetNameConflictBatch();
  // SEL-007 / SEL-011: drop a source already contained in another source so a folder and
  // one of its descendants are processed once, not double-handled in this transfer.
  sources = dedupeDescendantSources(sources);
  const command =
    mode === "copy"
      ? "file_ops_copy"
      : mode === "move"
        ? "file_ops_move"
        : "file_ops_create_shortcuts";
  // 复制/移动属于长任务：进入右下角任务列表并上报进度（FIL-70）。
  const trackedKind: LongTaskKind | null = mode === "copy" || mode === "move" ? mode : null;
  const taskId =
    trackedKind && sources.length ? beginLongTask(trackedKind, sources, destDir) : null;
  const finish = (
    ok: boolean,
    error: string | null,
    count: number,
  ): void => {
    if (taskId) finishLongTask(taskId, ok, error, count, transferred);
  };
  const transferred: string[] = [];
  for (const source of sources) {
    const name =
      mode === "link"
        ? `${baseName(source)} - Shortcut.lnk`
        : baseName(source);
    const destFile = `${destDir.replace(/[\\/]+$/, "")}\\${name}`;
    // 同目录粘贴：copy 仍走重名确认弹窗（「保留两者」可生成 - Copy 副本）；
    // cut 保持 Explorer 式无操作，link 的目标名不同不受影响。
    const selfPaste = samePath(source, destFile);
    let conflict = "keep";
    if (!selfPaste || mode === "copy") {
      const info = await invoke<{ exists: boolean; typeMismatch: boolean }>(
        "file_ops_name_conflict_info",
        { destDir, name, source },
      );
      const exists = selfPaste || info.exists;
      if (exists) {
        // TR-013 / CF-006: a file↔folder name clash gets a stronger warning in
        // the dialog; folder-into-folder merge is not a mismatch.
        const choice = await resolveNameConflictChoice(name, !selfPaste && info.typeMismatch);
        if (choice === "cancel") {
          finish(false, "Cancelled", transferred.length);
          return { ok: false, paths: transferred, error: "Cancelled" };
        }
        conflict = choice;
      }
    }
    const result = await invoke<FileOpsResult>(command, {
      sources: [source],
      destDir,
      conflict,
      taskId,
    });
    // LC-006 / LC-007 / LC-008 (已拍板 2026-11-22): a per-item failure with a
    // recognized error class opens the Explorer-style Retry / Skip / Abort dialog
    // for THIS item. A hard refusal (no class — folder-into-itself, volume root,
    // validation) keeps the existing whole-batch abort.
    let outcome = result;
    while (!outcome.ok && isRetryableTransferErrorClass(outcome.errorClass)) {
      const errorClass = outcome.errorClass as TransferErrorClass;
      const choice = await resolveTransferErrorChoice(errorClass, name);
      if (choice === "retry") {
        outcome = await invoke<FileOpsResult>(command, {
          sources: [source],
          destDir,
          conflict,
          taskId,
        });
        continue;
      }
      if (choice === "skip") {
        // Skip this item, leave its source untouched, and continue the batch.
        outcome = { ok: true, paths: [], error: null };
        break;
      }
      // Abort the whole batch. LC-005 rollback semantics: the failed item's
      // partial destination was already scrubbed by safe_io staging, and for a
      // copy we drop the copies completed in this batch (source stays intact).
      // An aborted/rolled-back transfer is the transfer's own teardown and is
      // never pushed onto the undo stack (UN-009) — callers only record undo on a
      // successful return, which this path does not produce.
      if (mode === "copy" && transferred.length) {
        await deleteEntries(transferred, true);
      }
      finish(false, outcome.error, transferred.length);
      return { ok: false, paths: [], error: outcome.error ?? "Aborted" };
    }
    if (!outcome.ok) {
      finish(false, outcome.error, transferred.length);
      return outcome;
    }
    transferred.push(...outcome.paths);
  }
  finish(true, null, transferred.length);
  return { ok: true, paths: transferred, error: null };
}

export async function renameEntry(
  path: string,
  newName: string,
  replace = false,
): Promise<FileOpsResult> {
  return invoke<FileOpsResult>("file_ops_rename", { path, newName, replace });
}

/** Whether `name` already exists directly under `destDir` (case-insensitive on Windows). */
export async function nameExists(destDir: string, name: string): Promise<boolean> {
  try {
    return await invoke<boolean>("file_ops_name_exists", { destDir, name });
  } catch {
    return false;
  }
}

export async function createFolder(
  parent: string,
  name: string,
): Promise<FileOpsResult> {
  return invoke<FileOpsResult>("file_ops_create_dir", { parent, name });
}

export async function createFile(
  parent: string,
  name: string,
): Promise<FileOpsResult> {
  return invoke<FileOpsResult>("file_ops_create_file", { parent, name });
}

export async function deleteEntries(
  paths: string[],
  permanent = false,
): Promise<FileOpsResult> {
  const command = permanent ? "file_ops_delete_permanent" : "file_ops_delete";
  if (!paths.length) {
    return { ok: true, paths: [], error: null };
  }
  // 批量删除进入任务中心；Rust 侧已 async + spawn_blocking。
  const taskId = beginLongTask("delete", paths, "");
  try {
    const result = await invoke<FileOpsResult>(command, { paths });
    finishLongTask(
      taskId,
      result.ok,
      result.error,
      result.paths.length,
      result.paths,
    );
    return result;
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    finishLongTask(taskId, false, message, 0);
    throw err;
  }
}

export async function pathsWithoutRecycle(paths: string[]): Promise<string[]> {
  return invoke<string[]>("file_ops_can_recycle", { paths });
}
