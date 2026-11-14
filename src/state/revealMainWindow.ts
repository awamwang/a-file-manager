// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AIA-272: reveal the main window only after the first Vue paint.
 * Window starts `visible: false` (+ maximized) so WebView2 black / maximize
 * geometry never flash on screen. Autostart-hidden skips reveal (tray restore).
 */
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { startupMark } from "./startupTiming.ts";

function afterNextPaint(): Promise<void> {
  return new Promise((resolve) => {
    // rAF is throttled/paused while the window is still hidden (WKWebView on
    // macOS), which would deadlock the reveal: hidden -> rAF never fires ->
    // show() never called -> stays hidden. A macrotask timer always fires, so
    // use it as a fallback to guarantee the window is revealed.
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
    setTimeout(resolve, 100);
  });
}

export async function revealMainWindowAfterReady(): Promise<void> {
  try {
    const shouldReveal = await invoke<boolean>("afm_startup_should_reveal");
    if (!shouldReveal) {
      startupMark("reveal:skipped_autostart_hidden");
      return;
    }

    // macOS/WKWebView: a borderless (decorations:false) window created with
    // `visible:false` is not reliably promoted by `maximize()` — the call throws
    // on a hidden borderless NSWindow and would abort the reveal before `show()`.
    // Skip the maximize dance there and just show after paint (config already
    // requests `maximized`, which WKWebView applies on show). Windows keeps the
    // DWM maximize-reassert to avoid the geometry flash.
    const ua =
      typeof navigator !== "undefined" ? navigator.userAgent || "" : "";
    const isMac = /Mac|iPhone|iPad|iPod/i.test(ua);

    const win = getCurrentWindow();
    if (!isMac) {
      try {
        if (!(await win.isMaximized())) {
          await win.maximize();
          // Brief settle so DWM applies maximized bounds before ShowWindow.
          await new Promise<void>((resolve) => {
            setTimeout(resolve, 50);
          });
        }
      } catch {
        // Non-fatal: proceed to show regardless of maximize state.
      }
    }

    await afterNextPaint();
    await win.show();
    await win.setFocus();
    startupMark("reveal:shown");
  } catch {
    // vite-only / missing IPC — nothing to reveal.
  }
}
