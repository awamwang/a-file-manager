// SPDX-License-Identifier: GPL-3.0-or-later
// Behavioral unit for the delete-confirm gate's pure logic. Pinned by scripts/check-delete-confirm.mjs.
// The async orchestration (loadConfirmBeforeDelete / confirmDeleteIfNeeded) needs Tauri + a DOM
// dialog, so this pins the one pure contract: the default-on parse rule the whole gate rests on.
import assert from "node:assert/strict";
import {
  parseConfirmBeforeDelete,
  CONFIRM_BEFORE_DELETE_KEY,
} from "../src/state/confirmBeforeDelete.ts";

assert.equal(CONFIRM_BEFORE_DELETE_KEY, "power.confirmBeforeDelete");

// Default-on: anything except a strict boolean false keeps the confirm prompt enabled,
// so a missing / corrupt / unset setting can never silently disable delete confirmation.
for (const value of [undefined, null, true, "false", 0, 1, {}, []]) {
  assert.equal(
    parseConfirmBeforeDelete(value),
    true,
    `parseConfirmBeforeDelete must default ON for ${JSON.stringify(value)}`,
  );
}
// Only an explicit `false` (the user checked "don't ask again") disables it.
assert.equal(parseConfirmBeforeDelete(false), false);

console.log(
  "confirm-before-delete-checks OK (default-on parse rule: only strict false disables the delete prompt).",
);
