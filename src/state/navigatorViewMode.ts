// SPDX-License-Identifier: GPL-3.0-or-later
/** Windows-style Navigator view modes (Explorer-aligned). */

import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { hydrateSettings, rememberSettingsDocument } from "./settingsHydrate.ts";
import type { GroupDirection, NavigatorGroupField } from "./navigatorGrouping.ts";
import {
  ICON_VIEW_ZOOM_ORDER,
  nudgeIconViewMode as nudgeIconViewModePure,
} from "./iconViewZoom.ts";

export type { GroupDirection, NavigatorGroupField };

export type NavigatorViewMode =
  | "iconsExtraLarge"
  | "iconsLarge"
  | "iconsMedium"
  | "iconsSmall"
  | "list"
  | "details"
  | "tiles"
  | "content";

export const NAVIGATOR_VIEW_MODES: readonly NavigatorViewMode[] = [
  "iconsExtraLarge",
  "iconsLarge",
  "iconsMedium",
  "iconsSmall",
  "list",
  "details",
  "tiles",
  "content",
] as const;

export { ICON_VIEW_ZOOM_ORDER };

/** Shell icon / thumbnail edge length for each view. */
export const VIEW_ICON_SIZE: Record<NavigatorViewMode, number> = {
  iconsExtraLarge: 256,
  iconsLarge: 96,
  iconsMedium: 64,
  iconsSmall: 48,
  list: 16,
  details: 16,
  tiles: 48,
  content: 32,
};

/** AIA-232: Ctrl+wheel icon zoom step. */
export function nudgeIconViewMode(
  current: NavigatorViewMode,
  direction: 1 | -1,
): NavigatorViewMode {
  return nudgeIconViewModePure(current, direction, ICON_VIEW_ZOOM_ORDER);
}

export const navigatorViewMode = ref<NavigatorViewMode>("details");

/** v17（ADR 0030）：分组字段；"none" = 不分组。 */
export const navigatorGroupBy = ref<NavigatorGroupField>("none");

/** 分组桶排序方向（组头间顺序；组内行序仍由 sortColumn/sortDirection 决定）。 */
export const navigatorGroupDirection = ref<GroupDirection>("asc");

/** 折叠组的会话级记忆（key = 桶 key；跨目录保留，不做持久化）。 */
export const collapsedGroupKeys = ref<Set<string>>(new Set());

const GROUP_FIELDS: readonly NavigatorGroupField[] = [
  "none",
  "name",
  "size",
  "modified",
  "type",
];

/** 分组菜单字段（Win11 顺序：None/名称/修改日期/类型/大小）。 */
export const NAVIGATOR_GROUP_FIELDS: readonly NavigatorGroupField[] = [
  "none",
  "name",
  "modified",
  "type",
  "size",
];

export function groupFieldLabelKey(field: NavigatorGroupField): string {
  switch (field) {
    case "none":
      return "chrome.groupNone";
    case "name":
      return "chrome.sortName";
    case "size":
      return "chrome.sortSize";
    case "modified":
      return "chrome.sortModified";
    case "type":
      return "chrome.sortType";
  }
}

function coerceGroupField(value: unknown): NavigatorGroupField {
  return typeof value === "string" && (GROUP_FIELDS as readonly string[]).includes(value)
    ? (value as NavigatorGroupField)
    : "none";
}

/** 切换分组字段时的 Windows 默认方向：name/type 升序，size/modified 降序。 */
export function defaultDirectionForGroupField(
  field: NavigatorGroupField,
): GroupDirection {
  return field === "size" || field === "modified" ? "desc" : "asc";
}

/** 会话级折叠切换（组头点击）。 */
export function toggleGroupCollapsed(key: string): void {
  const next = new Set(collapsedGroupKeys.value);
  if (next.has(key)) {
    next.delete(key);
  } else {
    next.add(key);
  }
  collapsedGroupKeys.value = next;
}

export const isIconView = computed(() =>
  navigatorViewMode.value.startsWith("icons"),
);

export const isDetailsView = computed(() => navigatorViewMode.value === "details");

export const isListView = computed(() => navigatorViewMode.value === "list");

export const isTilesView = computed(() => navigatorViewMode.value === "tiles");

export const isContentView = computed(() => navigatorViewMode.value === "content");

let loadPromise: Promise<void> | null = null;

function coerceMode(value: unknown): NavigatorViewMode {
  if (typeof value === "string" && (NAVIGATOR_VIEW_MODES as readonly string[]).includes(value)) {
    return value as NavigatorViewMode;
  }
  return "details";
}

/** Sync apply from already-hydrated settings (cold-start critical path). */
export function applyNavigatorViewModeFromValues(values: Record<string, unknown>): void {
  navigatorViewMode.value = coerceMode(values["core.navigatorViewMode"]);
  navigatorGroupBy.value = coerceGroupField(values["core.navigatorGroupBy"]);
  navigatorGroupDirection.value =
    values["core.navigatorGroupDirection"] === "desc" ? "desc" : "asc";
}

export async function loadNavigatorViewMode(): Promise<NavigatorViewMode> {
  if (!loadPromise) {
    loadPromise = (async () => {
      try {
        const doc = await hydrateSettings();
        applyNavigatorViewModeFromValues(doc.values ?? {});
      } catch {
        navigatorViewMode.value = "details";
        navigatorGroupBy.value = "none";
        navigatorGroupDirection.value = "asc";
      }
    })().finally(() => {
      loadPromise = null;
    });
  }
  await loadPromise;
  return navigatorViewMode.value;
}

export async function setNavigatorViewMode(mode: NavigatorViewMode): Promise<void> {
  navigatorViewMode.value = mode;
  try {
    const doc = await invoke<{ version: number; values: Record<string, unknown> }>(
      "settings_set",
      {
        patch: { "core.navigatorViewMode": mode },
      },
    );
    rememberSettingsDocument(doc);
  } catch {
    // Keep local mode even if persist fails; next load may revert.
  }
}

/** 切换分组字段：方向重置为该字段默认，折叠记忆清空，两个键一并持久化。 */
export async function setNavigatorGroupBy(
  field: NavigatorGroupField,
): Promise<void> {
  navigatorGroupBy.value = field;
  navigatorGroupDirection.value = defaultDirectionForGroupField(field);
  collapsedGroupKeys.value = new Set();
  try {
    const doc = await invoke<{ version: number; values: Record<string, unknown> }>(
      "settings_set",
      {
        patch: {
          "core.navigatorGroupBy": field,
          "core.navigatorGroupDirection": navigatorGroupDirection.value,
        },
      },
    );
    rememberSettingsDocument(doc);
  } catch {
    // Keep local field even if persist fails.
  }
}

export async function setNavigatorGroupDirection(
  direction: GroupDirection,
): Promise<void> {
  navigatorGroupDirection.value = direction;
  try {
    const doc = await invoke<{ version: number; values: Record<string, unknown> }>(
      "settings_set",
      {
        patch: { "core.navigatorGroupDirection": direction },
      },
    );
    rememberSettingsDocument(doc);
  } catch {
    // Keep local direction even if persist fails.
  }
}

export function viewModeLabelKey(mode: NavigatorViewMode): string {
  switch (mode) {
    case "iconsExtraLarge":
      return "chrome.viewIconsExtraLarge";
    case "iconsLarge":
      return "chrome.viewIconsLarge";
    case "iconsMedium":
      return "chrome.viewIconsMedium";
    case "iconsSmall":
      return "chrome.viewIconsSmall";
    case "list":
      return "chrome.viewList";
    case "details":
      return "chrome.viewDetails";
    case "tiles":
      return "chrome.viewTiles";
    case "content":
      return "chrome.viewContent";
  }
}
