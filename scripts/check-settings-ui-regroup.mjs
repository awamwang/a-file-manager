// SPDX-License-Identifier: GPL-3.0-or-later
/** FIL-59/65: settings UI regroup, effective defaults, favorites in Advanced. */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/SettingsView.vue"), "utf8");
const ui = readFileSync(join(root, "src/state/settingsUi.ts"), "utf8");
const effective = readFileSync(
  join(root, "src/state/settingsEffectiveDefaults.ts"),
  "utf8",
);
const defaults = readFileSync(join(root, "src-tauri/src/settings/defaults.rs"), "utf8");
const fields = readFileSync(join(root, "src/i18n/settingsFields.ts"), "utf8");
const zh = readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8");

assert.ok(ui.includes("SETTINGS_UI_HIDDEN_KEYS"));
assert.ok(ui.includes("power.awamGatewayUrl"));
assert.ok(ui.includes("experimental.debugStatusInTitlebar"));
assert.ok(ui.includes("core.discoveryBlacklist"));
assert.ok(ui.includes('"projects"'));
assert.ok(/"core\.projectsRoot":\s*"projects"/.test(ui));

assert.ok(view.includes("settingsUi"));
assert.ok(view.includes("visibleFieldGroups"));
assert.ok(view.includes("field-group-title"));
assert.ok(!view.includes("sectionHiddenNote"));
assert.ok(!/id:\s*"favorites"\s+as const/.test(view));
assert.ok(view.includes("favorites-in-advanced"));
assert.ok(view.includes("setFavoritesOsHookEnabled"));
assert.ok(view.includes("stringFieldDisplayValue"));
assert.ok(view.includes("settingsEffectiveDefaults"));

assert.ok(effective.includes("SETTINGS_EMPTY_MEANS_DEFAULT_KEYS"));
assert.ok(effective.includes("core.defaultLocation"));
assert.ok(effective.includes("core.projectsRoot"));

assert.ok(zh.includes("项目的公共配置"));
assert.ok(zh.includes("（默认：{default}）"));
assert.ok(!zh.includes("空=使用当前语言默认"));
assert.ok(!zh.includes("双入口"));
assert.ok(!zh.includes('"hiddenNotes"'));

for (const key of [
  "core.language",
  "core.everythingPreferred",
  "power.awamGatewayUrl",
  "experimental.debugStatusInTitlebar",
  "preview.externalProvider",
]) {
  assert.ok(fields.includes(`"${key}"`), `schema key retained: ${key}`);
  assert.ok(defaults.includes(`"${key}"`), `rust definition retained: ${key}`);
}

assert.ok(
  /"core.everythingPreferred"[\s\S]*SettingsTier::Power/.test(defaults),
  "everythingPreferred display tier is Power",
);
assert.ok(
  /"core.extensionsEnabled"[\s\S]*SettingsTier::Power/.test(defaults),
  "extensionsEnabled display tier is Power",
);
assert.ok(
  /"experimental.debugStatusInTitlebar"[\s\S]*json!\(false\)/.test(defaults),
  "debug titlebar default off for new installs",
);

const registerUrl = pathToFileURL(join(root, "scripts/register-ts-resolve.mjs")).href;
const unit = spawnSync(
  process.execPath,
  [
    "--import",
    registerUrl,
    "--experimental-strip-types",
    join(root, "scripts/settings-ui-regroup-checks.ts"),
  ],
  { cwd: root, encoding: "utf8" },
);
if (unit.stdout) process.stdout.write(unit.stdout);
if (unit.stderr) process.stderr.write(unit.stderr);
assert.equal(unit.status, 0, "settings-ui-regroup-checks must pass");

console.log("settings UI regroup checks OK (FIL-59/65)");
