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
  "projects.openWorkspaceInSystemExplorerHint",
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
  // Shortcut labels (primary modifier ⌘ / Option ⌥).
  "chrome.commandsShortcut",
  "chrome.commandsTitle",
  "searchPalette.hint",
  "detailsPane.multiHint",
  // "PC" wording in search copy.
  "discovery.meta",
  "discovery.everythingConfigHint",
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
