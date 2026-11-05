// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-148: Windows-style root crumb + new-tab guide + settings.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const browse = readFileSync(join(root, "src/state/navigatorBrowse.ts"), "utf8");
const guideState = readFileSync(join(root, "src/state/navigatorGuide.ts"), "utf8");
const behavior = readFileSync(join(root, "src/state/newTabBehavior.ts"), "utf8");
const defaults = readFileSync(join(root, "src-tauri/src/settings/defaults.rs"), "utf8");
const fields = readFileSync(join(root, "src/i18n/settingsFields.ts"), "utf8");
const navView = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));

const guidePath = readFileSync(join(root, "src/state/guidePath.ts"), "utf8");
assert.ok(guidePath.includes('GUIDE_PATH = "afm://guide"'));
assert.ok(guideState.includes("GUIDE_PATH"));
assert.ok(shell.includes("buildNavigatorBreadcrumbs"));
assert.ok(shell.includes('crumb.kind === "root"') || shell.includes("crumb.kind === 'root'"));
assert.ok(shell.includes("loadNavigatorPlaces"));
assert.ok(shell.includes("resolveHomePath"));
assert.ok(browse.includes("readNewTabBehavior"));
assert.ok(browse.includes("GUIDE_PATH"));
assert.ok(behavior.includes("core.newTabBehavior"));
assert.ok(defaults.includes('"core.newTabBehavior"'));
assert.ok(defaults.includes('"guide"'));
assert.ok(defaults.includes('"duplicate"'));
assert.ok(fields.includes('"core.newTabBehavior"'));
assert.ok(navView.includes("NavigatorGuide"));
assert.ok(navView.includes("isGuidePath"));
assert.equal(en.settings.fields.core.newTabBehavior.options.guide, "Guide page");
assert.equal(zh.settings.fields.core.newTabBehavior.options.guide, "引导页");
assert.equal(en.guide.rootLabel, "This PC");
assert.equal(zh.guide.rootLabel, "此电脑");

const history = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/nav-history-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (history.stdout) process.stdout.write(history.stdout);
if (history.stderr) process.stderr.write(history.stderr);
assert.equal(history.status, 0, "nav history checks failed");

console.log("navigator guide / root crumb checks OK (AIA-148).");
