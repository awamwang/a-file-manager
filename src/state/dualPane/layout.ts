// SPDX-License-Identifier: GPL-3.0-or-later
/** Pure dual-pane width clamp helpers (AIA-252). No Vue / Tauri imports. */

/** Matches `tauri.conf.json` windows[0].minWidth (single-pane baseline). */
export const DEFAULT_WINDOW_MIN_WIDTH_PX = 330;
/** Matches `tauri.conf.json` windows[0].minHeight. */
export const DEFAULT_WINDOW_MIN_HEIGHT_PX = 400;

/**
 * Hard floor per pane when dual (≈15rem @ 16px). Enough for icon + name column.
 * Do not pair with a `%` min() — that defeated the previous CSS “guard”.
 */
export const DUAL_PANE_MIN_PANE_PX = 240;

/** Gap between dual panes (`--fluent-space-sm`). */
export const DUAL_PANE_GAP_PX = 8;

/**
 * OS window min width while dual pane is on:
 * two pane floors + gap + reserved chrome (sidenav ≈200 + chrome ≈32 → 232).
 */
export const DUAL_PANE_WINDOW_MIN_WIDTH_PX =
  DUAL_PANE_MIN_PANE_PX * 2 + DUAL_PANE_GAP_PX + 232;

/** Window min-width clamp for the current dual-pane mode. */
export function windowMinWidthForDualPane(enabled: boolean): number {
  return enabled ? DUAL_PANE_WINDOW_MIN_WIDTH_PX : DEFAULT_WINDOW_MIN_WIDTH_PX;
}

/** True when `innerWidth` can host two clamped panes without forced overflow. */
export function dualPaneFitsInnerWidth(innerWidthPx: number): boolean {
  return Number.isFinite(innerWidthPx) && innerWidthPx >= DUAL_PANE_WINDOW_MIN_WIDTH_PX;
}
