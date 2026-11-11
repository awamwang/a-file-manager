// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 将 WebView/浏览器前进后退手势（鼠标侧键、BrowserBack/Forward、History traverse）
 * 从 SPA `history` 栈剥离，改由调用方绑定到 Navigator 会话历史。
 */

export type BrowserNavDirection = "back" | "forward";

/** MouseEvent.button：XButton1=后退，XButton2=前进（Chromium / WebView2）。 */
export function mouseButtonNavDirection(
  button: number,
): BrowserNavDirection | null {
  if (button === 3) return "back";
  if (button === 4) return "forward";
  return null;
}

/** 键盘 `BrowserBack` / `BrowserForward`（部分键鼠通过 AppCommand 注入）。 */
export function keyboardNavDirection(key: string): BrowserNavDirection | null {
  const normalized = key.trim().toLowerCase();
  if (normalized === "browserback") return "back";
  if (normalized === "browserforward") return "forward";
  return null;
}

export type BrowserNavGestureHandlers = {
  onBack: () => void;
  onForward: () => void;
};

type NavigationEntryLike = { index?: number };

type NavigateEventLike = Event & {
  navigationType?: string;
  canIntercept?: boolean;
  destination?: NavigationEntryLike;
  intercept?: (options: { handler: () => void | Promise<void> }) => void;
};

type NavigationLike = EventTarget & {
  currentEntry?: NavigationEntryLike | null;
  addEventListener: (
    type: "navigate",
    listener: (event: NavigateEventLike) => void,
  ) => void;
  removeEventListener: (
    type: "navigate",
    listener: (event: NavigateEventLike) => void,
  ) => void;
};

function navigationApi(): NavigationLike | null {
  const nav = (window as unknown as { navigation?: NavigationLike }).navigation;
  return nav && typeof nav.addEventListener === "function" ? nav : null;
}

function suppressMouseDefault(event: MouseEvent): void {
  if (!mouseButtonNavDirection(event.button)) return;
  event.preventDefault();
  event.stopPropagation();
}

function traverseDirection(
  nav: NavigationLike,
  event: NavigateEventLike,
): BrowserNavDirection | null {
  const from = nav.currentEntry?.index;
  const to = event.destination?.index;
  if (typeof from !== "number" || typeof to !== "number") return null;
  if (to < from) return "back";
  if (to > from) return "forward";
  return null;
}

/**
 * 安装捕获期监听：侧键触发 Navigator 历史；并拦截 History traverse，避免离开壳层路由。
 * 返回卸载函数。
 */
export function installBrowserNavGestureGuards(
  handlers: BrowserNavGestureHandlers,
): () => void {
  /** 同一手势可能同时冒泡 mouse + navigate，短窗去重避免连跳两步。 */
  let lastEmitAt = 0;
  let lastEmitDirection: BrowserNavDirection | null = null;

  const emit = (direction: BrowserNavDirection) => {
    const now =
      typeof performance !== "undefined" && typeof performance.now === "function"
        ? performance.now()
        : Date.now();
    if (lastEmitDirection === direction && now - lastEmitAt < 80) {
      return;
    }
    lastEmitAt = now;
    lastEmitDirection = direction;
    if (direction === "back") {
      handlers.onBack();
    } else {
      handlers.onForward();
    }
  };

  const onMouseDown = (event: MouseEvent) => {
    const direction = mouseButtonNavDirection(event.button);
    if (!direction) return;
    suppressMouseDefault(event);
    emit(direction);
  };

  const onMouseSuppress = (event: MouseEvent) => {
    suppressMouseDefault(event);
  };

  const onNavigate = (event: NavigateEventLike) => {
    // 仅处理 traverse（侧键 / BrowserBack 驱动的历史穿越），保留 push/replace 等壳内路由。
    if (event.navigationType !== "traverse") return;
    if (!event.canIntercept || typeof event.intercept !== "function") return;
    const nav = navigationApi();
    const direction = nav ? traverseDirection(nav, event) : null;
    event.intercept({
      handler() {
        // 取消 WebView 默认前进后退；若侧键回调未抢先，则由此驱动 Navigator 历史。
        if (direction) {
          emit(direction);
        }
      },
    });
  };

  window.addEventListener("mousedown", onMouseDown, true);
  window.addEventListener("mouseup", onMouseSuppress, true);
  window.addEventListener("auxclick", onMouseSuppress, true);

  const nav = navigationApi();
  if (nav) {
    nav.addEventListener("navigate", onNavigate);
  }

  return () => {
    window.removeEventListener("mousedown", onMouseDown, true);
    window.removeEventListener("mouseup", onMouseSuppress, true);
    window.removeEventListener("auxclick", onMouseSuppress, true);
    if (nav) {
      nav.removeEventListener("navigate", onNavigate);
    }
  };
}
