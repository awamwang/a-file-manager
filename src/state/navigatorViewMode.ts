// SPDX-License-Identifier: GPL-3.0-or-later
/** Windows-style Navigator view: List (name only) vs Details (size + date modified). */

import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type NavigatorViewMode = "list" | "details";

export const navigatorViewMode = ref<NavigatorViewMode>("details");

let loadPromise: Promise<void> | null = null;

function coerceMode(value: unknown): NavigatorViewMode {
  return value === "list" ? "list" : "details";
}

export async function loadNavigatorViewMode(): Promise<NavigatorViewMode> {
  if (!loadPromise) {
    loadPromise = (async () => {
      try {
        const doc = await invoke<{ values: Record<string, unknown> }>("settings_get");
        navigatorViewMode.value = coerceMode(doc.values["core.navigatorViewMode"]);
      } catch {
        navigatorViewMode.value = "details";
      }
    })().finally(() => {
      loadPromise = null;
    });
  }
  await loadPromise;
  return navigatorViewMode.value;
}

export async function setNavigatorViewMode(mode: NavigatorViewMode): Promise<void> {
  navigatorViewMode.value = mode;
  try {
    await invoke("settings_set", {
      patch: { "core.navigatorViewMode": mode },
    });
  } catch {
    // Keep local mode even if persist fails; next load may revert.
  }
}
