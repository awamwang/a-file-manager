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

console.log("Favorites places/settings checks OK (AIA-66/79).");
