// SPDX-License-Identifier: GPL-3.0-or-later
/** Settings `core.newTabBehavior`: guide page vs duplicate active tab (AIA-148). */

import { hydrateSettings } from "./settingsHydrate";

export const NEW_TAB_BEHAVIOR_SETTING_KEY = "core.newTabBehavior";

export type NewTabBehavior = "guide" | "duplicate";

export function parseNewTabBehavior(value: unknown): NewTabBehavior {
  return value === "duplicate" ? "duplicate" : "guide";
}

export async function readNewTabBehavior(): Promise<NewTabBehavior> {
  try {
    const doc = await hydrateSettings();
    return parseNewTabBehavior(doc.values[NEW_TAB_BEHAVIOR_SETTING_KEY]);
  } catch {
    return "guide";
  }
}
