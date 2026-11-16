// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 框选命中：与资源管理器一致——图标/平铺按图标视觉区相交，列表/详细信息/内容按整行相交。
 * 使用视口坐标（getBoundingClientRect 同源），避免 offsetLeft/Top 在 grid 画布下错位漏选。
 */

export type RectLike = {
  left: number;
  top: number;
  right: number;
  bottom: number;
};

/** 两轴对齐矩形是否相交（边界相贴算相交）。 */
export function rectsIntersect(a: RectLike, b: RectLike): boolean {
  return a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;
}

export type MarqueeHitMode = "icon" | "row";

/** 图标/平铺命中图标；其余视图命中整行。 */
export function marqueeHitMode(viewMode: {
  isIconView: boolean;
  isTilesView: boolean;
}): MarqueeHitMode {
  return viewMode.isIconView || viewMode.isTilesView ? "icon" : "row";
}

export type MarqueeHitEntry = {
  path: string;
  /** 图标视图为图标槽，其它为整行。 */
  hitRect: RectLike;
};

/**
 * 框选命中路径列表（保持 DOM 遍历顺序）。
 * `box` 与 `hitRect` 须为同一坐标系（通常为视口 client 坐标）。
 */
export function hitTestMarqueeEntries(
  box: RectLike,
  entries: readonly MarqueeHitEntry[],
  minSize = 2,
): string[] {
  if (box.right - box.left < minSize || box.bottom - box.top < minSize) {
    return [];
  }
  const hits: string[] = [];
  for (const entry of entries) {
    if (rectsIntersect(box, entry.hitRect)) {
      hits.push(entry.path);
    }
  }
  return hits;
}
