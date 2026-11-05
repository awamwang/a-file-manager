// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-101: startup page setting + cold-launch route.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const defaults = readFileSync(join(root, "src-tauri/src/settings/defaults.rs"), "utf8");
const fields = readFileSync(join(root, "src/i18n/settingsFields.ts"), "utf8");
const app = readFileSync(join(root, "src/App.vue"), "utf8");
const startup = readFileSync(join(root, "src/state/startupPage.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(defaults.includes('"core.startupPage"'));
assert.ok(defaults.includes('"navigator"'));
assert.ok(defaults.includes('"projects"'));
assert.ok(defaults.includes('"discovery"'));
assert.ok(fields.includes('"core.startupPage"'));
assert.ok(fields.includes('"navigator"') && fields.includes('"projects"'));
assert.ok(startup.includes("startupRouteFor"));
assert.ok(app.includes("startupRouteFor"));
assert.ok(app.includes("STARTUP_PAGE_SETTING_KEY"));
assert.equal(en.settings.fields.core.startupPage.options.navigator, "Explorer");

console.log("startup page checks OK (AIA-101).");
