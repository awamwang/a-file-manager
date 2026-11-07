// SPDX-License-Identifier: GPL-3.0-or-later
/** Run work after the browser is idle (fallback: short timeout). */

type IdleDeadlineLike = { didTimeout: boolean; timeRemaining: () => number };

type RequestIdleCallback = (
  callback: (deadline: IdleDeadlineLike) => void,
  options?: { timeout?: number },
) => number;

/** Schedule `fn` via `requestIdleCallback`, or `setTimeout` when unavailable. */
export function scheduleIdle(fn: () => void, timeoutMs = 4000): void {
  const ric = (globalThis as unknown as { requestIdleCallback?: RequestIdleCallback })
    .requestIdleCallback;
  if (typeof ric === "function") {
    ric(() => {
      fn();
    }, { timeout: timeoutMs });
    return;
  }
  setTimeout(fn, Math.min(2000, timeoutMs));
}

/**
 * Run `fn` after the browser has painted (double rAF), or on the next macrotask.
 * AIA-259: keep side-tree / status IPC off the WG/tab content first-paint path.
 */
export function scheduleAfterPaint(fn: () => void): void {
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(() => {
      requestAnimationFrame(fn);
    });
    return;
  }
  setTimeout(fn, 0);
}
