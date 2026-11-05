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
