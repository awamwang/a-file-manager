// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Static contract for the unified platform adaptation path (macOS port).
 *
 * One host policy must drive: schema enums/defaults/row visibility, persisted
 * value sanitizing, preview backend selection, health items and UI copy.
 * This script fails when a future change routes around that path again.
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const platformRs = read("src-tauri/src/platform/mod.rs");
const macosRs = read("src-tauri/src/platform/macos.rs");
const policyRs = read("src-tauri/src/settings/platform_policy.rs");
const defaultsRs = read("src-tauri/src/settings/defaults.rs");
const sanitizeRs = read("src-tauri/src/settings/sanitize.rs");
const bridgeRs = read("src-tauri/src/preview_bridge/mod.rs");
const depsRs = read("src-tauri/src/deps_probe.rs");
const platformTs = read("src/config/platform.ts");
const hydrateTs = read("src/state/settingsHydrate.ts");
const settingsView = read("src/views/SettingsView.vue");
const menuModel = read("src/components/navigator/contextMenuModel.ts");
const menuView = read("src/components/navigator/NavigatorContextMenu.vue");
const healthView = read("src/views/HealthView.vue");

// 1. Capability contract: both ends must agree on the preview capability bits.
for (const field of ["quick_look_preview", "peek_preview"]) {
  assert.match(platformRs, new RegExp(`pub ${field}: bool`), `Rust caps missing ${field}`);
}
for (const field of ["quickLookPreview", "peekPreview"]) {
  assert.match(platformTs, new RegExp(`${field}: boolean`), `TS caps missing ${field}`);
  assert.match(platformTs, new RegExp(`${field}: false`), `neutral caps missing ${field}`);
}
assert.match(platformRs, /pub trait PreviewProvider/, "platform layer must own the preview provider");
assert.match(macosRs, /impl PreviewProvider for NativeQuickLook/, "macOS must register Quick Look");
assert.match(macosRs, /caps\.quick_look_preview = true/, "macOS capability must be honest");
assert.match(platformRs, /pub fn preview_provider\(\)/, "one provider entry point in platform/");

// 2. Settings policy is the single source of truth for enum/default/hidden.
assert.match(policyRs, /pub fn rule_for\(/, "platform policy must be pure and testable");
assert.match(policyRs, /pub fn allowed_preview_values\(/, "bridge must reuse the same value list");
assert.match(defaultsRs, /platform_policy::apply\(definition, &caps\)/, "schema must apply policy");
assert.match(defaultsRs, /pub\(crate\) fn raw_setting_definitions/, "raw declarations must stay testable");
assert.match(sanitizeRs, /definition_map\(\)/, "sanitizer must use the same policy-applied definitions");
assert.match(platformRs, /pub\(crate\) fn neutral\(/, "policy tests need synthetic capabilities");
assert.match(platformRs, /peek_preview: false/, "neutral/non-Windows host rejects Peek");
assert.ok(!/only supported on Windows/.test(policyRs), "policy must not throw on unsupported hosts");
assert.match(defaultsRs, /"peek"\.to_string\(\)/, "Windows declarations must still contain Peek");

// 3. Frontend renders the platform-aware schema, not a static enum list.
assert.match(hydrateTs, /hidden\?: boolean/, "schema type must carry row visibility");
assert.match(settingsView, /def\.hidden/, "Settings must hide rows marked by host policy");
assert.match(settingsView, /v-for="option in def\.enumValues/, "enum options must come from host schema");
assert.match(bridgeRs, /platform_policy::allowed_preview_values/, "preview bridge must reject invalid modes");
assert.match(bridgeRs, /crate::platform::preview_provider\(\)\.is_some\(\)/, "bridge must use host provider");

// 4. Health / context menu follow the same capabilities.
assert.match(depsRs, /crate::platform::current\(\)\.capabilities\(\)/, "health must be capability-driven");
assert.match(depsRs, /if caps\.peek_preview/, "Peek check must only exist where Peek exists");
assert.match(depsRs, /crate::platform::home_dir\(\)/, "bookmark probe must use platform home");
assert.match(healthView, /depText\(item/, "health copy must allow platform overrides");
assert.match(menuModel, /caps\?: PlatformCapabilities/, "menu model must accept capabilities");
assert.match(menuModel, /nativeOpenWithDialog !== false/, "native Open With must be gated");
assert.match(menuModel, /nativeContextMenu === false/, "Win32 shell cascade must be gated");
assert.match(menuView, /caps: getCachedPlatformCapabilities\(\)/, "menu must pass the shared contract");

// 5. Copy layer: every locale must carry the expanded macOS override set.
const localeIds = ["en", "zh-CN", "ja", "de", "fr"];
for (const id of localeIds) {
  const catalog = JSON.parse(read(`src/i18n/locales/${id}.json`));
  assert.ok(Object.keys(catalog.platform.macos).length >= 50, `${id}: macOS copy coverage regressed`);
  assert.ok(catalog.health.deps.items.webview?.title, `${id}: missing system WebView health title`);
  assert.ok(catalog.health.deps.items.quicklook?.detail, `${id}: missing Quick Look detail copy`);
}

console.log("Unified platform adaptation OK (schema + preview + health + menu + copy).");
