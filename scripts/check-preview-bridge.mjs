// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-168: Preview Bridge MVP — Space → selectedPaths → Peek/QL/builtin.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const bridgeRs = read("src-tauri/src/preview_bridge/mod.rs");
const libRs = read("src-tauri/src/lib.rs");
const defaults = read("src-tauri/src/settings/defaults.rs");
const migrate = read("src-tauri/src/settings/migrate.rs");
const types = read("src-tauri/src/settings/types.rs");
const registry = read("src/shortcuts/registry.ts");
const runtime = read("src/shortcuts/runtime.ts");
const chord = read("src/shortcuts/chord.ts");
const nav = read("src/views/NavigatorView.vue");
const fe = read("src/state/previewBridge.ts");
const fields = read("src/i18n/settingsFields.ts");

assert.ok(bridgeRs.includes("preview_bridge_invoke"));
assert.ok(bridgeRs.includes("PowerToys.Peek.UI.exe"));
assert.ok(bridgeRs.includes("QuickLook.App.Pipe."));
assert.ok(bridgeRs.includes("QuickLook.App.PipeMessages.Toggle|"));
assert.ok(bridgeRs.includes("fallback_builtin"));
assert.ok(
  bridgeRs.includes("not a claim") ||
    bridgeRs.includes("not zero-adapter") ||
    bridgeRs.includes("thin protocol"),
  "Rust module must not claim zero-adapter for all QL",
);
assert.ok(libRs.includes("preview_bridge::preview_bridge_invoke"));
assert.ok(defaults.includes('"preview.externalProvider"'));
assert.ok(defaults.includes('"auto"'));
assert.ok(defaults.includes('"peek"'));
assert.ok(defaults.includes('"quicklook"'));
assert.ok(migrate.includes("schema v7"));
assert.ok(types.includes("SETTINGS_SCHEMA_VERSION: u32 = 7"));

assert.ok(registry.includes('id: "navigator.previewBridge"'));
assert.ok(registry.includes('key: "space"'));
assert.ok(chord.includes('if (key === " ") return "space"'));
assert.ok(runtime.includes("isEditableTarget"));
assert.ok(nav.includes("navigator.previewBridge"));
assert.ok(nav.includes("invokePreviewBridge"));
assert.ok(nav.includes("previewBridgeForcePane"));
assert.ok(fe.includes("preview_bridge_invoke"));
assert.ok(fields.includes('"preview.externalProvider"'));

const en = JSON.parse(read("src/i18n/locales/en.json"));
assert.equal(
  en.settings.fields.preview.externalProvider.options.auto.includes("Peek"),
  true,
);
assert.ok(en.shortcuts.items.navigatorPreviewBridge);
assert.ok(en.previewBridge.fallbackBuiltin);

console.log("Preview Bridge checks OK (Peek CLI / QL pipe / builtin fallback).");
