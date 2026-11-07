// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-66 + AIA-79: favorites live in Navigator places dropdown; OS verb in Settings.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const settings = readFileSync(join(root, "src/views/SettingsView.vue"), "utf8");

assert.ok(shell.includes("places-menu"));
assert.ok(shell.includes("openPlacesPath") || shell.includes("favoritesDoc"));
assert.ok(shell.includes("favoritesDoc"));
assert.ok(shell.includes("loadFavorites"));
assert.ok(!shell.includes("fav-os-hook"), "OS verb must leave the shell sidenav");
assert.ok(settings.includes("setFavoritesOsHookEnabled"), "OS verb belongs in Settings");
assert.ok(!view.includes("favorites-chip"), "chips moved out of list chrome");

// AIA-230: favorites / places entries are navigation entry points — no path-matched selected.
const favHeading = shell.indexOf('places-heading--fav">{{ t("favorites.label") }}');
assert.ok(favHeading >= 0, "favorites heading in places menu");
const favBlockEnd = shell.indexOf('places-heading--fav">{{ t("recycleBin.section") }}', favHeading);
assert.ok(favBlockEnd > favHeading, "recycle section follows favorites");
const favBlock = shell.slice(favHeading, favBlockEnd);
assert.ok(
  !favBlock.includes("places-item--active"),
  "AIA-230: favorites must not use sticky places-item--active",
);
assert.ok(
  !/places-item[\s\S]*?:class=\{[^}]*currentPath/.test(favBlock),
  "AIA-230: favorites must not bind selection to currentPath",
);

console.log("Favorites places/settings checks OK (AIA-66/79/230).");
