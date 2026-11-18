// SPDX-License-Identifier: GPL-3.0-or-later
/** First-run quick-start playback — deliberately independent of settings hydration. */

import { onMounted, onUnmounted } from "vue";
import { useRoute } from "vue-router";
import {
  QUICK_START_TOUR_ID,
  tourRegistry,
} from "./registry.ts";
import { isDone, isSeen } from "./progress.ts";
import { activeTour, startTour } from "./tourState.ts";

const FIRST_RUN_DELAY_MS = 1800;

/**
 * Call from AppShell setup. Schedules one delayed playback; it never awaits IPC,
 * never writes settings and is a no-op when progress already exists.
 */
export function useFirstRunTour(): void {
  let timer: number | undefined;
  const route = useRoute();

  onMounted(() => {
    // Never spend the first-run playback on a custom startup page; wait for a
    // later launch that opens Explorer instead of interrupting that flow.
    if (route.path !== "/" && route.path !== "/navigator") return;
    if (isDone(QUICK_START_TOUR_ID) || isSeen(QUICK_START_TOUR_ID)) return;
    timer = window.setTimeout(() => {
      if (isDone(QUICK_START_TOUR_ID) || isSeen(QUICK_START_TOUR_ID)) return;
      if (activeTour.value) return;
      const definition = tourRegistry[QUICK_START_TOUR_ID];
      if (!definition) return;
      startTour(QUICK_START_TOUR_ID, definition.steps, true);
    }, FIRST_RUN_DELAY_MS);
  });

  onUnmounted(() => {
    if (timer !== undefined) window.clearTimeout(timer);
  });
}
