// SPDX-License-Identifier: GPL-3.0-or-later
/** Recycle Bin 概念：虚拟路径 / 纯列表映射 / IPC 桥。 */

export { RECYCLE_PATH, normalizeRecycleKey, isRecyclePath } from "./path";
export {
  recycleListingFromItems,
  type RecycleBinItem,
  type RecycleNavigatorEntry,
  type RecycleNavigatorListing,
} from "./listing";
export {
  listRecycleBinItems,
  restoreRecyclePaths,
  loadRecycleListing,
} from "./bin";
