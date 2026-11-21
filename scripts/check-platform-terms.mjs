// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Static contract for platform-specific copy (macOS port, 7.22).
 *
 * Windows-first wording stays in the base catalog; `platform.macos` holds a flat
 * dotted-key -> macOS copy map, merged by `src/i18n/platformTerms.ts` before
 * vue-i18n sees the messages. This script pins the key set, the wording, the
 * merge wiring and the "no new platform branches in components" architecture
 * gate (6.5 rule 6).
 */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const localeIds = ["en", "zh-CN", "ja", "de", "fr"];
const catalogs = Object.fromEntries(
  localeIds.map((id) => [
    id,
    JSON.parse(readFileSync(join(root, `src/i18n/locales/${id}.json`), "utf8")),
  ]),
);

/** Explicit allow-list: adding a macOS override requires updating this list. */
const EXPECTED_KEYS = [
  // System file manager (AFM's own Navigator / Explorer views are NOT here).
  "contextMenu.openWithExplorer",
  "contextMenu.reveal",
  "projects.noticeOpenedInSystemExplorer",
  "providers.openInExplorer",
  "settings.fields.core.fileDoubleClickAction.options.reveal",
  "taskCenter.revealHint",
  // Recycle Bin -> Trash.
  "contextMenu.deleteConfirm",
  "contextMenu.deleteNoRecycle",
  "contextMenu.deleted",
  "recycleBin.section",
  "recycleBin.tabName",
  "shortcuts.items.navigatorDelete",
  // This PC -> This Mac.
  "guide.title",
  "guide.tabName",
  "guide.rootLabel",
  "guide.rootTitle",
  // Windows-only default path example.
  "ai.skills.libraryRootHint",
  // Preview bridge: macOS uses the system Quick Look, never PowerToys Peek.
  "settings.fields.preview.externalProvider.description",
  "settings.fields.preview.externalProvider.options.auto",
  "settings.fields.preview.externalProvider.options.quicklook",
  "shortcuts.items.navigatorPreviewBridge",
  "health.deps.meta",
  "health.deps.items.quicklook.title",
  "health.deps.items.quicklook.detail",
  // Recycle Bin wording outside the dedicated recycleBin namespace.
  "virtualInfo.kindRecycle",
  "virtualInfo.recycleName",
  "preview.linkKindShortcut",
  // Drive letters -> mounted volumes.
  "guide.drive",
  "guide.driveRemovable",
  "guide.driveRemote",
  "guide.driveCdrom",
  "guide.driveRamdisk",
  "guide.emptyDrives",
  "guide.sections.drives",
  // Windows path examples / Windows-only integrations in settings copy.
  "discovery.blacklistPlaceholder",
  "extensions.installPathPlaceholder",
  "settings.fields.core.launchAtStartup.description",
  "settings.fields.core.navigatorViewMode.description",
  "settings.fields.core.openTool.vscode.path.description",
  "settings.fields.core.openTool.cursor.path.description",
  "settings.fields.core.openTool.trae.path.description",
  "settings.fields.core.openTool.qoder.path.description",
  "providers.searchBlocked",
  "settings.favoritesMeta",
  // Shortcut labels (primary modifier ⌘ / Option ⌥).
  "chrome.commandsShortcut",
  "chrome.commandsTitle",
  "searchPalette.hint",
  "detailsPane.multiHint",
  // "PC" wording in search copy.
  "discovery.meta",
  "discovery.everythingConfigHint",
  // Post-7.22 features that shipped with Windows-only copy in the base catalog:
  // search palette shortcuts / Everything, extension paths, voice permission
  // guidance, Finder/open-in text, release asset name, skill deploy mode.
  "discovery.testMeta",
  "settings.search.paletteMeta",
  "settings.search.knowledgeMeta",
  "settings.search.fuzzyHint",
  "settings.extensions.dataRootPlaceholder",
  "settings.openTools.customMeta",
  "voice.error.permission",
  "voice.selftest.permissionHint",
  "voice.error.noAudio",
  "knowledgeView.openInExplorer",
  "about.link.releases.desc",
  "ai.skills.deployModeJunction",
  // QuickLook health status-specific copy (depText() prefers `...<status>.detail`
  // over the generic key, so each status leaf needs its own override).
  "health.deps.items.quicklook.ok.detail",
  "health.deps.items.quicklook.ok.fix",
  "health.deps.items.quicklook.missing.detail",
  "health.deps.items.quicklook.missing.fix",
  "health.deps.items.quicklook.warn.detail",
  "health.deps.items.quicklook.warn.fix",
  "health.deps.items.quicklook.unbridged.detail",
  "health.deps.items.quicklook.unbridged.fix",
];
const expectedSorted = [...EXPECTED_KEYS].sort();

function lookup(catalog, dotted) {
  return dotted.split(".").reduce((node, part) => {
    return node && typeof node === "object" ? node[part] : undefined;
  }, catalog);
}

/** Comments describe the architecture; only code lines are binding. */
function codeOnly(source) {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .map((line) => line.replace(/\/\/.*$/, ""))
    .join("\n");
}

/** Windows wording that must not survive in macOS copy. de/fr Trash words
 * (Papierkorb / Corbeille) are platform-neutral and deliberately stay. */
const FORBIDDEN = [
  /Ctrl\+/,
  /Alt\+/,
  /%APPDATA%/,
  /C:\\Windows/,
  /Recycle Bin/,
  /File Explorer/,
  /This PC/,
  /this PC/,
  /your PC/,
  /\bExplorer\b/,
  /WebView2/,
  /资源管理器/,
  /回收站/,
  /此电脑/,
  /エクスプローラー/,
  /ごみ箱/,
  /このPC/,
  /Dieser PC/,
  /Ce PC/,
  /ce PC/,
];

let effectiveOverrides = 0;
for (const id of localeIds) {
  const catalog = catalogs[id];
  assert.ok(catalog.platform && typeof catalog.platform === "object", `${id} missing platform section`);
  assert.deepEqual(
    Object.keys(catalog.platform),
    ["macos"],
    `${id}: only a macOS variant is allowed (Windows uses the base copy)`,
  );
  const macos = catalog.platform.macos;
  assert.ok(macos && typeof macos === "object", `${id} missing platform.macos`);
  assert.deepEqual(
    Object.keys(macos).sort(),
    expectedSorted,
    `${id}: macOS override key set drifted from the contract`,
  );
  for (const [key, value] of Object.entries(macos)) {
    assert.equal(typeof value, "string", `${id}:${key} override must be a string`);
    assert.ok(value.trim().length > 0, `${id}:${key} override must not be empty`);
    assert.equal(
      typeof lookup(catalog, key),
      "string",
      `${id}:${key} has no base message to override`,
    );
    for (const pattern of FORBIDDEN) {
      assert.ok(
        !pattern.test(value),
        `${id}:${key} macOS copy still contains Windows wording ${pattern}`,
      );
    }
    if (value !== lookup(catalog, key)) effectiveOverrides += 1;
  }
  for (const key of ["chrome.commandsShortcut", "chrome.commandsTitle", "searchPalette.hint"]) {
    assert.match(macos[key], /⌘\+⇧\+P/, `${id}:${key} must use the ⌘ chord`);
  }
  assert.match(
    macos["detailsPane.multiHint"],
    /⌥\+(Enter|Entrée)/,
    `${id}: properties hint must use the ⌥ chord`,
  );
  assert.match(
    macos["ai.skills.libraryRootHint"],
    /~\/Library\/Application Support\/com\.a-file-manager\.app\/\.agents\/skills/,
    `${id}: skills default path must be the macOS config dir`,
  );
  const previewOptions = macos["settings.fields.preview.externalProvider.options.auto"];
  assert.ok(
    !/Peek|Peek/i.test(previewOptions),
    `${id}: macOS auto preview label must not advertise Peek`,
  );
  assert.match(
    macos["settings.fields.preview.externalProvider.options.quicklook"],
    /Quick Look|クイックルック|快速查看/i,
    `${id}: macOS quicklook option must name the system preview`,
  );
  assert.match(
    macos["guide.sections.drives"],
    /volume|ボリューム|卷/i,
    `${id}: drive-letter section must become volumes on macOS`,
  );
  assert.ok(
    !/C:\\/.test(macos["discovery.blacklistPlaceholder"]),
    `${id}: blacklist example must use a macOS path`,
  );
  // Post-adaptation features: macOS copy must name the system surfaces and the
  // ⌘/⌥ chords, never Windows Settings / Everything / .exe / a Windows path.
  for (const key of [
    "discovery.testMeta",
    "settings.search.paletteMeta",
  ]) {
    assert.match(macos[key], /⌘\+⇧\+P/, `${id}:${key} must use the ⌘ chord`);
  }
  assert.match(macos["settings.search.knowledgeMeta"], /⌥\+7/, `${id}: knowledge source uses ⌥+7`);
  for (const key of [
    "settings.search.fuzzyHint",
    "discovery.testMeta",
  ]) {
    assert.doesNotMatch(macos[key], /Everything/i, `${id}:${key} must not advertise Everything`);
  }
  for (const key of ["voice.error.permission", "voice.selftest.permissionHint"]) {
    assert.match(
      macos[key],
      /System Settings|系统设置|システム設定|Systemeinstellungen|Réglages système/,
      `${id}:${key} must point to macOS System Settings`,
    );
  }
  assert.match(macos["knowledgeView.openInExplorer"], /Finder|访达|ファインダー/i,
    `${id}: knowledge reveal must say Finder`);
  assert.match(macos["about.link.releases.desc"], /\.dmg|macOS/i,
    `${id}: releases link must name the macOS asset`);
  assert.ok(
    !/C:\\|%APPDATA%|\.exe/.test(macos["settings.extensions.dataRootPlaceholder"] + macos["settings.openTools.customMeta"]),
    `${id}: extension / opener copy must not use Windows paths or .exe`,
  );
  assert.match(macos["ai.skills.deployModeJunction"], /symboli|符号|シンボリック/i,
    `${id}: skill deploy mode is a symlink on macOS`);
  for (const status of ["ok", "missing", "warn", "unbridged"]) {
    assert.ok(
      !/named pipe|PowerToys| SID |poizan42|名前付きパイプ|命名管道/i.test(
        macos[`health.deps.items.quicklook.${status}.detail`] +
        macos[`health.deps.items.quicklook.${status}.fix`],
      ),
      `${id}: quicklook ${status} copy must not leak Windows pipe/Peek concepts`,
    );
  }
}
assert.ok(
  effectiveOverrides >= 100,
  `overrides must actually change copy (got ${effectiveOverrides})`,
);

// F5 is intentionally NOT overridden, and the chord is no longer baked into
// the title: the action tip renders the configured chord at runtime.
assert.ok(
  !EXPECTED_KEYS.includes("chrome.refreshTitle"),
  "chrome.refreshTitle must stay on the base copy while the chord is F5",
);
assert.match(
  readFileSync(join(root, "src/shortcuts/registry.ts"), "utf8"),
  /key:\s*"f5"/,
  "refresh chord must still be F5 (otherwise update the tip too)",
);
for (const id of localeIds) {
  assert.doesNotMatch(
    catalogs[id].chrome.refreshTitle,
    /F5/,
    `${id}: refresh title must not bake in the chord`,
  );
}
assert.match(
  readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8"),
  /v-action-tip="\{ label: t\('chrome\.refresh'\), actionId: 'navigator\.refresh' \}"/,
  "refresh button must surface its chord through the action tip",
);

// Architecture gate: merge lives in the i18n layer, host detection in the
// capability contract, no new platform branches in components.
const indexTs = codeOnly(readFileSync(join(root, "src/i18n/index.ts"), "utf8"));
const termsTs = codeOnly(readFileSync(join(root, "src/i18n/platformTerms.ts"), "utf8"));
const mainTs = codeOnly(readFileSync(join(root, "src/main.ts"), "utf8"));
const platformTs = codeOnly(readFileSync(join(root, "src/config/platform.ts"), "utf8"));
assert.match(indexTs, /localizeCatalog\(/, "i18n must merge localized catalogs");
assert.match(indexTs, /export function applyPlatformTerms/, "i18n must expose re-merge");
assert.match(
  mainTs,
  /hydratePlatform\(\)\.then\(applyPlatformTerms\)/,
  "bootstrap must re-merge copy after the capability IPC resolves",
);
assert.doesNotMatch(termsTs, /@tauri-apps\/api/, "merge helpers must stay pure (Node-testable)");
assert.doesNotMatch(termsTs, /isMacPlatform/, "host detection belongs to config/platform.ts");
assert.match(
  platformTs,
  /export function platformShortcutLabel/,
  "shortcut labels must reuse the existing capability-layer formatter",
);

const runtime = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/platform-terms-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (runtime.stdout) process.stdout.write(runtime.stdout);
if (runtime.stderr) process.stderr.write(runtime.stderr);
if (runtime.status !== 0) process.exit(runtime.status ?? 1);

console.log(
  `platform copy overrides OK (${EXPECTED_KEYS.length} keys × ${localeIds.length} locales, ${effectiveOverrides} non-identical).`,
);
