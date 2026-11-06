// SPDX-License-Identifier: GPL-3.0-or-later
/** Resizable Details columns for Navigator. */

export type DetailsColumnId = "name" | "type" | "size" | "modified";

export type DetailsColumnWidths = Record<DetailsColumnId, number>;

export const DEFAULT_DETAILS_COLUMN_WIDTHS: DetailsColumnWidths = {
  name: 280,
  type: 120,
  size: 88,
  modified: 152,
};

export const MIN_DETAILS_COLUMN_WIDTH = 56;

export function clampColumnWidth(width: number): number {
  if (!Number.isFinite(width)) return MIN_DETAILS_COLUMN_WIDTH;
  return Math.max(MIN_DETAILS_COLUMN_WIDTH, Math.round(width));
}

export function resizeColumnWidth(
  widths: DetailsColumnWidths,
  column: DetailsColumnId,
  deltaPx: number,
): DetailsColumnWidths {
  return {
    ...widths,
    [column]: clampColumnWidth(widths[column] + deltaPx),
  };
}

export function detailsGridTemplate(widths: DetailsColumnWidths): string {
  // icon gutter + resizable columns
  return `2rem ${widths.name}px ${widths.type}px ${widths.size}px ${widths.modified}px`;
}
