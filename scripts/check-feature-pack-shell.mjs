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
  builtinEntries.includes("builtinEntries"),
  "builtinEntries.ts must export entry map",
);
assert.ok(router.includes("getRouteExtras"), "router must consume getRouteExtras");
assert.ok(router.includes("addRoute"), "pack routes must use router.addRoute");
assert.ok(router.includes("AI_ADDON_ENABLED"), "router must keep AI_ADDON_ENABLED");
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
