// SPDX-License-Identifier: GPL-3.0-or-later
/** Feature Pack 壳合并：侧栏 extras、hydrate IPC、路由/命令接线。 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const sideNav = read("src/shell/sideNav.ts");
const contributions = read("src/state/featurePack/contributions.ts");
const builtinEntries = read("src/state/featurePack/builtinEntries.ts");
const router = read("src/router/index.ts");
const registry = read("src/commands/registry.ts");
const main = read("src/main.ts");
const reload = read("src/state/uiReload.ts");

assert.ok(
  sideNav.includes("getSideNavExtras"),
  "sideNav.ts must call getSideNavExtras",
);
assert.ok(
  sideNav.includes("AI_ADDON_ENABLED"),
  "sideNav must keep AI_ADDON_ENABLED filtering",
);
assert.ok(
  contributions.includes("hydrateFeaturePackContributions"),
  "contributions.ts must export hydrateFeaturePackContributions",
);
assert.ok(
  contributions.includes("extension_host_list_contributions"),
  "hydrate must invoke extension_host_list_contributions",
);
assert.ok(
  contributions.includes("getRouteExtras") && contributions.includes("builtinEntries"),
  "routes register only when entry exists in builtinEntries",
);
assert.ok(
  contributions.includes("resolvableContributionPaths"),
  "contributions must build resolvable path set for shell + pack routes",
);
assert.ok(
  contributions.includes("isResolvableNavTo"),
  "sideNav extras must filter unknown targets",
);
assert.ok(
  contributions.includes("isResolvableCommandRoute"),
  "command extras must filter routes that would 404",
);
assert.ok(
  contributions.includes("AI_ADDON_ENABLED"),
  "resolvable paths must respect AI_ADDON_ENABLED core routes",
);
assert.ok(
  builtinEntries.includes("builtinEntries"),
  "builtinEntries.ts must export entry map",
);
assert.ok(
  router.includes("applyFeaturePackRoutes"),
  "router must register pack routes via applyFeaturePackRoutes",
);
assert.ok(router.includes("AI_ADDON_ENABLED"), "router must keep AI_ADDON_ENABLED");
assert.ok(
  contributions.includes("registeredPackRouteNames"),
  "must track added pack route names",
);
assert.ok(
  contributions.includes("removeRoute") && contributions.includes("addRoute"),
  "re-hydrate must remove stale pack routes before adding current ones",
);
assert.ok(
  contributions.includes("applyFeaturePackSideNav"),
  "hydrate must rebuild SIDE_NAV_ITEMS via applyFeaturePackSideNav",
);
assert.ok(
  sideNav.includes("SIDE_NAV_ITEMS.splice"),
  "applyFeaturePackSideNav must splice SIDE_NAV_ITEMS on re-hydrate",
);
{
  const fnStart = reload.indexOf("export async function reloadWindow");
  assert.ok(fnStart >= 0, "uiReload.ts must export reloadWindow");
  const fn = reload.slice(fnStart);
  const hydrateIdx = fn.indexOf("hydrateFeaturePackContributions");
  const remountIdx = fn.indexOf("shellRemountKey");
  assert.ok(
    hydrateIdx >= 0,
    "reloadWindow must re-call hydrateFeaturePackContributions",
  );
  assert.ok(
    remountIdx > hydrateIdx,
    "reloadWindow must hydrate Feature Pack contributions before remount",
  );
}
assert.ok(
  registry.includes("getCommandExtras"),
  "command registry must merge getCommandExtras",
);
assert.ok(
  registry.includes("AI_ADDON_ENABLED"),
  "commands must keep AI_ADDON_ENABLED filtering",
);
{
  const mountIdx = main.indexOf('.mount("#app")');
  assert.ok(mountIdx >= 0, "main.ts must mount #app");
  assert.ok(
    main.slice(0, mountIdx).includes("hydrateFeaturePackContributions"),
    "main.ts must hydrate Feature Pack contributions before mount",
  );
}

console.log("feature-pack-shell: ok");
