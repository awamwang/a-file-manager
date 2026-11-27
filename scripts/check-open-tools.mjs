// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 7.2.x open-tool expansion baseline (ADR 0025): ten new presets (Sublime,
 * Notepad++, Windsurf, JetBrains SKU trio, 7-Zip, Chrome, Edge, VLC) with the
 * file-extension scope filter and the Settings-page search/collapse surface.
 * Pins FE↔BE name parity (ids + settings keys + serde field names) so a
 * rename on either side fails this gate instead of silently breaking the
 * Settings page or the context-menu launcher submenu.
 */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const settingsView = read("src/views/SettingsView.vue");
const defaultsRs = read("src-tauri/src/settings/defaults.rs");
const ideLaunchRs = read("src-tauri/src/ide_launch/mod.rs");
const contextMenuModel = read("src/components/navigator/contextMenuModel.ts");
const openToolsTs = read("src/state/openTools.ts");
const pkg = JSON.parse(read("package.json"));

// Preset id sets: 5 legacy editor presets + 10 expansion presets.
const LEGACY_IDS = ["vscode", "cursor", "trae", "qoder", "qwenwork"];
const NEW_IDS = [
  "sublime",
  "notepadpp",
  "windsurf",
  "idea",
  "pycharm",
  "webstorm",
  "sevenzip",
  "chrome",
  "edge",
  "vlc",
];
const ALL_IDS = [...LEGACY_IDS, ...NEW_IDS];

// 1. FE↔BE settings-key parity: every preset id has an enabled+path schema
//    key in defaults.rs, and SettingsView's id list matches the same set.
for (const id of ALL_IDS) {
  assert.ok(
    new RegExp(`"${id}"`).test(settingsView),
    `SettingsView OPEN_TOOL_IDS must contain "${id}"`,
  );
  for (const suffix of ["enabled", "path"]) {
    assert.ok(
      defaultsRs.includes(`"core.openTool.${id}.${suffix}"`),
      `defaults.rs must define core.openTool.${id}.${suffix}`,
    );
  }
}

// 2. Extension scope plumbing: meta struct + wire field (serde camelCase =>
// fileExtensions) on both built-in and custom tools, plus the FE client type.
assert.ok(ideLaunchRs.includes("pub file_extensions"), "OpenToolMeta must carry file_extensions");
assert.ok(
  (ideLaunchRs.match(/pub file_extensions/g) || []).length >= 3,
  "OpenToolMeta / CustomOpenTool / IdeAppInfo must all carry file_extensions",
);
for (const constant of ["SEVENZIP_FILE_EXTENSIONS", "BROWSER_FILE_EXTENSIONS", "VLC_FILE_EXTENSIONS"]) {
  assert.ok(ideLaunchRs.includes(constant), `mod.rs must define ${constant}`);
}
assert.ok(
  openToolsTs.includes("fileExtensions: string[]"),
  "openTools.ts wire types must expose fileExtensions: string[]",
);

// 3. Context-menu launcher submenu consumes the scope filter (pure function,
//    unit-checked in scripts/context-menu-model-checks.ts).
assert.ok(
  contextMenuModel.includes("export function openToolMatchesSelection"),
  "contextMenuModel must export openToolMatchesSelection",
);
assert.ok(
  contextMenuModel.includes("openToolMatchesSelection(app.fileExtensions, entry)"),
  "launcher rows must be filtered by openToolMatchesSelection(app.fileExtensions, entry)",
);

// 4. Settings surface: one unified drag list (built-in + custom in the
//    persisted menu order) with per-row enable checkbox + Configure dialog,
//    search over the rows, unavailable built-ins collapse past the threshold,
//    custom tools get an extensions input normalized through the shared pure
//    helper. Rediscover and add-custom sit in the toolbar above the list.
assert.ok(settingsView.includes('v-model="openToolSearch"'), "search input must bind openToolSearch");
assert.ok(settingsView.includes("const filteredOpenToolRows"), "rows must filter by search");
assert.ok(settingsView.includes("const visibleOpenToolRows"), "the unified list must render visibleOpenToolRows");
assert.ok(settingsView.includes("@click=\"refreshIdeApps\""), "rediscover button must call refreshIdeApps");
assert.ok(settingsView.includes("@click=\"addCustomTool\""), "add-custom button must call addCustomTool");
assert.ok(settingsView.includes("async function onRowEnabledChange("), "row checkbox must route through onRowEnabledChange");
assert.ok(settingsView.includes("const configuringToolId"), "config dialog state must exist");
assert.ok(settingsView.includes("function openToolConfig("), "row Configure button must open openToolConfig");
assert.ok(
  settingsView.includes("UNAVAILABLE_COLLAPSE_THRESHOLD = 4"),
  "unavailable built-ins must collapse past 4 rows",
);
assert.ok(
  settingsView.includes('from "../state/openToolExtensions"'),
  "SettingsView must reuse the shared normalize helper",
);
assert.ok(
  settingsView.includes("normalizeOpenToolExtensions(raw)"),
  "custom-tool extensions input must normalize before persisting",
);

// 5. The new pure helper is behavior-checked headlessly (not just present).
const helperCheck = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/open-tools-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (helperCheck.stdout) process.stdout.write(helperCheck.stdout);
if (helperCheck.stderr) process.stderr.write(helperCheck.stderr);
assert.equal(helperCheck.status, 0, "open-tools-checks failed");

// 6. i18n parity for the Settings strings across every locale
//    (npm run test:i18n enforces the full key set; this pins the values exist
//    and keeps {count}/{name} in the expander and dialog labels).
const localeDir = join(root, "src/i18n/locales");
const { readdirSync } = await import("node:fs");
for (const file of readdirSync(localeDir).filter((name) => name.endsWith(".json"))) {
  const catalog = JSON.parse(readFileSync(join(localeDir, file), "utf8"));
  const openTools = catalog?.settings?.openTools ?? {};
  for (const key of [
    "searchPlaceholder",
    "showUnavailable",
    "hideUnavailable",
    "fileExtensionsPlaceholder",
    "configure",
    "configTitle",
    "configClose",
  ]) {
    assert.ok(
      typeof openTools[key] === "string" && openTools[key].length > 0,
      `${file}: settings.openTools.${key} must be a non-empty string`,
    );
  }
  assert.ok(
    openTools.showUnavailable.includes("{count}"),
    `${file}: settings.openTools.showUnavailable must keep the {count} placeholder`,
  );
  assert.ok(
    openTools.configTitle.includes("{name}"),
    `${file}: settings.openTools.configTitle must keep the {name} placeholder`,
  );
}

// 7. This gate stays wired into the aggregated chain (an orphan gate never runs).
assert.equal(
  pkg.scripts["test:open-tools"],
  "node scripts/check-open-tools.mjs",
  "package.json must register test:open-tools",
);
assert.ok(
  pkg.scripts.test.includes("test:open-tools"),
  "npm test chain must include test:open-tools",
);

console.log("Open tools expansion gate OK (10 new presets, extension scope, unified drag list + config dialog).");
