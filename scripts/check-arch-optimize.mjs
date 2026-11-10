// SPDX-License-Identifier: GPL-3.0-or-later
/** Arch extract invariants: side nav module + shared path-cards CSS. */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const sideNav = read("src/shell/sideNav.ts");
assert.ok(sideNav.includes("SIDE_NAV_ITEMS"));
assert.ok(sideNav.includes("isSideNavActive"));
assert.ok(sideNav.includes("sideNavIcon"));

const shell = read("src/components/shell/AppShell.vue");
assert.ok(shell.includes('from "../../shell/sideNav"'));
assert.ok(shell.includes("SIDE_NAV_ITEMS"));
assert.ok(!shell.includes("const navItems: NavItem[]"));

const main = read("src/main.ts");
assert.ok(main.includes("./styles/path-cards.css"));

const pathCards = read("src/styles/path-cards.css");
assert.ok(pathCards.includes(".path-card-grid"));
assert.ok(pathCards.includes(".path-card--with-action"));

const model = read("src/state/openStateModel.ts");
const pathKey = read("src/state/pathKey.ts");
assert.match(model, /export \{ toNativeFsPath, normalizePathKey \} from "\.\/pathKey"/);
assert.match(pathKey, /export function toNativeFsPath/);
assert.match(pathKey, /export function normalizePathKey/);
assert.match(pathKey, /replace\(\/\\\/\/g,\s*"\\\\"\)/);

const wg = read("src/views/WorkGroupsView.vue");
assert.ok(wg.includes("displayNameFromPath"));
assert.ok(!wg.includes(".path-card-grid"));

const projects = read("src/views/ProjectsView.vue");
assert.ok(!projects.includes(".path-card-grid {"));

console.log("arch-optimize: ok");
