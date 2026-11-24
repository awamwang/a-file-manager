// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Lightweight cold-start marks (`[AFM_startup]`).
 * Enable the console output with `localStorage.setItem("afm.startupTiming", "1")`
 * then reload; marks are always recorded in-memory (a handful of numbers) so
 * the `--afm-perf-baseline` runner can dump the chain without racing boot.
 */

const ORIGIN = performance.now();

const marks: Array<{ label: string; ms: number }> = [];

function enabled(): boolean {
  try {
    return globalThis.localStorage?.getItem("afm.startupTiming") === "1";
  } catch {
    return false;
  }
}

/** Record a named startup milestone (always recorded; printed when enabled). */
export function startupMark(label: string): void {
  const ms = performance.now() - ORIGIN;
  marks.push({ label, ms });
  if (!enabled()) return;
  console.info(`[AFM_startup] +${ms.toFixed(1)}ms ${label}`);
}

/** Recorded chain (S1) for the perf-baseline runner. */
export function startupMarks(): ReadonlyArray<{ label: string; ms: number }> {
  return marks;
}
