// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Navigator 列表虚拟项：组头 + 行展平，供大目录窗口化（含可折叠分组）。
 */

import type { NavigatorEntry } from "./navigatorBrowse.ts";
import type { NavigatorGroup } from "./navigatorGrouping.ts";

/** 组头行高（与 `.list-group-header` padding 对齐的近似值）。 */
export const LIST_GROUP_HEADER_HEIGHT_PX = 28;

export type ListVirtHeaderItem = {
  kind: "header";
  key: string;
  /** 原始桶 key（去 `h:` 前缀），供折叠切换。 */
  groupKey: string;
  label: string;
  count: number;
  collapsed: boolean;
};

export type ListVirtEntryItem<T extends NavigatorEntry = NavigatorEntry> = {
  kind: "entry";
  key: string;
  entry: T;
};

export type ListVirtItem<T extends NavigatorEntry = NavigatorEntry> =
  | ListVirtHeaderItem
  | ListVirtEntryItem<T>;

/** 未分组：仅条目。 */
export function flattenEntryItems<T extends NavigatorEntry>(
  entries: readonly T[],
): ListVirtItem<T>[] {
  return entries.map((entry) => ({
    kind: "entry" as const,
    key: entry.path,
    entry,
  }));
}

/**
 * 分组后展平为「组头 + 行」。折叠的组只保留组头（点击组头展开/折叠，
 * 折叠记忆仅会话级，见 navigatorViewMode.ts 的 collapsedGroupKeys）。
 */
export function flattenGroupedItems<T extends NavigatorEntry>(
  groups: readonly NavigatorGroup<T>[],
  collapsedKeys: ReadonlySet<string>,
): ListVirtItem<T>[] {
  const items: ListVirtItem<T>[] = [];
  for (const group of groups) {
    const collapsed = collapsedKeys.has(group.key);
    items.push({
      kind: "header",
      key: `h:${group.key}`,
      groupKey: group.key,
      label: group.label,
      count: group.entries.length,
      collapsed,
    });
    if (collapsed) continue;
    for (const entry of group.entries) {
      items.push({
        kind: "entry",
        key: entry.path,
        entry,
      });
    }
  }
  return items;
}

export function virtItemHeightPx(
  item: ListVirtItem,
  entryRowHeightPx: number,
): number {
  return item.kind === "header" ? LIST_GROUP_HEADER_HEIGHT_PX : entryRowHeightPx;
}

/**
 * 分组 + 网格视图的一行。
 *
 * 分组视图里组头横跨整行，组内条目按 `columns` 列流动 —— 与 Explorer 的
 * 「组头 + 组内图标网格」一致。虚拟化必须按**行**而不是按条目算，否则组内
 * 条目会被逐条当成一整行，图标就退化成一行一个。
 */
export type ListVirtGridRow = {
  /** 行高（px）。组头行用组头高，组内行用条目行高。 */
  height: number;
  /** 本行覆盖的虚拟项区间（半开）。 */
  startItem: number;
  endItem: number;
  /** 是否整行覆盖（组头）。组头必须与它所属组同批渲染，否则网格会错位。 */
  spansFullWidth: boolean;
};

/**
 * 把「组头 + 条目」的扁平序列切成网格视图的行。
 *
 * 组头独占一行；每组内的条目按 `columns` 个一行切分。
 */
export function buildGridRows<T extends NavigatorEntry>(
  items: readonly ListVirtItem<T>[],
  options: {
    /** 列数（>= 1）。必须与 CSS 实际排出的列数一致，否则行高对不上。 */
    columns: number;
    headerHeight: number;
    entryRowHeight: number;
  },
): ListVirtGridRow[] {
  const columns = Math.max(1, Math.trunc(options.columns));
  const rows: ListVirtGridRow[] = [];
  let index = 0;
  while (index < items.length) {
    const item = items[index]!;
    if (item.kind === "header") {
      rows.push({
        height: options.headerHeight,
        startItem: index,
        endItem: index + 1,
        spansFullWidth: true,
      });
      index += 1;
      continue;
    }
    // 组内条目是一段连续区间（下一个组头之前）。
    let end = index;
    while (end < items.length && items[end]!.kind === "entry") end += 1;
    for (let start = index; start < end; start += columns) {
      rows.push({
        height: options.entryRowHeight,
        startItem: start,
        endItem: Math.min(start + columns, end),
        spansFullWidth: false,
      });
    }
    index = end;
  }
  return rows;
}

export type ListGridVirtualWindow = {
  /** 是否启用窗口化（未达阈值时 false，切片为全集）。 */
  active: boolean;
  /** 行区间（半开）。 */
  startRow: number;
  endRow: number;
  /** 虚拟项区间（半开），已对齐到组边界。 */
  startIndex: number;
  endIndex: number;
  offsetBeforePx: number;
  offsetAfterPx: number;
};

/**
 * 分组 + 网格视图的行级窗口。
 *
 * 与 `computeVariableListVirtualWindow` 的区别有二：
 *  1. 高度按**行**给，而不是按条目给；
 *  2. 可见范围对齐到**组边界** —— 组头与它所属的组必须同批渲染，否则
 *     网格里组内条目会顶到上一行去（组头不再横跨整行）。
 */
export function computeGridListVirtualWindow(args: {
  rows: readonly ListVirtGridRow[];
  scrollTop: number;
  viewportHeight: number;
  overscanRows?: number;
  minRows?: number;
}): ListGridVirtualWindow {
  const total = args.rows.length;
  const minRows = args.minRows ?? 1;
  const overscan = args.overscanRows ?? 1;
  const viewportHeight = Math.max(0, args.viewportHeight);
  const scrollTop = Math.max(0, args.scrollTop);

  if (total <= minRows || viewportHeight <= 0) {
    return {
      active: false,
      startRow: 0,
      endRow: total,
      startIndex: 0,
      endIndex: args.rows.length
        ? args.rows[args.rows.length - 1]!.endItem
        : 0,
      offsetBeforePx: 0,
      offsetAfterPx: 0,
    };
  }

  const prefix = new Array<number>(total + 1);
  prefix[0] = 0;
  for (let i = 0; i < total; i += 1) {
    prefix[i + 1] = prefix[i]! + Math.max(0, args.rows[i]!.height);
  }
  const totalHeight = prefix[total]!;

  let startRow = 0;
  while (startRow < total && prefix[startRow + 1]! < scrollTop) {
    startRow += 1;
  }
  startRow = Math.max(0, startRow - overscan);

  const viewEnd = scrollTop + viewportHeight;
  let endRow = startRow;
  while (endRow < total && prefix[endRow]! < viewEnd) {
    endRow += 1;
  }
  endRow = Math.min(total, endRow + overscan);

  // 组边界对齐：组头行（spansFullWidth）开启一个组，组内行属于它。
  let groupStart = startRow;
  for (let i = startRow; i >= 0; i -= 1) {
    if (args.rows[i]!.spansFullWidth) {
      groupStart = i;
      break;
    }
  }
  let groupEnd = endRow;
  for (let i = endRow - 1; i >= 0; i -= 1) {
    if (args.rows[i]!.spansFullWidth) {
      groupEnd = i;
      break;
    }
  }

  const startIndex = args.rows[groupStart]?.startItem ?? 0;
  const endIndex =
    groupEnd > groupStart ? args.rows[groupEnd - 1]!.endItem : startIndex;

  return {
    active: true,
    startRow: groupStart,
    endRow: groupEnd,
    startIndex,
    endIndex,
    offsetBeforePx: prefix[groupStart]!,
    offsetAfterPx: Math.max(0, totalHeight - prefix[groupEnd]!),
  };
}

/** 分组网格里一个条目的视觉位置。 */
export type GroupedGridCell = {
  /** 视觉行索引（组头行也占一行，但没有条目）。 */
  row: number;
  /** 组内列（0-based）。 */
  col: number;
};

export type GroupedGridLayout = {
  /** 视觉行 → 该行的条目路径；组头行是空数组。 */
  rows: string[][];
  /** 路径 → 视觉位置。 */
  cells: Map<string, GroupedGridCell>;
};

/**
 * 分组网格的视觉布局。
 *
 * 每个组从新行开始（组头横跨整行），组内条目按列流动。方向键要按视觉行列
 * 走，不能按「条目序号 ÷ 列数」算 —— 组的条目数通常不是列数的整数倍，
 * 那样算跨组就会错位。
 */
export function buildGroupedGridLayout<T extends NavigatorEntry>(
  items: readonly ListVirtItem<T>[],
  gridRows: readonly ListVirtGridRow[],
): GroupedGridLayout {
  const rows: string[][] = [];
  const cells = new Map<string, GroupedGridCell>();
  for (const row of gridRows) {
    const paths: string[] = [];
    for (let index = row.startItem; index < row.endItem; index += 1) {
      const item = items[index];
      if (!item || item.kind !== "entry") continue;
      paths.push(item.entry.path);
      cells.set(item.entry.path, { row: rows.length, col: paths.length - 1 });
    }
    rows.push(paths);
  }
  return { rows, cells };
}

/**
 * 分组网格里的方向键移动。
 *
 * 左右在当前行内移动（不跨组）；上下跳到相邻的**有内容**的视觉行，跨组时
 * 取该行同一列、越界则取该行最后一个。返回 null 表示无处可去。
 */
export function nextGroupedGridPath(
  layout: GroupedGridLayout,
  currentPath: string,
  direction: "up" | "down" | "left" | "right",
): string | null {
  const cell = layout.cells.get(currentPath);
  if (!cell) return null;
  const { row, col } = cell;

  if (direction === "left") {
    if (col <= 0) return null;
    return layout.rows[row]![col - 1] ?? null;
  }
  if (direction === "right") {
    const line = layout.rows[row] ?? [];
    if (col + 1 >= line.length) return null;
    return line[col + 1] ?? null;
  }

  const step = direction === "up" ? -1 : 1;
  for (let r = row + step; r >= 0 && r < layout.rows.length; r += step) {
    const line = layout.rows[r] ?? [];
    if (!line.length) continue; // 组头行没有条目
    return line[Math.min(col, line.length - 1)] ?? null;
  }
  return null;
}
