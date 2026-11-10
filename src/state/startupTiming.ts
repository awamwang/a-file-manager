// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Lightweight cold-start marks (`[AFM_startup]`).
 * Enable with `localStorage.setItem("afm.startupTiming", "1")` then reload;
 * always no-ops when disabled so release builds stay quiet.
 */

const ORIGIN = performance.now();

function enabled(): boolean {
  try {
    return globalThis.localStorage?.getItem("afm.startupTiming") === "1";
  } catch {
    return false;
  }
}

/** Record a named startup milestone when timing is enabled. */
export function startupMark(label: string): void {
  if (!enabled()) return;
  const ms = (performance.now() - ORIGIN).toFixed(1);
  console.info(`[AFM_startup] +${ms}ms ${label}`);
}
