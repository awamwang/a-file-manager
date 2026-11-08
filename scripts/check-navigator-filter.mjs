// SPDX-License-Identifier: GPL-3.0-or-later
// Navigator toolbar search: recursive name match in the open folder and its children.
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
assert.ok(state.includes("scheduleFolderSearch"));
assert.ok(state.includes("navigator_search"));
assert.ok(state.includes("focusNavigatorFilter"));
assert.ok(state.includes("clearNavigatorFilter"));
assert.ok(shell.includes("navigator-filter-bar"));
assert.ok(shell.includes("navigator-filter-inline"));
assert.ok(shell.includes("navigator-folder-filter"));
assert.ok(shell.includes("v-model=\"navigatorFilterQuery\"") || shell.includes("v-model='navigatorFilterQuery'"));
assert.ok(shell.includes("isNavigatorRoute"));
assert.ok(!shell.includes("navigator-filter-toggle"), "icon-only search toggle must be removed");
assert.ok(view.includes("scheduleFolderSearch"));
assert.ok(view.includes("navigatorFilterQuery"));
assert.ok(view.includes("navigator.filterEmpty"));
assert.ok(view.includes("row-location"));
assert.equal(en.navigator.filterToggle, "Search folder");
assert.ok(
  en.navigator.filterPlaceholder.includes("type:") &&
    en.navigator.filterPlaceholder.includes("size:") &&
    en.navigator.filterPlaceholder.includes("date:"),
  "AIA-234: placeholder must advertise type/size/date filters",
);

const searchRs = readFileSync(join(root, "src-tauri/src/navigator/search.rs"), "utf8");
assert.ok(searchRs.includes("parse_search_query"), "AIA-234: query parser");
assert.ok(searchRs.includes("size_bounds") || searchRs.includes("SizeBound"), "AIA-234: size filter");
assert.ok(searchRs.includes("date_bounds") || searchRs.includes("DateBound"), "AIA-234: date filter");
assert.ok(searchRs.includes("extensions"), "AIA-234: type/ext filter");
assert.ok(searchRs.includes("type_filter_matches_extension"), "AIA-234: type unit test");
assert.ok(searchRs.includes("size_filter_gt"), "AIA-234: size unit test");

// Filter input must live under navigator chrome gate (not global for all routes).
const chromeStart = Math.max(
  shell.indexOf('v-show="isNavigatorRoute"'),
  shell.indexOf('v-if="isNavigatorRoute"'),
);
assert.ok(chromeStart >= 0, "navigator chrome must be gated");
const filterIdx = shell.indexOf("navigator-filter-bar");
assert.ok(filterIdx > chromeStart, "filter bar must be inside navigator route template");

// AIA-147: search input sits after path/address controls inside navigator-chrome.
const chromeBlock = shell.slice(
  shell.indexOf('class="navigator-chrome"'),
  shell.indexOf('class="content"'),
);
assert.ok(chromeBlock.includes("navigator-filter-inline"));
assert.ok(chromeBlock.includes("breadcrumbs") || chromeBlock.includes("address-input"));
const pathMarker = Math.max(
  chromeBlock.indexOf("navigator-crumb-menu"),
  chromeBlock.indexOf("address-input"),
);
const searchMarker = chromeBlock.indexOf("navigator-folder-filter");
assert.ok(pathMarker >= 0 && searchMarker > pathMarker, "search input must follow path bar");

console.log("navigator-filter checks OK (recursive folder search).");
