// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Native file drag-out (Windows OLE / CF_HDROP via tauri-plugin-drag).
 * Keeps HTML5 in-app drops intact; starts OS drag when the cursor leaves the window
 * so external apps (players, Explorer, editors) receive real file paths.
 */
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { resolveResource } from "@tauri-apps/api/path";
import { getCurrentWindow } from "@tauri-apps/api/window";

let outboundBusy = false;
let cachedDragIconPath: string | null = null;
let activeCleanup: (() => void) | null = null;

async function dragIconPath(): Promise<string> {
  if (!cachedDragIconPath) {
    cachedDragIconPath = await resolveResource("icons/32x32.png");
  }
  return cachedDragIconPath;
}

function isOutsideViewport(clientX: number, clientY: number): boolean {
  return (
    clientX <= 0 ||
    clientY <= 0 ||
    clientX >= window.innerWidth ||
    clientY >= window.innerHeight
  );
}

/** Start Windows-native OLE file drag so drop targets outside AFM get CF_HDROP paths. */
export async function startNativeFileDragOut(paths: string[]): Promise<void> {
  const cleaned = paths.map((p) => p.trim()).filter(Boolean);
  if (outboundBusy || cleaned.length === 0) return;
  outboundBusy = true;
  try {
    const icon = await dragIconPath();
    await startDrag({
      item: cleaned,
      icon,
      // Copy: players / external apps should not move files out of the listing.
      mode: "copy",
    });
  } catch (err) {
    console.error("native file drag-out failed", err);
  } finally {
    outboundBusy = false;
  }
}

/**
 * While an HTML5 drag is active, watch for leave-window / blur and switch to
 * native OLE drag-out (Sigma-style hybrid).
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
    if (isOutsideViewport(event.clientX, event.clientY)) {
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
