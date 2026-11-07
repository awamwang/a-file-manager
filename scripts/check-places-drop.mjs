// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-250: Places / favorites sidebar HTML5 file drop (shared surface, like tree/tabs).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const surface = read("src/state/fileDropSurface.ts");
const shell = read("src/components/shell/AppShell.vue");
const favCheck = read("scripts/check-favorites-sidebar.mjs");

assert.ok(surface.includes("AIA-250") || surface.includes("places"), "surface covers places");
assert.ok(surface.includes("acceptFolderDragOver"), "shared dragover");
assert.ok(surface.includes("commitFolderDrop"), "shared drop commit");

assert.ok(shell.includes("onPlacesItemDragOver"), "places item dragover");
assert.ok(shell.includes("onPlacesItemDrop"), "places item drop");
assert.ok(shell.includes("onPlacesMenuDragOver"), "menu opens on file drag");
assert.ok(shell.includes("placesHoverOpen"), "dwell-open scheduler");
assert.ok(shell.includes("placesAcceptsFileDrop"), "reject non-folder destinations");
assert.ok(shell.includes("isGuidePath") && shell.includes("isRecyclePath"), "guide/recycle helpers");
assert.ok(shell.includes('data-drop-target="places"'), "places menu drop marker");
assert.ok(shell.includes('data-drop-target="places-item"'), "places item drop marker");
assert.ok(shell.includes("places-item--file-drop"), "places drop visual");
assert.ok(shell.includes("commitFolderDrop"), "places uses shared commit");
assert.ok(shell.includes("acceptFolderDragOver"), "places uses shared accept");
assert.ok(shell.includes("createDropHoverScheduler"), "reuses hover scheduler");

// Favorites block still must not sticky-select by currentPath (AIA-230).
assert.ok(favCheck.includes("AIA-230"), "favorites sidebar check retained");

console.log("Places/favorites drop surface checks OK (AIA-250).");
