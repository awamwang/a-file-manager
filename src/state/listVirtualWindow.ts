// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Navigator 列表窗口化：大目录只挂载视口附近行，避免全量 DOM。
 * 纯模块（无 Vue），供 Node strip-types 检查与视图共用。
 */

import {
  estimateRowHeightPx,
  LIST_ROW_HEIGHT_PX,
  type PlaceholderViewMode,
} from "./listPlaceholders.ts";
import { estimateIconColumns } from "./navigatorKeyboardNav.ts";

/** 低于该条数时不做窗口化，避免小目录额外滚动同步开销。 */
export const LIST_VIRTUALIZE_MIN_ENTRIES = 80;
/** 视口上下各多挂载的行数，减轻快速滚动露白。 */
export const LIST_VIRTUAL_OVERSCAN_ROWS = 8;
/**
 * 缩略图请求相对可视行的额外行数（按行高换算，而非固定 px）。
 * 过小会导致滚动手感冷；过大则慢速外部缩略图器时取消粒度变差。
 */
export const THUMB_REQUEST_OVERSCAN_ROWS = 1;
/** 视口高度尚未量到时，按列数估算的兜底行数，避免空请求窗。 */
export const THUMB_REQUEST_FALLBACK_ROWS = 12;
/** 与 `--fluent-list-row-height-compact` 对齐。 */
export const COMPACT_LIST_ROW_HEIGHT_PX = 24;
/** `.list--tiles` 的 `gap`；图标视图仅 `column-gap`（`row-gap: 0`）。 */
export const LIST_GRID_GAP_PX = 8;
/** `.list--tiles` 的 `minmax(14rem, 1fr)` @ 16px。 */
export const TILES_COL_MIN_PX = 224;

export type ListVirtualWindow = {
  /** 是否启用窗口化（未达阈值或入参无效时为 false，切片为全集）。 */
  active: boolean;
  startIndex: number;
  /** 半开区间终点。 */
  endIndex: number;
  offsetBeforePx: number;
  offsetAfterPx: number;
};

export function estimateListColumns(
  mode: PlaceholderViewMode | string,
  containerWidth: number,
): number {
  if (typeof mode === "string" && mode.startsWith("icons")) {
    return estimateIconColumns(containerWidth, mode);
  }
  if (mode === "tiles") {
    if (!Number.isFinite(containerWidth) || containerWidth <= 0) return 1;
    return Math.max(1, Math.floor(containerWidth / TILES_COL_MIN_PX));
  }
  return 1;
}

export function resolveListRowHeightPx(
  mode: PlaceholderViewMode,
  compactListDensity: boolean,
): number {
  if (compactListDensity && (mode === "list" || mode === "details")) {
    return COMPACT_LIST_ROW_HEIGHT_PX;
  }
  return estimateRowHeightPx(mode);
}

function rowStridePx(rowHeight: number, columns: number, mode: string): number {
  // 图标格行间无 gap，步进即行高；平铺/多列仍计入 LIST_GRID_GAP_PX。
  if (mode.startsWith("icons")) return rowHeight;
  const usesGap = columns > 1 || mode === "tiles";
  return rowHeight + (usesGap ? LIST_GRID_GAP_PX : 0);
}

/**
 * 按滚动位置计算应挂载的条目窗口（列对齐），并给出前后占位高度。
 */
export function computeListVirtualWindow(args: {
  total: number;
  scrollTop: number;
  viewportHeight: number;
  rowHeight: number;
  columns: number;
  mode: string;
  overscanRows?: number;
  minTotal?: number;
}): ListVirtualWindow {
  const total = Math.max(0, Math.floor(args.total));
  const columns = Math.max(1, Math.floor(args.columns) || 1);
  const minTotal = args.minTotal ?? LIST_VIRTUALIZE_MIN_ENTRIES;
  const overscanRows = args.overscanRows ?? LIST_VIRTUAL_OVERSCAN_ROWS;
  const rowHeight = Math.max(1, args.rowHeight);
  const viewportHeight = Math.max(0, args.viewportHeight);
  const scrollTop = Math.max(0, args.scrollTop);

  if (total <= minTotal || viewportHeight <= 0) {
    return {
      active: false,
      startIndex: 0,
      endIndex: total,
      offsetBeforePx: 0,
      offsetAfterPx: 0,
    };
  }

  const stride = rowStridePx(rowHeight, columns, args.mode);
  const totalRows = Math.ceil(total / columns);
  const firstVisibleRow = Math.floor(scrollTop / stride);
  const visibleRows = Math.ceil(viewportHeight / stride) + 1;
  const startRow = Math.max(0, firstVisibleRow - overscanRows);
  const endRow = Math.min(totalRows, firstVisibleRow + visibleRows + overscanRows);
  const startIndex = startRow * columns;
  const endIndex = Math.min(total, endRow * columns);
  const offsetBeforePx = startRow * stride;
  const remainingRows = Math.max(0, totalRows - endRow);
  const offsetAfterPx = remainingRows * stride;

  return {
    active: true,
    startIndex,
    endIndex,
    offsetBeforePx,
    offsetAfterPx,
  };
}

export type ThumbRequestWindow = {
  startIndex: number;
  /** 半开区间终点。 */
  endIndex: number;
  /** 窗口内条目数（含 overscan）。 */
  visibleCount: number;
};

/**
 * 按滚动几何计算缩略图/壳图标应请求的条目窗口。
 * 始终覆盖整屏可视行（+ 少量按行高 overscan），不与 IPC 批大小挂钩。
 */
export function computeThumbRequestWindow(args: {
  total: number;
  scrollTop: number;
  viewportHeight: number;
  rowHeight: number;
  columns: number;
  mode: string;
  overscanRows?: number;
  fallbackRows?: number;
}): ThumbRequestWindow {
  const total = Math.max(0, Math.floor(args.total));
  const columns = Math.max(1, Math.floor(args.columns) || 1);
  const overscanRows = args.overscanRows ?? THUMB_REQUEST_OVERSCAN_ROWS;
  const fallbackRows = args.fallbackRows ?? THUMB_REQUEST_FALLBACK_ROWS;
  const rowHeight = Math.max(1, args.rowHeight);
  const viewportHeight = Math.max(0, args.viewportHeight);
  const scrollTop = Math.max(0, args.scrollTop);

  if (total === 0) {
    return { startIndex: 0, endIndex: 0, visibleCount: 0 };
  }

  // 列表尚未 layout 出高度时，先按兜底行数请求，避免空窗把已排队项全部 cancel。
  if (viewportHeight <= 0) {
    const endIndex = Math.min(total, columns * Math.max(1, fallbackRows));
    return { startIndex: 0, endIndex, visibleCount: endIndex };
  }

  const stride = rowStridePx(rowHeight, columns, args.mode);
  const totalRows = Math.ceil(total / columns);
  const firstVisibleRow = Math.floor(scrollTop / stride);
  const visibleRows = Math.max(1, Math.ceil(viewportHeight / stride) + 1);
  const startRow = Math.max(0, firstVisibleRow - overscanRows);
  const endRow = Math.min(totalRows, firstVisibleRow + visibleRows + overscanRows);
  const startIndex = startRow * columns;
  const endIndex = Math.min(total, endRow * columns);
  return {
    startIndex,
    endIndex,
    visibleCount: endIndex - startIndex,
  };
}

/**
 * 变高项窗口化（组头 + 行）。`heights[i]` 为第 i 项高度。
 * 前后 spacer 用精确前缀和，避免分组场景退回全量 DOM。
 */
export function computeVariableListVirtualWindow(args: {
  heights: readonly number[];
  scrollTop: number;
  viewportHeight: number;
  overscanItems?: number;
  minTotal?: number;
}): ListVirtualWindow {
  const total = args.heights.length;
  const minTotal = args.minTotal ?? LIST_VIRTUALIZE_MIN_ENTRIES;
  const overscan = args.overscanItems ?? LIST_VIRTUAL_OVERSCAN_ROWS;
  const viewportHeight = Math.max(0, args.viewportHeight);
  const scrollTop = Math.max(0, args.scrollTop);

  if (total <= minTotal || viewportHeight <= 0) {
    return {
      active: false,
      startIndex: 0,
      endIndex: total,
      offsetBeforePx: 0,
      offsetAfterPx: 0,
    };
  }

  const prefix = new Array<number>(total + 1);
  prefix[0] = 0;
  for (let i = 0; i < total; i += 1) {
    prefix[i + 1] = prefix[i]! + Math.max(0, args.heights[i] ?? 0);
  }
  const totalHeight = prefix[total]!;

  let startIndex = 0;
  while (startIndex < total && prefix[startIndex + 1]! < scrollTop) {
    startIndex += 1;
  }
  startIndex = Math.max(0, startIndex - overscan);

  const viewEnd = scrollTop + viewportHeight;
  let endIndex = startIndex;
  while (endIndex < total && prefix[endIndex]! < viewEnd) {
    endIndex += 1;
  }
  endIndex = Math.min(total, endIndex + overscan);

  return {
    active: true,
    startIndex,
    endIndex,
    offsetBeforePx: prefix[startIndex]!,
    offsetAfterPx: Math.max(0, totalHeight - prefix[endIndex]!),
  };
}

/** 便于测试：标准密度 details 行高。 */
export function standardDetailsRowHeight(): number {
  return LIST_ROW_HEIGHT_PX;
}
