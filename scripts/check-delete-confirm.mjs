// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-185: Fluent delete confirm + 「以后不再提问」 → power.confirmBeforeDelete.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const defaults = readFileSync(join(root, "src-tauri/src/settings/defaults.rs"), "utf8");
const fields = readFileSync(join(root, "src/i18n/settingsFields.ts"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const host = readFileSync(
  join(root, "src/components/shell/ConfirmDialogHost.vue"),
  "utf8",
);
const dialogState = readFileSync(join(root, "src/state/confirmDialog.ts"), "utf8");
const deleteState = readFileSync(
  join(root, "src/state/confirmBeforeDelete.ts"),
  "utf8",
);
const nav = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const menu = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const zh = JSON.parse(readFileSync(join(root, "src/i18n/locales/zh-CN.json"), "utf8"));

assert.ok(defaults.includes('"power.confirmBeforeDelete"'));
assert.match(
  defaults,
  /"power\.confirmBeforeDelete"[\s\S]{0,400}json!\(true\)/,
);
assert.ok(fields.includes('"power.confirmBeforeDelete"'));
assert.ok(dialogState.includes("openConfirmDialog"));
assert.ok(dialogState.includes("showDontAskAgain"));
assert.ok(deleteState.includes("CONFIRM_BEFORE_DELETE_KEY"));
assert.ok(deleteState.includes("confirmDeleteIfNeeded"));
assert.ok(deleteState.includes("persistConfirmBeforeDelete"));
assert.ok(host.includes("confirm-dialog-backdrop"));
assert.ok(host.includes("dialog.dontAskAgain") || host.includes("dontAskAgain"));
assert.ok(host.includes('align-items: center'));
assert.ok(host.includes("justify-content: center"));
assert.ok(shell.includes("ConfirmDialogHost"));
assert.ok(nav.includes("confirmDeleteIfNeeded"));
assert.ok(!nav.includes("window.confirm"));
assert.ok(menu.includes("confirmDeleteIfNeeded"));
assert.ok(!menu.includes("window.confirm"));
assert.equal(zh.dialog.dontAskAgain, "以后不再提问");
assert.equal(en.dialog.dontAskAgain, "Don't ask again");
assert.equal(
  zh.settings.fields.power.confirmBeforeDelete.title,
  "删除前确认",
);

console.log("delete-confirm checks OK (AIA-185).");
