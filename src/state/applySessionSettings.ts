// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Apply session refs from already-hydrated settings values (sync).
 * Called in main.ts before first Vue paint so Navigator listing / view mode /
 * close behavior do not await another settings round-trip on the critical path.
 */

import { applyCloseBehaviorFromValues } from "./closeBehavior";
import { applyNavigatorViewModeFromValues } from "./navigatorViewMode";
import { applyShowHiddenFromValues } from "./showHidden";

/** Sync-apply listing/chrome settings that affect first interactive UI. */
export function applySessionSettingsFromValues(values: Record<string, unknown>): void {
  applyShowHiddenFromValues(values);
  applyNavigatorViewModeFromValues(values);
  applyCloseBehaviorFromValues(values);
}
