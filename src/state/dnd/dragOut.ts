// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Native file drag-out (Windows OLE / CF_HDROP via tauri-plugin-drag + patched drag crate).
 *
 * FIL-45: 出站以 OLE 为唯一跨应用通道。按下即登记路径；越过系统拖势阈值后由宿主
 * 直启 `DoDragDrop`（不必等离窗）。`dragstart` 上 `preventDefault` 取消 HTML5，
 * 避免 Sublime 把 `text/plain`/`uri-list` 粘成路径、播放器只看到空 HTML5。
 * 窗内投放仍可通过 OLE 落回 WebView 文件滴入，或由后续手势补齐。
 *
 * 焦点：出站路径禁止 setFocus；按下不抢前台由 drag_focus（临时 WS_EX_NOACTIVATE 等）负责。
 */
import { invoke } from "@tauri-apps/api/core";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { resolveResource } from "@tauri-apps/api/path";

let outboundBusy = false;
let cachedDragIconPath: string | null = null;

async function dragIconPath(): Promise<string> {
  if (!cachedDragIconPath) {
    cachedDragIconPath = await resolveResource("icons/32x32.png");
  }
  return cachedDragIconPath;
}

function armHostOutboundPaths(paths: string[]): void {
  void invoke("dnd_arm_outbound_paths", { paths }).catch(() => {
    /* 非 Windows / 权限缺失时仍走 JS startDrag 回退 */
  });
}

function clearHostOutboundPaths(): void {
  void invoke("dnd_clear_outbound_paths").catch(() => {
    /* ignore */
  });
}

/**
 * 按下文件行时登记路径并预热拖影，供宿主在 SM_CXDRAG 阈值处直启 OLE（早于 dragstart）。
 */
export function armOutboundPathsForPress(paths: string[]): void {
  const cleaned = paths.map((p) => p.trim()).filter(Boolean);
  if (!cleaned.length) return;
  armHostOutboundPaths(cleaned);
  void dragIconPath();
}

/** 单击（无拖势）结束时清空登记，避免下次误启 OLE。 */
export function clearOutboundPathsForPress(): void {
  clearHostOutboundPaths();
}

/**
 * Start Windows-native OLE file drag (COPY|MOVE|LINK).
 * FIL-45: 也可由宿主阈值直启；此处为 dragstart 回退与幂等补启。
 */
export async function startNativeFileDragOut(paths: string[]): Promise<void> {
  const cleaned = paths.map((p) => p.trim()).filter(Boolean);
  if (outboundBusy || cleaned.length === 0) return;
  outboundBusy = true;
  armHostOutboundPaths(cleaned);
  try {
    const icon = cachedDragIconPath ?? (await dragIconPath());
    await startDrag({
      item: cleaned,
      icon,
      mode: "copy",
    });
  } catch (err) {
    console.error("native file drag-out failed", err);
  } finally {
    outboundBusy = false;
    clearHostOutboundPaths();
  }
}

/**
 * @deprecated FIL-45：保留导出名以兼容旧调用；等价于立即 `startNativeFileDragOut`。
 */
export function armNativeFileDragOut(paths: string[]): void {
  void startNativeFileDragOut(paths);
}
