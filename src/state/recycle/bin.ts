// SPDX-License-Identifier: GPL-3.0-or-later
/** Recycle Bin listing / restore bridge. */

import { invoke } from "@tauri-apps/api/core";
import type { FileOpsResult } from "../fileClipboard.ts";
import { beginLongTask, finishLongTask } from "../taskCenter.ts";
import {
  recycleListingFromItems,
  type RecycleBinItem,
  type RecycleNavigatorListing,
} from "./listing.ts";

export type { RecycleBinItem };
export { recycleListingFromItems };

function parentDir(path: string): string {
  const norm = path.replace(/\//g, "\\").replace(/\\+$/, "");
  const idx = norm.lastIndexOf("\\");
  return idx > 0 ? norm.slice(0, idx) : norm;
}

export async function listRecycleBinItems(): Promise<RecycleBinItem[]> {
  return invoke<RecycleBinItem[]>("file_ops_recycle_list");
}

/** 批量还原走长任务中心；Rust 侧 async + spawn_blocking，避免阻塞 UI。 */
export async function restoreRecyclePaths(paths: string[]): Promise<FileOpsResult> {
  if (!paths.length) {
    return { ok: true, paths: [], error: null };
  }
  const taskId = beginLongTask("restore", paths, parentDir(paths[0] ?? ""));
  try {
    const result = await invoke<FileOpsResult>("file_ops_restore", { paths });
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

export async function loadRecycleListing(): Promise<RecycleNavigatorListing> {
  const items = await listRecycleBinItems();
  return recycleListingFromItems(items);
}
