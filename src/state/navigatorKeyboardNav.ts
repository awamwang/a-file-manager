// SPDX-License-Identifier: GPL-3.0-or-later
/** Arrow-key selection movement for Navigator listings (Explorer-like). */

export type ArrowDirection = "up" | "down" | "left" | "right";

export type ArrowSelectionState = {
  selectedPaths: string[];
  selectionAnchor: string | null;
};

/** Min column widths mirroring `.list--icons-*` CSS minmax values (px @ 16px rem). */
const ICON_COL_MIN_PX: Record<string, number> = {
  iconsExtraLarge: 288, // 18rem
  iconsLarge: 120, // 7.5rem
  iconsMedium: 96, // 6rem
  iconsSmall: 80, // 5rem
};

export function estimateIconColumns(
  containerWidth: number,
  mode: string,
): number {
  if (!mode.startsWith("icons")) return 1;
  const min = ICON_COL_MIN_PX[mode] ?? 96;
  if (!Number.isFinite(containerWidth) || containerWidth <= 0) return 1;
  return Math.max(1, Math.floor(containerWidth / min));
}

export function nextIndexForArrow(
  currentIndex: number,
  direction: ArrowDirection,
  count: number,
  columns: number,
): number {
  if (count <= 0) return -1;
  const cols = Math.max(1, columns);
  if (currentIndex < 0) {
    return direction === "up" || direction === "left" ? count - 1 : 0;
  }
  const row = Math.floor(currentIndex / cols);
  const col = currentIndex % cols;
  const lastRow = Math.floor((count - 1) / cols);

  switch (direction) {
    case "left":
      if (cols === 1) return Math.max(0, currentIndex - 1);
      return currentIndex > 0 ? currentIndex - 1 : currentIndex;
    case "right":
      if (cols === 1) return Math.min(count - 1, currentIndex + 1);
      return currentIndex < count - 1 ? currentIndex + 1 : currentIndex;
    case "up": {
      if (row <= 0) return currentIndex;
      return (row - 1) * cols + col;
    }
    case "down": {
      if (row >= lastRow) return currentIndex;
      const next = (row + 1) * cols + col;
      return next < count ? next : currentIndex;
    }
  }
}

export function primarySelectionIndex(
  orderedPaths: string[],
  selected: string[],
  anchor: string | null,
): number {
  if (!orderedPaths.length || !selected.length) return -1;
  if (anchor && selected.includes(anchor)) {
    return orderedPaths.indexOf(anchor);
  }
  for (let i = selected.length - 1; i >= 0; i -= 1) {
    const idx = orderedPaths.indexOf(selected[i]!);
    if (idx >= 0) return idx;
  }
  return -1;
}

function rangePaths(
  orderedPaths: string[],
  fromPath: string,
  toPath: string,
): string[] {
  const fromIndex = orderedPaths.indexOf(fromPath);
  const toIndex = orderedPaths.indexOf(toPath);
  if (fromIndex < 0 || toIndex < 0) return [toPath];
  const start = Math.min(fromIndex, toIndex);
  const end = Math.max(fromIndex, toIndex);
  return orderedPaths.slice(start, end + 1);
}

/**
 * Pure arrow-step selection. Caller writes the returned state into refs.
 */
export function moveSelectionByArrow(
  orderedPaths: string[],
  direction: ArrowDirection,
  columns: number,
  extend: boolean,
  state: ArrowSelectionState,
): { path: string; state: ArrowSelectionState } | null {
  if (!orderedPaths.length) return null;
  const from = primarySelectionIndex(
    orderedPaths,
    state.selectedPaths,
    state.selectionAnchor,
  );
  const next = nextIndexForArrow(from, direction, orderedPaths.length, columns);
  if (next < 0) return null;
  const path = orderedPaths[next]!;
  if (extend) {
    let anchor = state.selectionAnchor;
    if (!anchor || !orderedPaths.includes(anchor)) {
      anchor = from >= 0 ? orderedPaths[from]! : path;
    }
    return {
      path,
      state: {
        selectedPaths: rangePaths(orderedPaths, anchor, path),
        selectionAnchor: anchor,
      },
    };
  }
  return {
    path,
    state: {
      selectedPaths: [path],
      selectionAnchor: path,
    },
  };
}

export function arrowDirectionFromKey(key: string): ArrowDirection | null {
  switch (key.toLowerCase()) {
    case "arrowup":
      return "up";
    case "arrowdown":
      return "down";
    case "arrowleft":
      return "left";
    case "arrowright":
      return "right";
    default:
      return null;
  }
}
