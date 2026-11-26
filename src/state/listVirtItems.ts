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
