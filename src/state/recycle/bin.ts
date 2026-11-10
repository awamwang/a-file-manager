// SPDX-License-Identifier: GPL-3.0-or-later
/** Recycle Bin listing / restore bridge. */

import { invoke } from "@tauri-apps/api/core";
import type { FileOpsResult } from "../fileClipboard";
import {
  recycleListingFromItems,
  type RecycleBinItem,
  type RecycleNavigatorListing,
} from "./listing";

export type { RecycleBinItem };
export { recycleListingFromItems };

export async function listRecycleBinItems(): Promise<RecycleBinItem[]> {
  return invoke<RecycleBinItem[]>("file_ops_recycle_list");
}

export async function restoreRecyclePaths(paths: string[]): Promise<FileOpsResult> {
  return invoke<FileOpsResult>("file_ops_restore", { paths });
}

export async function loadRecycleListing(): Promise<RecycleNavigatorListing> {
  const items = await listRecycleBinItems();
  return recycleListingFromItems(items);
}
