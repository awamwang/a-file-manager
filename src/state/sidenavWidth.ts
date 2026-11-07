// SPDX-License-Identifier: GPL-3.0-or-later
/** AIA-237: resizable folder-tree / product sidenav width (persisted). */

import { ref } from "vue";

export const DEFAULT_SIDENAV_WIDTH = 200;
export const MIN_SIDENAV_WIDTH = 160;
export const MAX_SIDENAV_WIDTH = 480;
/** Leave at least this much for the browse workspace while dragging. */
export const MIN_WORKSPACE_WIDTH = 280;

const STORAGE_KEY = "afm.sidenavWidth";

export const sidenavWidth = ref(DEFAULT_SIDENAV_WIDTH);

export function clampSidenavWidth(
  width: number,
  viewportWidth?: number,
): number {
  if (!Number.isFinite(width)) return DEFAULT_SIDENAV_WIDTH;
  let max = MAX_SIDENAV_WIDTH;
  if (viewportWidth != null && Number.isFinite(viewportWidth) && viewportWidth > 0) {
    max = Math.min(max, Math.max(MIN_SIDENAV_WIDTH, viewportWidth - MIN_WORKSPACE_WIDTH));
  }
  return Math.min(max, Math.max(MIN_SIDENAV_WIDTH, Math.round(width)));
}

export function readStoredSidenavWidth(): number {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw == null) return DEFAULT_SIDENAV_WIDTH;
    return clampSidenavWidth(Number(raw));
  } catch {
    return DEFAULT_SIDENAV_WIDTH;
  }
}

export function writeStoredSidenavWidth(width: number): void {
  const next = clampSidenavWidth(width);
  try {
    if (next === DEFAULT_SIDENAV_WIDTH) localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, String(next));
  } catch {
    /* quota / private mode */
  }
}

export function loadSidenavWidth(): number {
  const next = readStoredSidenavWidth();
  sidenavWidth.value = next;
  return next;
}

/** Live width while dragging; does not touch storage. */
export function applySidenavWidth(width: number, viewportWidth?: number): number {
  const next = clampSidenavWidth(width, viewportWidth);
  sidenavWidth.value = next;
  return next;
}

/** Clamp, apply, and persist (drag end / keyboard). */
export function setSidenavWidth(width: number, viewportWidth?: number): number {
  const next = applySidenavWidth(width, viewportWidth);
  writeStoredSidenavWidth(next);
  return next;
}
