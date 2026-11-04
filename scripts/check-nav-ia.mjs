// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-46: Discovery + Providers above Settings; Health not top-level sidenav.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const router = readFileSync(join(root, "src/router/index.ts"), "utf8");
const settings = readFileSync(join(root, "src/views/SettingsView.vue"), "utf8");

const navBlock = shell.match(/const navItems: NavItem\[] = \[([\s\S]*?)\];/);
assert.ok(navBlock, "navItems block missing");
const ids = [...navBlock[1].matchAll(/id:\s*"([^"]+)"/g)].map((m) => m[1]);
assert.deepEqual(
  ids,
  [
    "navigator",
    "work-groups",
    "projects",
    "extensions",
    "discovery",
    "providers",
    "settings",
  ],
  "expected IA order with Discovery/Providers above Settings and no top-level health",
);
assert.ok(!ids.includes("health"), "health must not be top-level sidenav");

assert.ok(router.includes('path: "/health", redirect: "/settings/health"'));
assert.ok(router.includes('path: "/settings/health"'));
assert.ok(settings.includes("sectionHealth") || settings.includes("settings.sectionHealth"));
assert.ok(settings.includes("HealthView"));

console.log("Nav IA checks OK (AIA-46 sidenav + Settings Health subsection).");
