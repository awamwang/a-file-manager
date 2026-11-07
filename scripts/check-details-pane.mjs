// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-235: Alt+Shift+P details pane is separate from Alt+Enter properties.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const pane = readFileSync(join(root, "src/components/navigator/DetailsPane.vue"), "utf8");
const registry = readFileSync(join(root, "src/shortcuts/registry.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(view.includes("DetailsPane"));
assert.ok(view.includes("detailsPaneOpen"));
assert.ok(view.includes("shortcutToggleDetailsPane"));
assert.ok(view.includes('registerShortcutHandler("navigator.detailsPane"'));
assert.ok(view.includes("PropertiesPanel"), "properties sidebar must remain");
assert.ok(view.includes("navigator.properties"), "Alt+Enter properties must remain");
assert.ok(pane.includes("navigator_stat"));
assert.ok(pane.includes("detailsPane.title"));
assert.ok(!pane.includes("applyAttributes"), "details pane must not edit attributes");
assert.ok(registry.includes('id: "navigator.detailsPane"'));
assert.ok(registry.includes("shift: true") && registry.includes('key: "p"'));
assert.equal(en.shortcuts.items.navigatorDetailsPane, "Toggle details pane");
assert.equal(en.detailsPane.title, "Details");

console.log("details-pane checks OK (AIA-235).");
