// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Native file drag-out (Windows OLE / CF_HDROP via tauri-plugin-drag + patched drag crate).
 * Keeps HTML5 in-app drops intact; starts OS drag when the cursor leaves (or nears the
 * edge of) the window so Explorer receives real paths with copy|move|link.
 *
 * AIA-261/264: outbound must not raise AFM over the drop target — host distinguishes
 * click vs drag (`MA_NOACTIVATE` + SM_CXDRAG poll + press-time prior FG), and the
 * patched `DoDragDrop` path restores that prior FG (no setFocus here).
 */
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { resolveResource } from "@tauri-apps/api/path";
import { getCurrentWindow } from "@tauri-apps/api/window";

let outboundBusy = false;
let cachedDragIconPath: string | null = null;
let activeCleanup: (() => void) | null = null;

/** Edge margin so OLE arms before Explorer sees HTML5-only / no-drop. */
const EDGE_MARGIN_PX = 8;

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

/**
 * Start Windows-native OLE file drag (patched drag offers COPY|MOVE|LINK).
 * Does not call window focus APIs — see AIA-261/264.
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
 * While an HTML5 drag is active, watch for leave-window / near-edge / blur and
 * switch to native OLE drag-out (Hybrid 出站).
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

  activeCleanup = () => {
    document.removeEventListener("drag", onDrag);
    document.removeEventListener("dragleave", onDragLeave);
    document.removeEventListener("dragend", onDragEnd);
    window.removeEventListener("blur", onBlur);
    document.removeEventListener("visibilitychange", onVisibility);
    if (unlistenFocus) {
      unlistenFocus();
      unlistenFocus = null;
    }
    activeCleanup = null;
  };
}

export function disarmNativeFileDragOut(): void {
  if (activeCleanup) {
    activeCleanup();
  }
}
