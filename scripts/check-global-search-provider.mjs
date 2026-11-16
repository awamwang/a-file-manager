// SPDX-License-Identifier: GPL-3.0-or-later
// 7.21: the host-wide search provider is an adapter concern. Keep mdfind,
// Everything IPC selection and engine labels out of the business modules.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (p) => readFileSync(join(root, p), "utf8");
/** Ignore doc/line comments so prose cannot satisfy or trip a code assertion. */
const codeOnly = (src) =>
  src
    .split("\n")
    .filter((line) => !line.trimStart().startsWith("//"))
    .join("\n");

const platformMod = read("src-tauri/src/platform/mod.rs");
const macos = read("src-tauri/src/platform/macos.rs");
const windows = read("src-tauri/src/platform/windows.rs");
const discoveryMod = read("src-tauri/src/discovery/mod.rs");
const platformBackend = read("src-tauri/src/discovery/platform_backend.rs");
const backendTrait = read("src-tauri/src/discovery/backend.rs");
const types = read("src-tauri/src/discovery/types.rs");
const service = read("src-tauri/src/discovery/service.rs");
const blacklist = read("src-tauri/src/discovery/blacklist.rs");
const view = read("src/views/DiscoveryView.vue");
const settingsUi = read("src/state/settingsUi.ts");
const platformTs = read("src/config/platform.ts");

// 1. Neutral contract lives in the platform layer.
for (const needle of [
  "pub enum GlobalSearchEngine",
  "pub struct SearchHit",
  "pub struct SearchQueryResult",
  "pub struct SearchProbe",
  "fn global_search_engine(&self) -> GlobalSearchEngine",
  "fn probe_global_search(&self) -> SearchProbe",
  "pub fn global_search(",
  "pub fn probe_global_search()",
]) {
  assert.ok(platformMod.includes(needle), `platform/mod.rs missing ${needle}`);
}
assert.ok(
  platformMod.includes("pub global_search_engine: String"),
  "capabilities must expose global_search_engine",
);

// 2. macOS owns Spotlight; Windows only declares the choice.
for (const needle of [
  'const MDFIND_PATH: &str = "/usr/bin/mdfind"',
  "fn spotlight_name_query(",
  "fn parse_mdfind_output(",
  "GlobalSearchEngine::Spotlight",
]) {
  assert.ok(macos.includes(needle), `macOS adapter missing ${needle}`);
}
assert.ok(
  macos.includes('caps.global_search_engine = "spotlight"'),
  "macOS capabilities must report spotlight",
);
assert.ok(
  macos.includes("GlobalSearchEngine::None"),
  "non-macOS Unix must declare no host index",
);
assert.ok(
  windows.includes("GlobalSearchEngine::EverythingIpc") &&
    windows.includes('caps.global_search_engine = "everything"'),
  "Windows must keep Everything IPC as its engine",
);
assert.ok(
  !windows.includes("mdfind"),
  "Windows adapter must not depend on Spotlight",
);
assert.ok(
  !macos.includes('caps.global_search_engine = "everything"'),
  "macOS must never claim the Everything backend",
);

// 3. discovery/ selects the backend through the adapter, not #[cfg].
assert.ok(
  discoveryMod.includes("crate::platform::global_search_engine()") &&
    discoveryMod.includes("PlatformSearchBackend"),
  "discovery/mod.rs must select its backend via the platform layer",
);
assert.ok(
  !discoveryMod.includes("#[cfg(windows)]") &&
    !discoveryMod.includes("#[cfg(target_os"),
  "discovery/mod.rs must not open a new platform branch (6.5 gate 6)",
);
assert.ok(
  platformBackend.includes("platform::global_search(") &&
    platformBackend.includes("DiscoveryEngine::Spotlight"),
  "PlatformSearchBackend must delegate to the adapter and label the engine",
);
const platformBackendCode = codeOnly(platformBackend);
assert.ok(
  !platformBackendCode.includes("mdfind") &&
    !platformBackendCode.includes("Command::new") &&
    !platformBackendCode.includes("windows::"),
  "PlatformSearchBackend must be a thin type mapper, not an OS integration",
);
assert.ok(
  backendTrait.includes("fn engine(&self) -> DiscoveryEngine"),
  "EverythingBackend must report its engine",
);
assert.ok(types.includes("Spotlight"), "DiscoveryEngine needs a Spotlight variant");
assert.ok(
  service.includes("DiscoveryEngine::Everything | DiscoveryEngine::Spotlight") &&
    service.includes("engine: self.everything.engine()"),
  "service must route Spotlight queries and label their results",
);
assert.ok(
  service.includes("self.everything.engine() == DiscoveryEngine::Spotlight"),
  "Spotlight must skip Everything's empty-query totitems probe",
);
assert.ok(
  blacklist.includes("crate::platform::to_native_fs_path") &&
    !blacklist.includes('replace(\'/\', "\\\\")'),
  "blacklist paths are real host paths and must not be Windows-ized",
);

// 4. Frontend: real paths go through fsPath.ts, UI labels follow capabilities.
assert.ok(
  view.includes('"spotlight"') &&
    view.includes("getCachedPlatformCapabilities") &&
    view.includes("toNativeFsPath"),
  "DiscoveryView must know the Spotlight engine and use fsPath.ts",
);
assert.ok(
  !view.includes('replace(/\\//g, "\\\\")') && !view.includes('replace(/\\//g'),
  "DiscoveryView must not rewrite / to \\ on its own",
);
assert.ok(
  view.includes("spotlightActive") && view.includes("primaryEngineLabel"),
  "engine cards must follow the host index",
);
assert.ok(
  settingsUi.includes('caps.globalSearchEngine !== "everything"'),
  "Windows-only 'Prefer Everything' setting must be hidden on macOS",
);
assert.ok(
  platformTs.includes("globalSearchEngine: \"everything\" | \"spotlight\" | \"none\"") &&
    platformTs.includes('globalSearchEngine: "none"'),
  "TS capabilities must mirror the Rust field and neutral default",
);

// 5. All five locales carry the new engine label and engine-agnostic copy.
for (const locale of ["en", "zh-CN", "ja", "de", "fr"]) {
  const catalog = JSON.parse(read(`src/i18n/locales/${locale}.json`));
  const d = catalog.discovery;
  assert.equal(d.engines.spotlight, "Spotlight", `${locale} engines.spotlight`);
  assert.equal(d.engines.everything, "Everything", `${locale} engines.everything`);
  for (const key of [
    "everythingConfigHint",
    "fallbackOff",
    "fallbackStandby",
    "fallbackActive",
    "fallbackConfigHint",
    "fallbackBlacklistNote",
  ]) {
    assert.ok(
      d[key].includes("{engine}"),
      `${locale} discovery.${key} must interpolate {engine}`,
    );
  }
  assert.ok(
    !d.syntaxHint.includes("Everything"),
    `${locale} syntaxHint must not hardcode Everything`,
  );
}

console.log("Global search provider checks OK (7.21: Spotlight in adapter, no OS leak).");
