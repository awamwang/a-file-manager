// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 列表指针拖动排序（WebView2 下 HTML5 DnD 不可靠）。
 * 阈值位移后才进入拖动；拖动结束后短暂抑制 click，避免误打开/激活。
 * 提供拖起浮层、落地插入边与短促落地动画状态，供列表 UI 渲染预览。
 */
import { ref, type Ref } from "vue";

const DEFAULT_THRESHOLD_PX = 6;
const DEFAULT_CLICK_SUPPRESS_MS = 220;
const DEFAULT_LAND_ANIM_MS = 280;

export type ListDragPreview = {
  label: string;
  meta: string;
  width: number;
  height: number;
  offsetX: number;
  offsetY: number;
  x: number;
  y: number;
};

export type ListPointerReorderApi = {
  dragFromId: Ref<string | null>;
  dragOverId: Ref<string | null>;
  dragMoved: Ref<boolean>;
  dragPreview: Ref<ListDragPreview | null>;
  dropEdge: Ref<"before" | "after" | null>;
  landedId: Ref<string | null>;
  onPointerDown: (event: PointerEvent, itemId: string) => void;
  shouldSuppressActivate: () => boolean;
  dispose: () => void;
};

export type ListPointerReorderOptions = {
  /** 命中检测用的列表项选择器，默认 `[data-list-item-id]`。 */
  itemSelector?: string;
  thresholdPx?: number;
  clickSuppressMs?: number;
  landAnimMs?: number;
  /** 命中这些选择器时不启动拖动（⋯ 菜单、改名输入等）。 */
  ignoreSelector?: string;
  /** 拖起浮层文案；缺省时从列表项 DOM 抽取。 */
  getPreview?: (itemId: string) => { label: string; meta?: string } | null;
  onReorder: (fromId: string, toId: string) => void | Promise<void>;
};

export function createListPointerReorder(
  options: ListPointerReorderOptions,
): ListPointerReorderApi {
  const itemSelector = options.itemSelector ?? "[data-list-item-id]";
  const thresholdPx = options.thresholdPx ?? DEFAULT_THRESHOLD_PX;
  const clickSuppressMs = options.clickSuppressMs ?? DEFAULT_CLICK_SUPPRESS_MS;
  const landAnimMs = options.landAnimMs ?? DEFAULT_LAND_ANIM_MS;
  const ignoreSelector =
    options.ignoreSelector ??
    "input, textarea, button, a, select, [contenteditable='true'], .list-more-menu, .list-inline-rename";

  const dragFromId = ref<string | null>(null);
  const dragOverId = ref<string | null>(null);
  const dragMoved = ref(false);
  const dragPreview = ref<ListDragPreview | null>(null);
  const dropEdge = ref<"before" | "after" | null>(null);
  const landedId = ref<string | null>(null);

  let pointerId: number | null = null;
  let fromId: string | null = null;
  let startX = 0;
  let startY = 0;
  let reorderBusy = false;
  let clickSuppressTimer: ReturnType<typeof setTimeout> | null = null;
  let landedTimer: ReturnType<typeof setTimeout> | null = null;

  function clearPointerListeners() {
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerUp);
    window.removeEventListener("pointercancel", onPointerUp);
    pointerId = null;
  }

  function clearDragPreview() {
    dragPreview.value = null;
    dropEdge.value = null;
  }

  function armClickSuppress() {
    dragMoved.value = true;
    if (clickSuppressTimer != null) clearTimeout(clickSuppressTimer);
    clickSuppressTimer = setTimeout(() => {
      dragMoved.value = false;
      clickSuppressTimer = null;
    }, clickSuppressMs);
  }

  function armLanded(itemId: string) {
    if (landedTimer != null) clearTimeout(landedTimer);
    landedId.value = itemId;
    landedTimer = setTimeout(() => {
      landedId.value = null;
      landedTimer = null;
    }, landAnimMs);
  }

  function itemIdAtPoint(clientX: number, clientY: number): string | null {
    const nodes = document.querySelectorAll<HTMLElement>(itemSelector);
    for (const node of nodes) {
      const rect = node.getBoundingClientRect();
      if (
        clientX >= rect.left &&
        clientX <= rect.right &&
        clientY >= rect.top &&
        clientY <= rect.bottom
      ) {
        const id = node.dataset.listItemId;
        return id && id.length ? id : null;
      }
    }
    return null;
  }

  function orderedItemIds(): string[] {
    const nodes = document.querySelectorAll<HTMLElement>(itemSelector);
    const ids: string[] = [];
    for (const node of nodes) {
      const id = node.dataset.listItemId;
      if (id && id.length) ids.push(id);
    }
    return ids;
  }

  function dropEdgeFor(sourceId: string, targetId: string): "before" | "after" | null {
    const ids = orderedItemIds();
    const from = ids.indexOf(sourceId);
    const to = ids.indexOf(targetId);
    if (from < 0 || to < 0 || from === to) return null;
    return from < to ? "after" : "before";
  }

  function itemElement(itemId: string): HTMLElement | null {
    const escaped =
      typeof CSS !== "undefined" && typeof CSS.escape === "function"
        ? CSS.escape(itemId)
        : itemId.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
    return document.querySelector<HTMLElement>(`[data-list-item-id="${escaped}"]`);
  }

  function resolvePreviewCopy(itemId: string): { label: string; meta: string } {
    const custom = options.getPreview?.(itemId);
    if (custom?.label) {
      return { label: custom.label, meta: custom.meta ?? "" };
    }
    const el = itemElement(itemId);
    const label =
      el?.querySelector("strong")?.textContent?.trim() ||
      el?.textContent?.trim() ||
      itemId;
    const meta = el?.querySelector(".fluent-page-meta")?.textContent?.trim() || "";
    return { label, meta };
  }

  function beginDragPreview(event: PointerEvent, itemId: string) {
    const el = itemElement(itemId);
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const copy = resolvePreviewCopy(itemId);
    dragPreview.value = {
      label: copy.label,
      meta: copy.meta,
      width: Math.round(rect.width),
      height: Math.round(rect.height),
      offsetX: event.clientX - rect.left,
      offsetY: event.clientY - rect.top,
      x: event.clientX,
      y: event.clientY,
    };
  }

  function updateDragPreview(event: PointerEvent) {
    const preview = dragPreview.value;
    if (!preview) return;
    dragPreview.value = {
      ...preview,
      x: event.clientX,
      y: event.clientY,
    };
  }

  function onPointerMove(event: PointerEvent) {
    if (pointerId !== event.pointerId || !fromId) return;
    const dist = Math.hypot(event.clientX - startX, event.clientY - startY);
    if (!dragMoved.value) {
      if (dist < thresholdPx) return;
      dragMoved.value = true;
      beginDragPreview(event, fromId);
    } else {
      updateDragPreview(event);
    }
    dragFromId.value = fromId;
    const overId = itemIdAtPoint(event.clientX, event.clientY);
    if (!overId || overId === fromId) {
      dragOverId.value = null;
      dropEdge.value = null;
      return;
    }
    dragOverId.value = overId;
    dropEdge.value = dropEdgeFor(fromId, overId);
  }

  async function onPointerUp(event: PointerEvent) {
    if (pointerId !== event.pointerId) return;
    const sourceId = fromId;
    const targetId = dragOverId.value;
    const moved = dragMoved.value;
    clearPointerListeners();
    dragFromId.value = null;
    dragOverId.value = null;
    clearDragPreview();
    fromId = null;

    if (moved) {
      event.preventDefault();
      armClickSuppress();
    }

    if (!moved || !sourceId || !targetId || sourceId === targetId || reorderBusy) return;
    reorderBusy = true;
    try {
      await options.onReorder(sourceId, targetId);
      armLanded(sourceId);
    } finally {
      reorderBusy = false;
    }
  }

  function onPointerDown(event: PointerEvent, itemId: string) {
    if (event.button !== 0) return;
    if (event.target instanceof Element && event.target.closest(ignoreSelector)) {
      return;
    }
    clearPointerListeners();
    pointerId = event.pointerId;
    fromId = itemId;
    startX = event.clientX;
    startY = event.clientY;
    dragFromId.value = itemId;
    dragOverId.value = null;
    clearDragPreview();
    if (clickSuppressTimer == null) {
      dragMoved.value = false;
    }
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
    window.addEventListener("pointercancel", onPointerUp);
  }

  function shouldSuppressActivate(): boolean {
    return dragMoved.value;
  }

  function dispose() {
    clearPointerListeners();
    if (clickSuppressTimer != null) {
      clearTimeout(clickSuppressTimer);
      clickSuppressTimer = null;
    }
    if (landedTimer != null) {
      clearTimeout(landedTimer);
      landedTimer = null;
    }
    dragFromId.value = null;
    dragOverId.value = null;
    dragMoved.value = false;
    clearDragPreview();
    landedId.value = null;
    fromId = null;
  }

  return {
    dragFromId,
    dragOverId,
    dragMoved,
    dragPreview,
    dropEdge,
    landedId,
    onPointerDown,
    shouldSuppressActivate,
    dispose,
  };
}
