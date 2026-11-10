// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AIA-242: hover-to-expand (tree) / hover-to-activate (tabs) while file-dragging.
 * Matches Explorer / Sigma ~500ms dwell before the secondary action.
 */

export const DROP_HOVER_MS = 500;

/** One pending hover key at a time; reschedule only when the key changes. */
export function createDropHoverScheduler(delayMs = DROP_HOVER_MS) {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let activeKey: string | null = null;

  return {
    /** Arm `action` after dwell on `key`. No-op if already armed for the same key. */
    schedule(key: string, action: () => void): void {
      if (activeKey === key) return;
      this.clear();
      activeKey = key;
      timer = setTimeout(() => {
        timer = null;
        action();
      }, delayMs);
    },
    clear(): void {
      if (timer != null) {
        clearTimeout(timer);
        timer = null;
      }
      activeKey = null;
    },
    get key(): string | null {
      return activeKey;
    },
  };
}
