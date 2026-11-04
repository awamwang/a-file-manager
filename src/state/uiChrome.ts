// SPDX-License-Identifier: GPL-3.0-or-later

import { invoke } from "@tauri-apps/api/core";

export type UiTheme = "light" | "dark" | "system";
export type ListDensity = "standard" | "compact";

type SettingsDocument = {
  version: number;
  values: Record<string, unknown>;
};

let currentTheme: UiTheme = "system";
let currentDensity: ListDensity = "standard";

export function getUiTheme(): UiTheme {
  return currentTheme;
}

export function getListDensity(): ListDensity {
  return currentDensity;
}

function asTheme(value: unknown): UiTheme {
  if (value === "light" || value === "dark" || value === "system") return value;
  return "system";
}

function asDensity(value: unknown): ListDensity {
  return value === "compact" ? "compact" : "standard";
}

/** Apply theme + list density to <html> for CSS selectors. Chrome never uses Compact. */
export function applyUiChrome(theme: UiTheme, density: ListDensity): void {
  currentTheme = theme;
  currentDensity = density;

  const root = document.documentElement;
  if (theme === "system") {
    delete root.dataset.theme;
    root.style.colorScheme = "";
  } else {
    root.dataset.theme = theme;
    root.style.colorScheme = theme;
  }

  if (density === "compact") {
    root.dataset.listDensity = "compact";
  } else {
    delete root.dataset.listDensity;
  }
}

export function applyUiChromeFromValues(values: Record<string, unknown>): void {
  applyUiChrome(asTheme(values["core.theme"]), asDensity(values["core.listDensity"]));
}

/** Load settings document and sync chrome tokens. Safe fallback if IPC unavailable. */
export async function loadUiChromeFromSettings(): Promise<void> {
  try {
    const doc = await invoke<SettingsDocument>("settings_get");
    applyUiChromeFromValues(doc.values ?? {});
    return;
  } catch {
    applyUiChrome("system", "standard");
  }
}
