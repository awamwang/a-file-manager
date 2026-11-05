// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-181 P0: settings/WG hydrate bus, tabs-first OpenState, deferred preload/launchers.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const hydrate = read("src/state/settingsHydrate.ts");
const idle = read("src/state/scheduleIdle.ts");
const browse = read("src/state/navigatorBrowse.ts");
const wgChrome = read("src/state/workGroupChrome.ts");
const app = read("src/App.vue");
const shell = read("src/components/shell/AppShell.vue");
const menu = read("src/components/navigator/NavigatorContextMenu.vue");
const providersView = read("src/views/ProvidersView.vue");
const providerCache = read("src/state/providerCache.ts");
const closeBehavior = read("src/state/closeBehavior.ts");
const viewMode = read("src/state/navigatorViewMode.ts");
const uiChrome = read("src/state/uiChrome.ts");

// --- settings hydrate bus ---
assert.ok(hydrate.includes("export function hydrateSettings"), "settingsHydrate must export hydrateSettings");
assert.ok(hydrate.includes("rememberSettingsDocument"), "settingsHydrate must export remember");
assert.ok(hydrate.includes("settings_get"), "hydrate must call settings_get once");
assert.ok(app.includes("hydrateSettings"), "App.vue must use hydrateSettings");
assert.ok(closeBehavior.includes("hydrateSettings"), "closeBehavior must share hydrate");
assert.ok(viewMode.includes("hydrateSettings"), "navigatorViewMode must share hydrate");
assert.ok(uiChrome.includes("hydrateSettings"), "uiChrome must share hydrate");
assert.ok(closeBehavior.includes("rememberSettingsDocument"), "closeBehavior must remember after set");
assert.ok(viewMode.includes("rememberSettingsDocument"), "viewMode must remember after set");

// --- WG hydrate bus ---
assert.ok(browse.includes("export function hydrateWorkGroups"), "browse must export hydrateWorkGroups");
assert.ok(browse.includes("rememberHydratedWorkGroups"), "browse must export rememberHydratedWorkGroups");
assert.ok(wgChrome.includes("hydrateWorkGroups"), "workGroupChrome must use hydrateWorkGroups");
assert.ok(
  /loadWorkGroupsChrome\([\s\S]*?force:\s*true/.test(shell) ||
    shell.includes("loadWorkGroupsChrome({ force: true })"),
  "WG menu open must force fresh disk read",
);

// --- tabs-first OpenState bootstrap ---
{
  const bootStart = browse.indexOf("export function bootstrapNavigatorOpenState");
  assert.ok(bootStart >= 0, "bootstrapNavigatorOpenState must exist");
  const bootEnd = browse.indexOf("\nexport async function navigateTo", bootStart);
  const boot = browse.slice(bootStart, bootEnd > bootStart ? bootEnd : browse.length);
  assert.ok(
    /listLoading\.value\s*=\s*true/.test(boot),
    "bootstrap must set listLoading before listing",
  );
  assert.ok(
    /activeTab\?\.path[\s\S]*?void navigateTo\(/.test(boot),
    "bootstrap must not await navigateTo for restored tab (tabs paint first)",
  );
  assert.ok(
    !/activeTab\?\.path[\s\S]*?await navigateTo\(/.test(boot),
    "restored-tab path must not block bootstrap on await navigateTo",
  );
}

// --- provider preload deferred ---
assert.ok(idle.includes("requestIdleCallback") || idle.includes("setTimeout"), "scheduleIdle helper");
assert.ok(app.includes("scheduleIdle"), "App.vue must scheduleIdle preload");
assert.ok(
  /scheduleIdle\(\(\)\s*=>\s*\{[\s\S]*?preloadProviderCache/.test(app),
  "preload must run inside scheduleIdle, not immediately on mount",
);
assert.ok(
  !/void preloadProviderCache\(\);\s*\}\)\(\);\s*\}\);/.test(app.replace(/\s+/g, " ")),
  "App must not fire preload immediately after startup routing",
);
assert.ok(
  providerCache.includes("idle after app start") || providerCache.includes("scheduleIdle"),
  "providerCache docs must describe idle wire",
);
assert.ok(
  providersView.includes("preloadProviderCache"),
  "ProvidersView must warm on first visit as safety net",
);

// --- launcher warm not on Navigator mount ---
assert.ok(
  /watch\(\s*\(\)\s*=>\s*props\.open[\s\S]*?preloadLaunchersIfNeeded\(\)/.test(menu),
  "context menu must warm launchers when open becomes true",
);
{
  const mountIdx = menu.indexOf("onMounted(() => {");
  assert.ok(mountIdx >= 0, "NavigatorContextMenu must have onMounted");
  const mountEnd = menu.indexOf("});", mountIdx);
  const mountBody = menu.slice(mountIdx, mountEnd);
  assert.ok(
    !mountBody.includes("preloadLaunchersIfNeeded();"),
    "onMounted must not call preloadLaunchersIfNeeded (AIA-181)",
  );
}

// --- P1: Discovery estimate deferred ---
{
  const discoveryMod = read("src-tauri/src/discovery/mod.rs");
  const discoveryService = read("src-tauri/src/discovery/service.rs");
  const discoveryView = read("src/views/DiscoveryView.vue");
  assert.ok(discoveryMod.includes("discovery_estimate_count"), "estimate command registered");
  assert.ok(
    /estimated_item_count:\s*None/.test(discoveryService),
    "status() must leave estimated_item_count None",
  );
  assert.ok(discoveryView.includes("discovery_estimate_count"), "UI must fetch estimate separately");
  assert.ok(
    /void refreshEstimate\(/.test(discoveryView),
    "estimate must be fire-and-forget after light status",
  );
}

// --- P1: non-default routes lazy-loaded ---
{
  const router = read("src/router/index.ts");
  assert.ok(
    /import NavigatorView from ["'].*NavigatorView\.vue["']/.test(router),
    "Navigator (default startup) stays eager",
  );
  assert.ok(
    !/import DiscoveryView from/.test(router),
    "Discovery must not be a static import",
  );
  assert.ok(
    /component:\s*\(\)\s*=>\s*import\(["'].*DiscoveryView\.vue["']\)/.test(router),
    "Discovery must use dynamic import()",
  );
  assert.ok(
    /component:\s*\(\)\s*=>\s*import\(["'].*ProvidersView\.vue["']\)/.test(router),
    "Providers must use dynamic import()",
  );
  assert.ok(
    /component:\s*\(\)\s*=>\s*import\(["'].*SettingsView\.vue["']\)/.test(router),
    "Settings must use dynamic import()",
  );
  assert.ok(
    /component:\s*\(\)\s*=>\s*import\(["'].*ProjectsView\.vue["']\)/.test(router),
    "Projects must use dynamic import()",
  );
}

console.log("startup-async checks OK (AIA-181/182).");
