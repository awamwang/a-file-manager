// SPDX-License-Identifier: GPL-3.0-or-later
// Guard: every L2 onboarding hint is bound to the route of the page it describes.
// A deactivated keep-alive view (NavigatorView) must not leave its teleported bubble on screen.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

// The pure matcher is the single source of truth for route scoping.
const scope = read("src/help/hintRouteScope.ts");
for (const symbol of [
  "export function routesForHintEntry",
  "export function isHintPathOnRoutes",
  "export function isHintPageActive",
]) {
  assert.ok(scope.includes(symbol), `hintRouteScope.ts missing: ${symbol}`);
}

// HintBubble must consume that matcher on the current route, on mount, on route
// changes (including while keep-alive deactivated), and on cache reactivation.
const hintBubble = read("src/help/components/HintBubble.vue");
for (const needle of [
  'import { useRoute } from "vue-router";',
  'import { isHintPageActive } from "../hintRouteScope.ts";',
  "const route = useRoute();",
  "isHintPageActive(entry.value, route.path)",
  "watch(pageActive",
  "onDeactivated(",
  "onActivated(",
]) {
  assert.ok(hintBubble.includes(needle), `HintBubble.vue missing route binding: ${needle}`);
}

const refresh = hintBubble.match(/function refreshVisibility\(\): void \{[\s\S]*?\n\}/)?.[0] ?? "";
assert.ok(refresh.length > 0, "HintBubble.vue missing refreshVisibility");
assert.ok(/if\s*\(!hintAllowed\.value\)\s*return/.test(refresh), "refreshVisibility lost hintEnabled gate");
assert.ok(
  /visible\.value\s*=/.test(refresh) &&
    /pageActive\.value/.test(refresh) &&
    /!isSeen\(props\.entryId\)/.test(refresh),
  "visibility is not re-derived from active route + unseen state",
);
assert.ok(
  !/visible\.value\s*=\s*true/.test(hintBubble),
  "HintBubble latches visible=true instead of deriving it from the active route",
);
for (const [hook, pattern] of [
  ["onMounted", /onMounted\(\(\) => \{[\s\S]*?\n\}\)/],
  ["watch", /watch\(pageActive, \(\) => \{[\s\S]*?\n\}\)/],
  ["onActivated", /onActivated\(\(\) => \{[\s\S]*?\n\}\)/],
]) {
  const block = hintBubble.match(pattern)?.[0] ?? "";
  assert.ok(block.length > 0, `${hook} hook missing or changed shape`);
  assert.ok(block.includes("refreshVisibility();"), `${hook} does not refresh hint visibility`);
}
const deactivated = hintBubble.match(/onDeactivated\(\(\) => \{[\s\S]*?\n\}\)/)?.[0] ?? "";
assert.ok(
  deactivated.length > 0 && /visible\.value\s*=\s*false/.test(deactivated),
  "keep-alive deactivation does not hide the teleported hint",
);
assert.ok(
  /<Teleport to="body">[\s\S]*?v-if="visible"/.test(hintBubble),
  "teleported bubble render contract changed; re-check the route-scope fix",
);

// All five hint mount points still pass the entry id that resolves the binding,
// and the registry still publishes a route for each of them.
const registry = read("src/help/registry.ts");
for (const [rel, id] of [
  ["src/views/NavigatorView.vue", "navigator"],
  ["src/views/WorkGroupsView.vue", "work-groups"],
  ["src/views/ProjectsView.vue", "projects"],
  ["src/views/ProvidersView.vue", "providers"],
  ["src/views/AiView.vue", "ai"],
]) {
  assert.ok(
    new RegExp(`<HintBubble\\b[^>]*\\bentry-id="${id}"`).test(read(rel)),
    `${rel} no longer mounts the ${id} hint`,
  );
  const at = registry.indexOf(`id: "${id}",`);
  assert.ok(at >= 0, `registry missing ${id} entry`);
  assert.ok(/route:\s*"/.test(registry.slice(at, at + 600)), `registry ${id} entry has no route binding`);
}

// Execute the exact same pure predicate the component consumes.
const unit = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/hint-route-scope-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (unit.stdout) process.stdout.write(unit.stdout);
if (unit.stderr) process.stderr.write(unit.stderr);
assert.equal(unit.status, 0, "hint-route-scope behavior checks failed");

console.log("hint-route-scope checks OK (switching pages hides the source hint).");
