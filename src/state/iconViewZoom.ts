// SPDX-License-Identifier: GPL-3.0-or-later
/** Pure Ctrl+wheel icon-size ladder (AIA-232 / AIA-233). */

export type IconViewZoomMode =
  | "iconsExtraLarge"
  | "iconsLarge"
  | "iconsMedium"
  | "iconsSmall";

/** AIA-232/233: Small → Medium → Large → Extra large. Index 0 = smallest. */
export const ICON_VIEW_ZOOM_ORDER: readonly IconViewZoomMode[] = [
  "iconsSmall",
  "iconsMedium",
  "iconsLarge",
  "iconsExtraLarge",
] as const;

/**
 * Wheel up (deltaY < 0) → larger; wheel down → smaller.
 * Non-icon views enter at medium (up) or small (down).
 */
export function nudgeIconViewMode<T extends string>(
  current: T,
  direction: 1 | -1,
  ladder: readonly T[] = ICON_VIEW_ZOOM_ORDER as readonly T[],
): T {
  if (!ladder.length) return current;
  const idx = ladder.indexOf(current);
  if (idx < 0) {
    return direction > 0
      ? (ladder[Math.min(1, ladder.length - 1)] ?? ladder[0]!)
      : ladder[0]!;
  }
  const next = Math.min(ladder.length - 1, Math.max(0, idx + direction));
  return ladder[next]!;
}
