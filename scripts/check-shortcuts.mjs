// SPDX-License-Identifier: GPL-3.0-or-later
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (relative) => readFileSync(join(root, relative), "utf8");
const settings = read("src/views/SettingsView.vue");
const shell = read("src/components/shell/AppShell.vue");
const router = read("src/router/index.ts");
const main = read("src/main.ts");
const tipHost = read("src/components/common/ActionTipHost.vue");
const contextMenu = read("src/components/navigator/NavigatorContextMenu.vue");

assert.ok(settings.includes("sectionShortcuts") || settings.includes("settings.sectionShortcuts"));
assert.ok(settings.includes("listShortcutsForDisplay"));
assert.ok(shell.includes("dispatchShortcutEvent"));
assert.ok(shell.includes("registerShortcutHandler"));
assert.ok(router.includes('path: "/settings/shortcuts"'));

// Configurable chords + one-to-many recorder in Settings.
assert.ok(settings.includes("listShortcutActionsForDisplay"));
assert.ok(settings.includes("setActionChords"));
assert.ok(settings.includes("chordFromEvent"));
assert.ok(settings.includes("validateNewChord"));
assert.ok(settings.includes("recordingBindingIndex"), "click-to-remap state missing");
assert.ok(settings.includes("startRecording(row.id, index)"), "chord click must start remap");
assert.ok(settings.includes("chord--recording"), "remap target highlight missing");

// Shortcut overrides persist to shortcut-overrides.json via the Rust backend,
// not localStorage (legacy key is a one-time migration source only).
const shortcutsConfig = read("src/shortcuts/config.ts");
const shortcutsRs = read("src-tauri/src/shortcuts/mod.rs");
const libRs = read("src-tauri/src/lib.rs");
assert.ok(shortcutsConfig.includes("shortcuts_overrides_get"), "config must seed from backend");
assert.ok(shortcutsConfig.includes("shortcuts_overrides_set"), "config must write through backend");
assert.ok(shortcutsConfig.includes("hydrateShortcutOverridesFromBackend"));
assert.ok(
  !shortcutsConfig.includes("localStorage.setItem"),
  "localStorage must be migration source only",
);
assert.ok(
  main.includes("hydrateShortcutOverridesFromBackend"),
  "bootstrap must hydrate overrides before mount",
);
assert.ok(shortcutsRs.includes("shortcut-overrides.json"));
assert.ok(shortcutsRs.includes("sanitize_overrides"));
assert.ok(shortcutsRs.includes("MAX_CHORDS_PER_ACTION"));
assert.ok(libRs.includes("shortcuts::shortcuts_overrides_get"));
assert.ok(libRs.includes("shortcuts::shortcuts_overrides_set"));

// Floating hints must resolve FINAL chords and be wired on nav + explorer actions.
assert.ok(main.includes("vActionTip"), "global v-action-tip directive missing");
assert.ok(shell.includes("ActionTipHost"));
assert.ok(shell.includes("v-action-tip"));
assert.ok(tipHost.includes("effectiveChordLabelsForId"));
assert.ok(contextMenu.includes("effectiveChordLabelsForId"));
assert.ok(contextMenu.includes("v-action-tip"));
assert.ok(!contextMenu.includes("BUILTIN_SHORTCUTS"), "context menu must not read builtins directly");

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/shortcut-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);
