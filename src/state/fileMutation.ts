// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * FIL-61：文件变更完成后的统一刷新钩子。
 * 定向作废 listing 缓存并强制重列当前目录；不做高频轮询。
 */

import {
  invalidateListingParents,
  reloadListingAfterMutation,
} from "./navigatorBrowse";
import { invalidateListingCache } from "./navigatorListingCache";
import { dirsAffectedByUndo, type UndoRecord } from "./fileUndo/stack";

export type FileMutationRefreshOptions = {
  /** 条目路径：作废其父目录缓存（剪切源等）。 */
  invalidateEntryPaths?: string[];
  /** 目录路径：直接作废该目录缓存（移动目标、undo 对端等）。 */
  invalidateDirs?: string[];
  includeSecondary?: boolean;
  /** 默认 true；仅作废缓存、暂不重列时设为 false。 */
  reloadCurrent?: boolean;
};

/** 操作成功后的统一收尾：作废相关缓存并重列当前（及可选副窗格）。 */
export async function afterFileMutation(
  options: FileMutationRefreshOptions = {},
): Promise<void> {
  if (options.invalidateEntryPaths?.length) {
    invalidateListingParents(options.invalidateEntryPaths);
  }
  for (const dir of options.invalidateDirs ?? []) {
    const trimmed = dir.trim();
    if (trimmed) invalidateListingCache(trimmed);
  }
  if (options.reloadCurrent === false) return;
  await reloadListingAfterMutation({
    includeSecondary: options.includeSecondary,
  });
}

/** 撤销成功后按 UndoRecord 作废源/目标父目录并重列。 */
export async function afterUndoMutation(
  record: UndoRecord,
  options?: { includeSecondary?: boolean },
): Promise<void> {
  await afterFileMutation({
    invalidateDirs: dirsAffectedByUndo(record),
    includeSecondary: options?.includeSecondary,
  });
}

export { dirsAffectedByUndo };