// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Viewport-sized Navigator loading placeholders (AIA-267).
 * Pure module (no Vue) so Node strip-types checks can import it.
 */

export type PlaceholderViewMode =
  | "iconsExtraLarge"
  | "iconsLarge"
  | "iconsMedium"
  | "iconsSmall"
  | "list"
  | "details"
  | "tiles"
  | "content";

/** Keep in sync with `VIEW_ICON_SIZE` in navigatorViewMode.ts. */
export const PLACEHOLDER_ICON_SIZE: Record<PlaceholderViewMode, number> = {
  iconsExtraLarge: 256,
  iconsLarge: 96,
  iconsMedium: 64,
  iconsSmall: 48,
  list: 16,
  details: 16,
  tiles: 48,
  content: 32,
};

/** Keep in sync with `ICON_COL_MIN_PX` in navigatorKeyboardNav.ts. */
const ICON_COL_MIN_PX: Record<string, number> = {
  iconsExtraLarge: 288,
  iconsLarge: 120,
  iconsMedium: 96,
  iconsSmall: 80,
};

/** Matches `--fluent-list-row-height-standard`. */
export const LIST_ROW_HEIGHT_PX = 32;
/** Sticky details header row. */
export const DETAILS_HEADER_HEIGHT_PX = 32;
/** Approximate `.list--tiles .row--tile` min-height (4.5rem @ 16px). */
export const TILE_ROW_HEIGHT_PX = 72;
/** Content view row with meta line. */
export const CONTENT_ROW_HEIGHT_PX = 48;
/** 图标格下方标签区 + 内边距（与 `--nav-icon-caption-extra` / `.row--icon` 行高同步）。 */
export const ICON_CAPTION_EXTRA_PX = 64;
/** Cap so a huge window does not mount hundreds of inert nodes. */
export const PLACEHOLDER_COUNT_CAP = 120;
/** Fold fallback when the list surface has not been measured yet. */
export const PLACEHOLDER_FALLBACK_COUNT = 24;

function estimateIconColumns(containerWidth: number, mode: string): number {
  if (!mode.startsWith("icons")) return 1;
  const min = ICON_COL_MIN_PX[mode] ?? 96;
  if (!Number.isFinite(containerWidth) || containerWidth <= 0) return 1;
  return Math.max(1, Math.floor(containerWidth / min));
}

export function estimateRowHeightPx(mode: PlaceholderViewMode): number {
  if (mode.startsWith("icons")) {
    return PLACEHOLDER_ICON_SIZE[mode] + ICON_CAPTION_EXTRA_PX;
  }
  if (mode === "tiles") return TILE_ROW_HEIGHT_PX;
  if (mode === "content") return CONTENT_ROW_HEIGHT_PX;
  return LIST_ROW_HEIGHT_PX;
}

/**
 * How many skeleton cells fill the current list viewport for the active view.
 * Uses the same icon column estimate as keyboard navigation so layout matches.
 */
export function estimatePlaceholderCount(
  mode: PlaceholderViewMode,
  containerWidth: number,
  containerHeight: number,
): number {
  if (!Number.isFinite(containerWidth) || !Number.isFinite(containerHeight)) {
    return PLACEHOLDER_FALLBACK_COUNT;
  }
  if (containerWidth <= 0 || containerHeight <= 0) {
    return PLACEHOLDER_FALLBACK_COUNT;
  }

  const columns = estimateIconColumns(containerWidth, mode);
  const rowHeight = estimateRowHeightPx(mode);
  let usableHeight = containerHeight;
  if (mode === "details") {
    usableHeight = Math.max(0, containerHeight - DETAILS_HEADER_HEIGHT_PX);
  }
  const rows = Math.max(1, Math.ceil(usableHeight / Math.max(1, rowHeight)));
  return Math.min(PLACEHOLDER_COUNT_CAP, Math.max(columns, rows * columns));
}

/** CSS length for the icon/thumbnail slot (avoids md FluentIcon → shell-icon growth). */
export function iconSlotCssSize(mode: PlaceholderViewMode): string {
  return `${PLACEHOLDER_ICON_SIZE[mode]}px`;
}
