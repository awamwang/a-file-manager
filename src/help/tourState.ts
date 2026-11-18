// SPDX-License-Identifier: GPL-3.0-or-later
/** Global active-tour handle: Help Center, deep links and first-run share one slot. */

import { ref } from "vue";
import type { TourStep } from "./types.ts";

export type ActiveTour = {
  id: string;
  steps: readonly TourStep[];
  /** Replay must not change progress; first-run playback does. */
  recordProgress: boolean;
};

export const activeTour = ref<ActiveTour | null>(null);

/** Polling budget for a lazy view's first anchor to appear. */
const ANCHOR_POLL_MS = 60;
const ANCHOR_WAIT_TIMEOUT_MS = 3000;

/** Supersedes older pending launches; every stop cancels its own pending poll. */
let launchToken = 0;

/** Start (or replace) the single visible guided tour. */
export function startTour(id: string, steps: readonly TourStep[], recordProgress: boolean): void {
  if (!steps.length) return;
  activeTour.value = { id, steps, recordProgress };
}

/** Close the active tour without touching progress. */
export function stopTour(): void {
  launchToken += 1;
  activeTour.value = null;
}

/** True when the anchor exists and has a measurable box. */
function anchorReady(anchor: string): boolean {
  if (!anchor || typeof document === "undefined") return false;
  try {
    const element = document.querySelector<HTMLElement>(
      `[data-tour-id="${CSS.escape(anchor)}"]`,
    );
    if (!element) return false;
    const box = element.getBoundingClientRect();
    return box.width > 0 || box.height > 0;
  } catch {
    return false;
  }
}

/**
 * Navigate-then-play bridge for replay / feature deep links.
 *
 * The Help Center unmounts as soon as `router.push()` leaves `/help`, so the
 * wait loop lives at module scope (not in a component). It polls only for a
 * bounded time and aborts if a newer launch starts or the route changes.
 */
export function startTourWhenAnchorReady(
  anchor: string,
  expectedPath: string,
  onReady: () => void,
): void {
  const token = ++launchToken;
  const deadline = Date.now() + ANCHOR_WAIT_TIMEOUT_MS;

  const poll = (): void => {
    if (token !== launchToken) return;
    if (
      Date.now() > deadline ||
      (expectedPath && window.location.pathname !== expectedPath)
    ) {
      // Abandon this pending launch only. Never stopTour() here: this launch has
      // not started a tour yet, so any active tour belongs to someone else
      // (e.g. the first-run quickstart) and must keep playing.
      return;
    }
    if (anchorReady(anchor)) {
      onReady();
      return;
    }
    window.setTimeout(poll, ANCHOR_POLL_MS);
  };

  window.setTimeout(poll, 0);
}
