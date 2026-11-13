// SPDX-License-Identifier: GPL-3.0-or-later
/** Recycle Bin 概念：虚拟路径 / 纯列表映射 / IPC 桥。 */

export { RECYCLE_PATH, normalizeRecycleKey, isRecyclePath } from "./path.ts";
export {
  recycleListingFromItems,
  type RecycleBinItem,
  type RecycleNavigatorEntry,
  type RecycleNavigatorListing,
} from "./listing.ts";
export {
  listRecycleBinItems,
  restoreRecyclePaths,
  loadRecycleListing,
} from "./bin.ts";
export {
  RECYCLE_SHELL_TARGET,
  openRecycleBinInSystem,
  type OpenRecycleBinResult,
} from "./openSystem.ts";
