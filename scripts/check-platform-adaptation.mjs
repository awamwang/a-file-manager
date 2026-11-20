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
const windowsRsSource = read("src-tauri/src/platform/windows.rs");
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

// 6. Post-unification features (ADR 0018 voice / ADR 0022 managed sidecars).
// New platform differences must route through the same contract, never through
// a naked #[cfg] or a hardcoded Windows env var in a business module.
for (const field of ["microphone_permission_precheck", "sidecar_native_artifacts"]) {
  assert.match(platformRs, new RegExp(`pub ${field}: bool`), `Rust caps missing ${field}`);
}
for (const field of ["microphonePermissionPrecheck", "sidecarNativeArtifacts"]) {
  assert.match(platformTs, new RegExp(`${field}: boolean`), `TS caps missing ${field}`);
  assert.match(platformTs, new RegExp(`${field}: false`), `neutral caps missing ${field}`);
}
// Windows owns the synchronous mic consent store. Native feed artifacts ship
// on both Windows and macOS; platform-scoped lookup keeps each host on its own
// binary (a Mac must never pick a Windows .exe).
assert.match(windowsRsSource, /caps\.microphone_permission_precheck = true/, "Windows owns consent precheck");
assert.match(windowsRsSource, /caps\.sidecar_native_artifacts = true/, "Windows ships native sidecars");
assert.match(macosRs, /caps\.sidecar_native_artifacts = true/, "macOS ships native sidecars");
assert.ok(
  !/caps\.microphone_permission_precheck = true/.test(macosRs),
  "macOS must not claim a synchronous mic precheck",
);
assert.match(
  read("src-tauri/src/voice/mod.rs"),
  /capabilities\(\)\.microphone_permission_precheck/,
  "voice precheck must be capability-gated",
);
// Microphone TCC prompt needs a usage description in the merged Info.plist.
const infoPlist = read("src-tauri/Info.plist");
assert.match(infoPlist, /NSMicrophoneUsageDescription/, "Info.plist must declare mic usage");
assert.ok(
  !/com\.apple\.security\.app-sandbox/.test(infoPlist),
  "do not enable App Sandbox (would need audio-input entitlement + break file access)",
);
// Sidecar exec-bit + interpreter resolution live in the adapter, not callers.
assert.match(platformRs, /fn make_executable\(&self/, "platform declares exec-bit contract");
assert.match(platformRs, /fn node_command\(&self/, "platform declares Node interpreter contract");
assert.match(macosRs, /fn make_executable\(&self/, "macOS implements chmod");
assert.match(macosRs, /fn node_command\(&self/, "macOS probes Homebrew node");
assert.match(
  read("src-tauri/src/managed_dependency/store.rs"),
  /platform::make_executable\(&tmp, None\)/,
  "downloaded artifacts must get an exec bit via platform",
);
assert.match(
  read("src-tauri/src/extension_host/pack_fs.rs"),
  /platform::make_executable\(&out_path, unix_mode\)/,
  "pack zip extraction must preserve Unix mode via platform",
);
assert.match(
  read("src-tauri/src/tool_runtime/sidecar.rs"),
  /platform::node_command\(\)/,
  "shared Node runtime must resolve interpreter via platform",
);
assert.ok(
  !/std::env::var\("USERPROFILE"\)/.test(read("src-tauri/src/tool_runtime/gateway.rs")),
  "gateway must not hardcode USERPROFILE (use platform::home_dir)",
);
// A host without a published artifact for its platform must not be flipped to
// Managed mode. Safety now comes from platform-scoped feed lookup, not from
// disabling the capability on macOS.
assert.match(
  read("src-tauri/src/extension_host/host.rs"),
  /capabilities\(\)\.sidecar_native_artifacts/,
  "managed-mode decision must be gated by the host artifact capability",
);

// 7. ADR 0022 macOS packaging: per-platform feed entries.
const storeRs = read("src-tauri/src/managed_dependency/store.rs");
const feedRs = read("src-tauri/src/managed_dependency/feed.rs");
const commandsRs = read("src-tauri/src/managed_dependency/commands.rs");
const buildFeedPy = read("scripts/build_pack_feed.py");
assert.match(platformRs, /pub fn artifact_platform_key\(\)/, "platform derives the feed artifact key");
assert.match(storeRs, /pub platform: Option<String>/, "feed entry carries an optional platform tag");
assert.match(storeRs, /pub fn find_entry_for_platform/, "feed lookup is platform-aware");
assert.match(feedRs, /resolve_managed_binding_for_platform/, "binding resolver is platform-scoped");
assert.ok(
  !/store::find_entry\(m, &record\.pack_id\)/.test(commandsRs),
  "provisioning paths must not blind-pick a feed entry by packId",
);
assert.match(commandsRs, /find_host_entry/, "provisioning paths use the host-scoped lookup");
assert.match(buildFeedPy, /def artifact_platform_key/, "build script derives Rust-compatible platform keys");
assert.match(buildFeedPy, /"platform": platform_key/, "build script writes platform into feed entries");
assert.match(buildFeedPy, /e\.get\("packId"\), e\.get\("platform"\)/, "feed merge dedups by (packId, platform)");

console.log("Unified platform adaptation OK (schema + preview + health + menu + copy + post-port features + per-platform feed).");
