// SPDX-License-Identifier: GPL-3.0-or-later
/** AIA-276: Settings / primary view switch must not block first paint on remount IPC. */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const hydrate = read("src/state/settingsHydrate.ts");
const settings = read("src/views/SettingsView.vue");
const app = read("src/App.vue");
const shell = read("src/components/shell/AppShell.vue");
const nav = read("src/views/NavigatorView.vue");
const wg = read("src/views/WorkGroupsView.vue");

// --- settings hydrate: schema single-flight ---
assert.ok(
  hydrate.includes("export function hydrateSettingsSchema"),
  "must export hydrateSettingsSchema",
);
assert.ok(
  hydrate.includes("export function getCachedSettingsSchema"),
  "must export getCachedSettingsSchema",
);
assert.ok(hydrate.includes("settings_schema_get"), "schema hydrate calls settings_schema_get");

// --- SettingsView: optimistic paint + deferred non-critical IPC ---
assert.ok(
  settings.includes("getCachedSettingsDocument"),
  "Settings must paint from document cache",
);
assert.ok(
  settings.includes("getCachedSettingsSchema") || settings.includes("hydrateSettingsSchema"),
  "Settings must reuse schema hydrate/cache",
);
assert.ok(settings.includes("paintSettingsFromCache"), "optimistic paint helper");
assert.ok(settings.includes("scheduleAfterPaint"), "defer non-critical work after paint");
assert.ok(
  settings.includes("defineAsyncComponent"),
  "HealthView must be async so it stays off default Settings chunk",
);
{
  const loadStart = settings.indexOf("async function loadAll");
  assert.ok(loadStart >= 0, "loadAll required");
  const loadEnd = settings.indexOf("\nfunction valueOf", loadStart);
  const loadBody = settings.slice(loadStart, loadEnd > loadStart ? loadEnd : loadStart + 1200);
  assert.ok(
    loadBody.includes("scheduleAfterPaint"),
    "autostart sync must not await on Settings entry critical path",
  );
  assert.ok(
    /applyLaunchAtStartupPreference/.test(loadBody),
    "autostart still synced (deferred)",
  );
  assert.ok(
    !/await applyLaunchAtStartupPreference/.test(loadBody),
    "must not await autostart on loadAll critical path",
  );
}
{
  const mountStart = settings.indexOf("onMounted(");
  assert.ok(mountStart >= 0);
  const mountBody = settings.slice(mountStart, mountStart + 600);
  assert.ok(
    mountBody.includes('activeSection.value === "power"'),
    "OS-hook status must be gated to Power section (FIL-65)",
  );
  assert.ok(
    !/onMounted\(\(\) => \{[\s\S]*void loadFavoritesOsHookStatus\(\)/.test(mountBody) ||
      mountBody.includes('activeSection.value === "power"'),
    "must not unconditionally load favorites OS hook on every Settings open",
  );
}

// --- Keep Explorer warm ---
assert.ok(app.includes("KeepAlive"), "App must KeepAlive Navigator across route switches");
assert.ok(app.includes("NavigatorView"), "KeepAlive include NavigatorView");
assert.ok(nav.includes('name: "NavigatorView"'), "NavigatorView must declare KeepAlive name");
assert.ok(nav.includes("onActivated"), "KeepAlive: re-register file shortcuts on activate");
assert.ok(nav.includes("onDeactivated"), "KeepAlive: drop file shortcuts on deactivate");
assert.ok(
  nav.includes("registerNavFileShortcuts"),
  "Navigator file shortcuts still registered",
);
assert.ok(
  shell.includes('v-show="isNavigatorRoute"'),
  "FolderTree / chrome must use v-show to stay warm",
);
assert.ok(
  shell.includes("navigator-chrome-host"),
  "navigator chrome host wrapper for warm keep",
);

// --- Work Groups optimistic paint (same slice, cheap) ---
assert.ok(wg.includes("paintWorkGroupsFromCache"), "WG view paints from chrome cache");
assert.ok(wg.includes("workGroupsDoc"), "WG view reuses workGroupsDoc");
assert.ok(wg.includes("hydrateWorkGroups"), "WG refresh shares hydrate bus");

console.log("Settings / view-switch lag checks OK (AIA-276).");
