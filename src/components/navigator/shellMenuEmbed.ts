// SPDX-License-Identifier: GPL-3.0-or-later
/** Windows shell 右键菜单适配器：原生 TrackPopupMenu（FIL-53 / AIA-229）。 */

import { invoke } from "@tauri-apps/api/core";

export type ShellMenuResult = {
  success: boolean;
  cancelled: boolean;
  error: string | null;
};

/**
 * 原生菜单是独立 Win32 窗口，但 owner 是 AFM 主窗。菜单弹出/关闭时 WebView
 * 会收到穿透的 mousedown / click / contextmenu，落点正是「当前光标下的文件行」，
 * 于是 onRowClick / onContextMenu 把选中改成鼠标最新位置的文件。守卫在捕获
 * 阶段吞掉整个菜单生命周期内的指针事件，保持触发右键时的选中不变。
 */
const PENETRATING_EVENTS = [
  "pointerdown",
  "mousedown",
  "mouseup",
  "click",
  "dblclick",
  "contextmenu",
] as const;

let guardActive = false;
let guardDepth = 0;
let guardReleaseTimer: ReturnType<typeof setTimeout> | null = null;

function swallowPenetratingEvent(event: Event) {
  if (!guardActive) return;
  event.preventDefault();
  event.stopImmediatePropagation();
}

/** 在关闭 AFM 自绘菜单之前调用，覆盖原生菜单弹出前的事件窗口。 */
export function beginShellMenuGuard(): void {
  guardDepth += 1;
  if (guardReleaseTimer !== null) {
    clearTimeout(guardReleaseTimer);
    guardReleaseTimer = null;
  }
  if (guardActive) return;
  guardActive = true;
  for (const name of PENETRATING_EVENTS) {
    window.addEventListener(name, swallowPenetratingEvent, true);
  }
}

/** 菜单关闭后延迟释放，吃掉仍排在 WebView 队列里的 mouseup / click。 */
export function endShellMenuGuard(): void {
  guardDepth = Math.max(0, guardDepth - 1);
  if (guardDepth > 0) return;
  if (guardReleaseTimer !== null) clearTimeout(guardReleaseTimer);
  guardReleaseTimer = setTimeout(() => {
    guardReleaseTimer = null;
    guardActive = false;
    for (const name of PENETRATING_EVENTS) {
      window.removeEventListener(name, swallowPenetratingEvent, true);
    }
  }, 200);
}

/** 视口 CSS 坐标 → 屏幕物理坐标；锚点在触发时固定，不随后续鼠标移动漂移。 */
async function toScreenPoint(x: number, y: number): Promise<{ x: number; y: number }> {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    const [origin, factor] = await Promise.all([win.innerPosition(), win.scaleFactor()]);
    return {
      x: Math.round(origin.x + x * factor),
      y: Math.round(origin.y + y * factor),
    };
  } catch {
    // 回退：Chromium 的 screenX/screenY 是窗口外框的屏幕 CSS 坐标。
    const factor = window.devicePixelRatio || 1;
    return {
      x: Math.round((window.screenX + x) * factor),
      y: Math.round((window.screenY + y) * factor),
    };
  }
}

/** 在触发时确定的屏幕坐标弹出系统右键菜单；失败返回错误文案，取消返回 null。 */
export async function showNativeShellContextMenu(
  path: string,
  x: number,
  y: number,
): Promise<string | null> {
  beginShellMenuGuard();
  try {
    const screen = await toScreenPoint(x, y);
    const result = await invoke<ShellMenuResult>("shell_show_context_menu", {
      path,
      x: screen.x,
      y: screen.y,
    });
    if (!result.success && result.error) return result.error;
    return null;
  } catch (err) {
    return err instanceof Error ? err.message : String(err);
  } finally {
    endShellMenuGuard();
  }
}
