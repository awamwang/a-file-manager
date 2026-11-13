// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const rust = readFileSync(join(root, "src-tauri/src/favorites/shell_verb.rs"), "utf8");
const fav = readFileSync(join(root, "src-tauri/src/favorites/mod.rs"), "utf8");
const lib = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const settingsView = readFileSync(join(root, "src/views/SettingsView.vue"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(rust.includes("AFM_ADD_FAVORITE_FLAG"));
assert.ok(rust.includes("AFileManager.AddFavorite"));
assert.ok(fav.includes("favorites_os_hook_set_enabled"));
assert.ok(fav.includes("ensure_favorite"));
assert.ok(lib.includes("favorites_os_hook_set_enabled"));
assert.ok(lib.includes("apply_add_favorite_cli"));
// FIL-65：OS 动词开关移入 Settings 高级（power）分区。
assert.ok(settingsView.includes("setFavoritesOsHookEnabled"));
assert.ok(
  settingsView.includes("favorites.osVerb") || settingsView.includes("t(\"favorites.osVerb\")"),
);
assert.equal(en.favorites.osVerb.includes("AFM"), true);

console.log("Favorites shell verb checks OK (AIA-67).");
