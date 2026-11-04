// SPDX-License-Identifier: GPL-3.0-or-later
// Run via: node --experimental-strip-types scripts/fluent-icon-checks.ts

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  FLUENT_GLYPHS,
  NAV_ICON_BY_ID,
  SETTINGS_ICON_BY_KEY,
  settingsIconForKey,
  type FluentGlyphId,
} from "../src/components/icons/glyphs.ts";

const ids = Object.keys(FLUENT_GLYPHS) as FluentGlyphId[];
assert.ok(ids.length >= 20, "expected a useful glyph catalog");

for (const id of ids) {
  const glyph = FLUENT_GLYPHS[id];
  assert.equal(typeof glyph.codepoint, "string");
  assert.ok(glyph.codepoint.length >= 1, `${id} missing codepoint`);
  assert.ok(Array.isArray(glyph.paths) && glyph.paths.length >= 1, `${id} missing svg paths`);
}

for (const nav of [
  "navigator",
  "discovery",
  "providers",
  "work-groups",
  "projects",
  "extensions",
  "settings",
  "health",
]) {
  assert.ok(NAV_ICON_BY_ID[nav], `missing nav icon ${nav}`);
  assert.ok(FLUENT_GLYPHS[NAV_ICON_BY_ID[nav]], `nav icon ${nav} not in catalog`);
}

for (const key of Object.keys(SETTINGS_ICON_BY_KEY)) {
  assert.ok(FLUENT_GLYPHS[SETTINGS_ICON_BY_KEY[key]], `settings icon ${key} missing glyph`);
}

assert.equal(settingsIconForKey("core.language"), "globe");
assert.equal(settingsIconForKey("unknown.setting"), "settings");

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const iconVue = readFileSync(join(root, "src/components/icons/FluentIcon.vue"), "utf8");
assert.ok(iconVue.includes("Segoe Fluent Icons"));
assert.ok(iconVue.includes("forceSvg"));
assert.ok(iconVue.includes("fluent-icon__svg"));

const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
assert.ok(shell.includes("FluentIcon"));
assert.ok(shell.includes("sidenav-item--active::before") || shell.includes("stroke-accent-soft"));
assert.ok(shell.includes('name="back"'));
assert.ok(shell.includes('name="minimize"'));
assert.ok(!/win-btn[\s\S]*?[─□✕❐]/.test(shell.replace(/<!--[\s\S]*?-->/g, "")), "caption still uses ASCII glyphs");

const settings = readFileSync(join(root, "src/views/SettingsView.vue"), "utf8");
assert.ok(settings.includes("settings-layout"));
assert.ok(settings.includes("settingsIconForKey"));
assert.ok(settings.includes("fluent-icon-wash"));
assert.ok(settings.includes("tier-nav"));

console.log(`fluent-icon-checks: ok (${ids.length} glyphs)`);
