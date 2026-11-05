// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-79: Navigator-owned chrome layout (Files-like).
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const settings = readFileSync(join(root, "src/views/SettingsView.vue"), "utf8");
const router = readFileSync(join(root, "src/router/index.ts"), "utf8");

if (!shell.includes("isNavigatorRoute")) {
  console.error("AppShell missing isNavigatorRoute gate");
  process.exit(1);
}
if (!shell.includes('v-if="isNavigatorRoute"') && !shell.includes("v-if=\"isNavigatorRoute\"")) {
  console.error("AppShell must gate navigator chrome with v-if=isNavigatorRoute");
  process.exit(1);
}
// AIA-91: chrome lives in right workspace, not above full-window body.
if (!shell.includes('class="workspace"')) {
  console.error("AppShell missing workspace column for navigator chrome");
  process.exit(1);
}
{
  const headerEnd = shell.indexOf("</header>");
  const body = shell.indexOf('class="body"');
  const between = shell.slice(headerEnd, body);
  if (between.includes("navigator-chrome")) {
    console.error("Navigator chrome must not sit between titlebar and body");
    process.exit(1);
  }
}
for (const needle of [
  "navigator-chrome",
  "nav-icon-btn",
  "places-menu",
  "navigator-home-btn",
  "navigator-work-group-menu",
  "titlebar-actions",
  "toggleCommandPanel",
  "toggleDualPane",
  "favoritesDoc",
  "quickAccessList",
]) {
  if (!shell.includes(needle)) {
    console.error(`AppShell missing ${needle}`);
    process.exit(1);
  }
}

// Nav buttons must be icon-only in navigator chrome (no Back/Forward/Up text spans).
const chromeBlock = shell.slice(
  shell.indexOf("navigator-chrome"),
  shell.indexOf("tabstrip"),
);
if (/nav-icon-btn[\s\S]*?<span>\{\{\s*t\("chrome\.(back|forward|up)"\)/.test(chromeBlock)) {
  console.error("Navigator chrome still shows text labels on Back/Forward/Up");
  process.exit(1);
}

// Sidenav must not host QA / favorites / OS verb.
if (shell.includes("fav-os-hook") || shell.includes("class=\"quick-access\"")) {
  console.error("AppShell sidenav still hosts Quick Access / OS favorites verb");
  process.exit(1);
}

if (!settings.includes("settings/favorites") || !settings.includes("setFavoritesOsHookEnabled")) {
  console.error("Settings missing OS favorites verb panel");
  process.exit(1);
}
if (!router.includes("/settings/favorites")) {
  console.error("Router missing /settings/favorites");
  process.exit(1);
}
if (!settings.includes("settings/open-tools") || !settings.includes("ide_list_apps")) {
  console.error("Settings missing Open tools section");
  process.exit(1);
}
if (!router.includes("/settings/open-tools")) {
  console.error("Router missing /settings/open-tools");
  process.exit(1);
}
if (!shell.includes("setNavigatorViewMode") || !shell.includes("viewDetails")) {
  console.error("AppShell missing Windows-style list/details view toggle");
  process.exit(1);
}

console.log("navigator-chrome-layout: ok");
