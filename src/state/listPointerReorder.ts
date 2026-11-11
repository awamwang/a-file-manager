// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 列表指针拖动排序（WebView2 下 HTML5 DnD 不可靠）。
 * 阈值位移后才进入拖动；拖动结束后短暂抑制 click，避免误打开/激活。
 */
import { ref, type Ref } from "vue";

const DEFAULT_THRESHOLD_PX = 6;
const DEFAULT_CLICK_SUPPRESS_MS = 220;

export type ListPointerReorderApi = {
  dragFromId: Ref<string | null>;
  dragOverId: Ref<string | null>;
  dragMoved: Ref<boolean>;
  onPointerDown: (event: PointerEvent, itemId: string) => void;
  shouldSuppressActivate: () => boolean;
  dispose: () => void;
};

export type ListPointerReorderOptions = {
  /** 命中检测用的列表项选择器，默认 `[data-list-item-id]`。 */
  itemSelector?: string;
  thresholdPx?: number;
  clickSuppressMs?: number;
  /** 命中这些选择器时不启动拖动（⋯ 菜单、改名输入等）。 */
  ignoreSelector?: string;
  onReorder: (fromId: string, toId: string) => void | Promise<void>;
};

export function createListPointerReorder(
  options: ListPointerReorderOptions,
): ListPointerReorderApi {
  const itemSelector = options.itemSelector ?? "[data-list-item-id]";
  const thresholdPx = options.thresholdPx ?? DEFAULT_THRESHOLD_PX;
  const clickSuppressMs = options.clickSuppressMs ?? DEFAULT_CLICK_SUPPRESS_MS;
  const ignoreSelector =
    options.ignoreSelector ??
    "input, textarea, button, a, select, [contenteditable='true'], .list-more-menu, .list-inline-rename";

  const dragFromId = ref<string | null>(null);
  const dragOverId = ref<string | null>(null);
  const dragMoved = ref(false);

  let pointerId: number | null = null;
  let fromId: string | null = null;
  let startX = 0;
  let startY = 0;
  let reorderBusy = false;
  let clickSuppressTimer: ReturnType<typeof setTimeout> | null = null;

  function clearPointerListeners() {
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerUp);
    window.removeEventListener("pointercancel", onPointerUp);
    pointerId = null;
  }

  function armClickSuppress() {
    dragMoved.value = true;
    if (clickSuppressTimer != null) clearTimeout(clickSuppressTimer);
    clickSuppressTimer = setTimeout(() => {
      dragMoved.value = false;
      clickSuppressTimer = null;
    }, clickSuppressMs);
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

  function onPointerMove(event: PointerEvent) {
    if (pointerId !== event.pointerId || !fromId) return;
    const dist = Math.hypot(event.clientX - startX, event.clientY - startY);
    if (!dragMoved.value) {
      if (dist < thresholdPx) return;
      dragMoved.value = true;
    }
    dragFromId.value = fromId;
    const overId = itemIdAtPoint(event.clientX, event.clientY);
    if (!overId || overId === fromId) {
      dragOverId.value = null;
      return;
    }
    dragOverId.value = overId;
  }

  async function onPointerUp(event: PointerEvent) {
    if (pointerId !== event.pointerId) return;
    const sourceId = fromId;
    const targetId = dragOverId.value;
    const moved = dragMoved.value;
    clearPointerListeners();
    dragFromId.value = null;
    dragOverId.value = null;
    fromId = null;

    if (moved) {
      event.preventDefault();
      armClickSuppress();
    }

    if (!moved || !sourceId || !targetId || sourceId === targetId || reorderBusy) return;
    reorderBusy = true;
    try {
      await options.onReorder(sourceId, targetId);
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
    dragFromId.value = null;
    dragOverId.value = null;
    dragMoved.value = false;
    fromId = null;
  }

  return {
    dragFromId,
    dragOverId,
    dragMoved,
    onPointerDown,
    shouldSuppressActivate,
    dispose,
  };
}
