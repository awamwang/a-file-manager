// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Native file drag-out (Windows OLE / CF_HDROP via tauri-plugin-drag + patched drag crate).
 * Keeps HTML5 in-app drops intact; starts OS drag when the cursor leaves (or nears the
 * edge of) the window so Explorer receives real paths with copy|move|link.
 *
 * AIA-261/264/266/268/280 + FIL-37/39: outbound must not raise AFM over the drop target —
 * host distinguishes click vs drag (`MA_NOACTIVATE` + SM_CXDRAG poll + press-time prior
 * FG + z-order restore); rows stay `tabindex=-1` (no mousedown preventDefault —
 * that cancels HTML5 dragstart); patched `DoDragDrop` restores prior FG.
 * FIL-39: promote OLE earlier (near-edge / leave-client / host `afm://dnd/ole-promote`
 * when drag gesture + cursor not over root HWND) while keeping in-window HTML5 (ADR 0012).
 * No setFocus / SetForegroundWindow here.
 */
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { resolveResource } from "@tauri-apps/api/path";
import { cursorPosition, getCurrentWindow } from "@tauri-apps/api/window";

let outboundBusy = false;
let cachedDragIconPath: string | null = null;
let activeCleanup: (() => void) | null = null;

/** 近边提前提升 OLE，使光标进入外部目标前已进入 DoDragDrop（FIL-39）。 */
const EDGE_MARGIN_PX = 24;

/** 宿主 Hybrid 提升事件（drag_focus：拖势 + 光标不在根 HWND 树）。 */
const OLE_PROMOTE_EVENT = "afm://dnd/ole-promote";

async function dragIconPath(): Promise<string> {
  if (!cachedDragIconPath) {
    cachedDragIconPath = await resolveResource("icons/32x32.png");
  }
  return cachedDragIconPath;
}

function isOutsideOrNearEdge(clientX: number, clientY: number): boolean {
  return (
    clientX <= EDGE_MARGIN_PX ||
    clientY <= EDGE_MARGIN_PX ||
    clientX >= window.innerWidth - EDGE_MARGIN_PX ||
    clientY >= window.innerHeight - EDGE_MARGIN_PX
  );
}

/** 用窗口外接矩形判断指针是否离开客户区附近（拖事件可能停更时的兜底）。 */
async function isCursorNearOrOutsideWindow(): Promise<boolean> {
  try {
    const win = getCurrentWindow();
    const [cursor, pos, size] = await Promise.all([
      cursorPosition(),
      win.outerPosition(),
      win.outerSize(),
    ]);
    const m = EDGE_MARGIN_PX;
    return (
      cursor.x <= pos.x + m ||
      cursor.y <= pos.y + m ||
      cursor.x >= pos.x + size.width - m ||
      cursor.y >= pos.y + size.height - m
    );
  } catch {
    return false;
  }
}

/**
 * Start Windows-native OLE file drag (patched drag offers COPY|MOVE|LINK).
 * Does not call window focus APIs — see AIA-261/264/266 / FIL-37.
 */
export async function startNativeFileDragOut(paths: string[]): Promise<void> {
  const cleaned = paths.map((p) => p.trim()).filter(Boolean);
  if (outboundBusy || cleaned.length === 0) return;
  outboundBusy = true;
  try {
    const icon = await dragIconPath();
    await startDrag({
      item: cleaned,
      icon,
      // Mode retained for plugin API; Windows patch always offers copy|move|link.
      mode: "copy",
    });
  } catch (err) {
    console.error("native file drag-out failed", err);
  } finally {
    outboundBusy = false;
  }
}

/**
 * While an HTML5 drag is active, watch for leave-window / near-edge / blur / host
 * promote and switch to native OLE drag-out (Hybrid 出站；ADR 0012).
 */
export function armNativeFileDragOut(paths: string[]): void {
  disarmNativeFileDragOut();
  const cleaned = paths.map((p) => p.trim()).filter(Boolean);
  if (!cleaned.length) return;

  let fired = false;
  const tryOutbound = () => {
    if (fired || outboundBusy) return;
    fired = true;
    disarmNativeFileDragOut();
    void startNativeFileDragOut(cleaned);
  };

  const onDrag = (event: DragEvent) => {
    if (isOutsideOrNearEdge(event.clientX, event.clientY)) {
      tryOutbound();
    }
  };

  const onDragLeave = (event: DragEvent) => {
    if (!event.relatedTarget) {
      tryOutbound();
    }
  };

  const onDragEnd = () => {
    disarmNativeFileDragOut();
  };

  const onBlur = () => {
    tryOutbound();
  };

  const onVisibility = () => {
    if (document.hidden) tryOutbound();
  };

  document.addEventListener("drag", onDrag);
  document.addEventListener("dragleave", onDragLeave);
  document.addEventListener("dragend", onDragEnd);
  window.addEventListener("blur", onBlur);
  document.addEventListener("visibilitychange", onVisibility);

  let unlistenFocus: (() => void) | null = null;
  void getCurrentWindow()
    .onFocusChanged(({ payload: focused }) => {
      if (!focused) tryOutbound();
    })
    .then((unlisten) => {
      unlistenFocus = unlisten;
    })
    .catch(() => {
      /* non-Tauri / permission — blur + drag listeners still cover common cases */
    });

  let unlistenPromote: UnlistenFn | null = null;
  void listen(OLE_PROMOTE_EVENT, () => {
    tryOutbound();
  })
    .then((unlisten) => {
      unlistenPromote = unlisten;
    })
    .catch(() => {
      /* 事件权限缺失时仍靠近边 / leave / blur */
    });

  let pollStopped = false;
  const poll = () => {
    if (pollStopped || fired) return;
    void isCursorNearOrOutsideWindow().then((outside) => {
      if (outside) tryOutbound();
      else if (!pollStopped && !fired) {
        window.setTimeout(poll, 32);
      }
    });
  };
  window.setTimeout(poll, 32);

  activeCleanup = () => {
    pollStopped = true;
    document.removeEventListener("drag", onDrag);
    document.removeEventListener("dragleave", onDragLeave);
    document.removeEventListener("dragend", onDragEnd);
    window.removeEventListener("blur", onBlur);
    document.removeEventListener("visibilitychange", onVisibility);
    if (unlistenFocus) {
      unlistenFocus();
      unlistenFocus = null;
    }
    if (unlistenPromote) {
      unlistenPromote();
      unlistenPromote = null;
    }
    activeCleanup = null;
  };
}

function disarmNativeFileDragOut(): void {
  if (activeCleanup) {
    activeCleanup();
  }
}
