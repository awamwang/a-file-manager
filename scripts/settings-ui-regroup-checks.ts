// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  compareSettingsUiKeys,
  isSettingsUiHiddenKey,
  SETTINGS_UI_HIDDEN_KEYS,
  settingsUiGroupForKey,
} from "../src/state/settingsUi.ts";
import {
  displaySettingsStringValue,
  isSettingsEmptyMeansDefaultKey,
  normalizeSettingsStringForSave,
} from "../src/state/settingsEffectiveDefaults.ts";

assert.ok(SETTINGS_UI_HIDDEN_KEYS.includes("power.awamGatewayUrl"));
assert.ok(isSettingsUiHiddenKey("experimental.debugStatusInTitlebar"));
assert.equal(isSettingsUiHiddenKey("core.language"), false);

assert.equal(settingsUiGroupForKey("core.theme"), "appearance");
assert.equal(settingsUiGroupForKey("core.startupPage"), "startup");
assert.equal(settingsUiGroupForKey("core.showHidden"), "files");
assert.equal(settingsUiGroupForKey("core.projectsRoot"), "projects");
assert.equal(settingsUiGroupForKey("power.confirmBeforeDelete"), "safety");
assert.equal(settingsUiGroupForKey("experimental.previewPane"), "preview");

assert.ok(compareSettingsUiKeys("core.language", "core.projectsRoot") < 0);
assert.ok(compareSettingsUiKeys("core.newFileDefaultName", "core.projectsRoot") < 0);
assert.ok(compareSettingsUiKeys("core.projectsRoot", "power.confirmBeforeDelete") < 0);

assert.ok(isSettingsEmptyMeansDefaultKey("core.defaultLocation"));
assert.equal(
  displaySettingsStringValue("", "core.defaultLocation", {
    "core.defaultLocation": "C:\\Users\\Demo",
  }),
  "C:\\Users\\Demo",
);
assert.equal(
  displaySettingsStringValue("D:\\Home", "core.defaultLocation", {
    "core.defaultLocation": "C:\\Users\\Demo",
  }),
  "D:\\Home",
);
assert.equal(
  normalizeSettingsStringForSave("core.defaultLocation", "C:\\Users\\Demo", {
    "core.defaultLocation": "C:\\Users\\Demo",
  }),
  "",
);

console.log("settings-ui-regroup-checks OK");
