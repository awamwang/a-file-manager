// SPDX-License-Identifier: GPL-3.0-or-later
/** Pure Recycle Bin → Navigator listing mapping. */

import { RECYCLE_PATH } from "./recyclePath.ts";

export type RecycleBinItem = {
  name: string;
  originalPath: string;
  recyclePath: string;
};

export type RecycleNavigatorEntry = {
  name: string;
  path: string;
  kind: "file";
  size: number;
  modifiedMs: null;
};

export type RecycleNavigatorListing = {
  path: string;
  entries: RecycleNavigatorEntry[];
  dirCount: number;
  fileCount: number;
};

export function recycleListingFromItems(
  items: RecycleBinItem[],
): RecycleNavigatorListing {
  const entries = items.map((item) => ({
    name: item.name,
    path: item.originalPath || item.recyclePath,
    kind: "file" as const,
    size: 0,
    modifiedMs: null,
  }));
  return {
    path: RECYCLE_PATH,
    entries,
    dirCount: 0,
    fileCount: entries.length,
  };
}
