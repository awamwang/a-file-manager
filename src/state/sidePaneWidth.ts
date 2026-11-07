// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AIA-256: resizable Details / Preview side panes (persisted, like sidenav).
 * Min floor aligns with dual-pane 15rem spirit (AIA-252 / DUAL_PANE_MIN_PANE_PX).
 */

import { ref } from "vue";

/** ≈15rem @ 16px — same floor spirit as dual-pane panes. */
export const SIDE_PANE_MIN_WIDTH = 240;
export const SIDE_PANE_MAX_WIDTH = 560;
/** Leave at least this much for the browse list while dragging. */
export const SIDE_PANE_MIN_WORKSPACE = 280;

/** Matches prior CSS `width: min(22rem, …)` default. */
export const DEFAULT_PREVIEW_PANE_WIDTH = 352;
/** Matches prior CSS `width: min(20rem, …)` default. */
export const DEFAULT_DETAILS_PANE_WIDTH = 320;

const PREVIEW_STORAGE_KEY = "afm.previewPaneWidth";
const DETAILS_STORAGE_KEY = "afm.detailsPaneWidth";

export const previewPaneWidth = ref(DEFAULT_PREVIEW_PANE_WIDTH);
export const detailsPaneWidth = ref(DEFAULT_DETAILS_PANE_WIDTH);

export function clampSidePaneWidth(
  width: number,
  defaultWidth: number,
  viewportWidth?: number,
): number {
  if (!Number.isFinite(width)) return defaultWidth;
  let max = SIDE_PANE_MAX_WIDTH;
  if (viewportWidth != null && Number.isFinite(viewportWidth) && viewportWidth > 0) {
    max = Math.min(
      max,
      Math.max(SIDE_PANE_MIN_WIDTH, viewportWidth - SIDE_PANE_MIN_WORKSPACE),
    );
  }
  return Math.min(max, Math.max(SIDE_PANE_MIN_WIDTH, Math.round(width)));
}

function readStored(key: string, defaultWidth: number): number {
  try {
    const raw = localStorage.getItem(key);
    if (raw == null) return defaultWidth;
    return clampSidePaneWidth(Number(raw), defaultWidth);
  } catch {
    return defaultWidth;
  }
}

function writeStored(key: string, width: number, defaultWidth: number): void {
  const next = clampSidePaneWidth(width, defaultWidth);
  try {
    if (next === defaultWidth) localStorage.removeItem(key);
    else localStorage.setItem(key, String(next));
  } catch {
    /* quota / private mode */
  }
}

export function loadPreviewPaneWidth(): number {
  const next = readStored(PREVIEW_STORAGE_KEY, DEFAULT_PREVIEW_PANE_WIDTH);
  previewPaneWidth.value = next;
  return next;
}

export function loadDetailsPaneWidth(): number {
  const next = readStored(DETAILS_STORAGE_KEY, DEFAULT_DETAILS_PANE_WIDTH);
  detailsPaneWidth.value = next;
  return next;
}

/** Live width while dragging; does not touch storage. */
export function applyPreviewPaneWidth(width: number, viewportWidth?: number): number {
  const next = clampSidePaneWidth(width, DEFAULT_PREVIEW_PANE_WIDTH, viewportWidth);
  previewPaneWidth.value = next;
  return next;
}

export function applyDetailsPaneWidth(width: number, viewportWidth?: number): number {
  const next = clampSidePaneWidth(width, DEFAULT_DETAILS_PANE_WIDTH, viewportWidth);
  detailsPaneWidth.value = next;
  return next;
}

/** Clamp, apply, and persist (drag end / keyboard). */
export function setPreviewPaneWidth(width: number, viewportWidth?: number): number {
  const next = applyPreviewPaneWidth(width, viewportWidth);
  writeStored(PREVIEW_STORAGE_KEY, next, DEFAULT_PREVIEW_PANE_WIDTH);
  return next;
}

export function setDetailsPaneWidth(width: number, viewportWidth?: number): number {
  const next = applyDetailsPaneWidth(width, viewportWidth);
  writeStored(DETAILS_STORAGE_KEY, next, DEFAULT_DETAILS_PANE_WIDTH);
  return next;
}
