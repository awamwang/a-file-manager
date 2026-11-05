// SPDX-License-Identifier: GPL-3.0-or-later
/** Cold-start route from Settings `core.startupPage` (AIA-101). */

export const STARTUP_PAGE_SETTING_KEY = "core.startupPage";

export type StartupPageId = "navigator" | "projects" | "discovery";

const STARTUP_ROUTES: Record<StartupPageId, string> = {
  navigator: "/navigator",
  projects: "/projects",
  discovery: "/discovery",
};

export function parseStartupPage(value: unknown): StartupPageId {
  if (value === "projects" || value === "discovery" || value === "navigator") {
    return value;
  }
  return "navigator";
}

export function startupRouteFor(value: unknown): string {
  return STARTUP_ROUTES[parseStartupPage(value)];
}
