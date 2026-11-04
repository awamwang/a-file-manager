// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");

assert.ok(shell.includes("fav-pins"));
assert.ok(shell.includes("openFavoritePin"));
assert.ok(shell.includes("favoritesDoc"));
assert.ok(shell.includes("loadFavorites"));
assert.ok(!view.includes("favorites-chip"), "chips moved to sidenav");

console.log("Favorites sidebar checks OK (AIA-66).");
