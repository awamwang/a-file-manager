// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-88: Navigator-only embedded folder filter bar.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const state = readFileSync(join(root, "src/state/navigatorFilter.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(state.includes("navigatorFilterQuery"));
assert.ok(state.includes("entryMatchesFilter"));
assert.ok(shell.includes("navigator-filter-bar"));
assert.ok(shell.includes("navigator-folder-filter"));
assert.ok(shell.includes("v-if=\"navigatorFilterOpen\"") || shell.includes("v-if='navigatorFilterOpen'"));
assert.ok(shell.includes("isNavigatorRoute"));
assert.ok(view.includes("entryMatchesFilter"));
assert.ok(view.includes("navigatorFilterQuery"));
assert.ok(view.includes("navigator.filterEmpty"));
assert.equal(en.navigator.filterToggle, "Filter folder");

// Filter bar must live under navigator chrome gate (not global for all routes).
const chromeStart = shell.indexOf('v-if="isNavigatorRoute"');
assert.ok(chromeStart >= 0);
const filterIdx = shell.indexOf("navigator-filter-bar");
assert.ok(filterIdx > chromeStart, "filter bar must be inside navigator route template");

console.log("navigator-filter checks OK (AIA-88).");
