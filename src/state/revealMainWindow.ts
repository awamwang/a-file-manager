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
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });
}

export async function revealMainWindowAfterReady(): Promise<void> {
  try {
    const shouldReveal = await invoke<boolean>("afm_startup_should_reveal");
    if (!shouldReveal) {
      startupMark("reveal:skipped_autostart_hidden");
      return;
    }

    const win = getCurrentWindow();
    // Config already maximizes while hidden; re-assert in case the HWND
    // restored to the normal size before show (Windows + decorations:false).
    if (!(await win.isMaximized())) {
      await win.maximize();
      // Brief settle so DWM applies maximized bounds before ShowWindow.
      await new Promise<void>((resolve) => {
        setTimeout(resolve, 50);
      });
    }

    await afterNextPaint();
    await win.show();
    await win.setFocus();
    startupMark("reveal:shown");
  } catch {
    // vite-only / missing IPC — nothing to reveal.
  }
}
