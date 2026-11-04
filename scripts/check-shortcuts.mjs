// SPDX-License-Identifier: GPL-3.0-or-later
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const settings = readFileSync(join(root, "src/views/SettingsView.vue"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const router = readFileSync(join(root, "src/router/index.ts"), "utf8");

assert.ok(settings.includes("sectionShortcuts") || settings.includes("settings.sectionShortcuts"));
assert.ok(settings.includes("listShortcutsForDisplay"));
assert.ok(shell.includes("dispatchShortcutEvent"));
assert.ok(shell.includes("registerShortcutHandler"));
assert.ok(router.includes('path: "/settings/shortcuts"'));

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/shortcut-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);
