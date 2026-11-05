// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const providers = readFileSync(join(root, "src/views/ProvidersView.vue"), "utf8");

assert.ok(!/Rust schema|health_get|vNext host|awam extension layer/i.test(en.settings.meta));
assert.ok(!/Rust schema|health_get|vNext host|awam extension layer/i.test(en.providers.meta));
assert.ok(!/Rust schema|health_get|vNext host|awam extension layer/i.test(en.extensions.meta));
assert.ok(!/Rust schema|health_get|vNext host|awam extension layer/i.test(en.health.meta));
assert.ok(en.providers.emptyBackends.length > 8);
assert.ok(providers.includes("providers.emptyBackends"));
assert.ok(
  view.includes("min-width: min(12rem, 40%)") || view.includes("flex: 1 1 12rem"),
);

console.log("ui-optimize: ok");
