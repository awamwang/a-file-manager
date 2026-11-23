// SPDX-License-Identifier: GPL-3.0-or-later
// Behavioral unit for the L2 hint route-scope gate. Pinned by scripts/check-hint-route-scope.mjs.
// The Vue lifecycle itself cannot run headless here (no e2e per ADR 0025), so this pins the
// pure predicate HintBubble uses; the .mjs pins that the component actually consumes it.
import assert from "node:assert/strict";
import {
  isHintPageActive,
  isHintPathOnRoutes,
  routesForHintEntry,
} from "../src/help/hintRouteScope.ts";

const BINDINGS = {
  navigator: { route: "/navigator", applicability: { routes: ["/navigator", "/"] } },
  "work-groups": { route: "/work-groups", applicability: { routes: ["/work-groups"] } },
  projects: { route: "/projects", applicability: { routes: ["/projects"] } },
  providers: {
    route: "/extensions/providers",
    applicability: { routes: ["/extensions/providers", "/providers"] },
  },
  ai: { route: "/ai", applicability: { routes: ["/ai"] } },
} as const;

// Registration lookup: an unknown entry must never light up a bubble.
assert.deepEqual(routesForHintEntry(undefined), []);
assert.deepEqual(routesForHintEntry({}), []);
assert.deepEqual(routesForHintEntry({ route: "/projects" }), ["/projects"]);
assert.deepEqual(routesForHintEntry({ route: "/projects", applicability: { routes: [] } }), [
  "/projects",
]);
assert.deepEqual(routesForHintEntry(BINDINGS.providers), [
  "/extensions/providers",
  "/providers",
]);

// Path matcher: exact + child only, no empty path, no sibling-prefix false positive.
assert.equal(isHintPathOnRoutes([], "/navigator"), false);
assert.equal(isHintPathOnRoutes(["/navigator"], ""), false);
assert.equal(isHintPathOnRoutes(["/navigator"], "/navigator"), true);
assert.equal(isHintPathOnRoutes(["/navigator"], "/navigator/tabs/one"), true);
assert.equal(isHintPathOnRoutes(["/navigator"], "/work-groups"), false);
assert.equal(isHintPathOnRoutes(["/navigator"], "/navigator-extra"), false);
assert.equal(isHintPathOnRoutes(["/"], "/"), true);
assert.equal(isHintPathOnRoutes(["/"], "/work-groups"), false);

// Every hint is active only on its own page: full 5 x 5 cross-page matrix.
const ids = Object.keys(BINDINGS);
for (const id of ids) {
  const binding = BINDINGS[id];
  for (const route of routesForHintEntry(binding)) {
    assert.equal(
      isHintPageActive(binding, route),
      true,
      `${id} hint must show on its own route ${route}`,
    );
    assert.equal(
      isHintPageActive(binding, `${route}/child`),
      true,
      `${id} hint must show on a child route of ${route}`,
    );
  }
  for (const other of ids) {
    if (other === id) continue;
    for (const route of routesForHintEntry(BINDINGS[other])) {
      assert.equal(
        isHintPageActive(binding, route),
        false,
        `${id} hint leaked onto ${other}'s route ${route}`,
      );
    }
  }
  // The Help Center is a different surface; it must never carry a page hint.
  assert.equal(isHintPageActive(binding, "/help"), false, `${id} hint leaked onto /help`);
  assert.equal(isHintPageActive(binding, "/help/article"), false, `${id} hint leaked into /help/`);
}

// Provider hint follows the embedded Extensions section, not the base /extensions page.
assert.equal(isHintPageActive(BINDINGS.providers, "/extensions"), false);
assert.equal(isHintPageActive(BINDINGS.providers, "/extensions/store"), false);
assert.equal(isHintPageActive(BINDINGS.providers, "/extensions/providers"), true);
assert.equal(isHintPageActive(BINDINGS.providers, "/providers"), true);

// The exact defect: a visible Navigator hint must vanish after switching to WorkGroups,
// and must not reappear on an unrelated page even if it was never dismissed.
let navigatorHintVisible = isHintPageActive(BINDINGS.navigator, "/navigator");
assert.equal(navigatorHintVisible, true);
navigatorHintVisible = isHintPageActive(BINDINGS.navigator, "/work-groups");
assert.equal(navigatorHintVisible, false, "Navigator hint stayed visible on WorkGroups page");
navigatorHintVisible = isHintPageActive(BINDINGS.navigator, "/projects");
assert.equal(navigatorHintVisible, false, "Navigator hint stayed visible on Projects page");
// Returning to the page (keep-alive reactivation) restores it while still unseen.
navigatorHintVisible = isHintPageActive(BINDINGS.navigator, "/navigator");
assert.equal(navigatorHintVisible, true, "unseen hint must return when its page is active again");

console.log("hint-route-scope-checks OK (L2 hints are bound to their own page route).");
