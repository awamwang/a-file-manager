// SPDX-License-Identifier: GPL-3.0-or-later
// Behavioral unit for the Fluent prompt dialog state machine. Pinned by scripts/check-prompt-dialog.mjs.
// promptDialog.ts imports only vue `ref`, so the promise lifecycle is testable without a browser.
import assert from "node:assert/strict";
import {
  openPromptDialog,
  resolvePromptDialog,
  cancelPromptDialog,
  promptDialogOpen,
  promptDialogValue,
  promptDialogBusy,
  promptDialogRequest,
} from "../src/state/promptDialog.ts";

async function main(): Promise<void> {
  // 1. open seeds defaultValue, marks the dialog open, and confirm resolves with the edited value.
  const p1 = openPromptDialog({ title: "Rename", defaultValue: "docs" });
  assert.equal(promptDialogOpen.value, true);
  assert.equal(promptDialogValue.value, "docs");
  assert.equal(promptDialogRequest.value?.title, "Rename");
  promptDialogValue.value = "reports";
  resolvePromptDialog(true);
  assert.deepEqual(await p1, { confirmed: true, value: "reports" });
  assert.equal(promptDialogOpen.value, false, "settle closes the dialog");

  // 2. absent defaultValue -> empty string; cancel resolves confirmed:false.
  const p2 = openPromptDialog({ title: "New" });
  assert.equal(promptDialogValue.value, "");
  resolvePromptDialog(false);
  assert.deepEqual(await p2, { confirmed: false, value: "" });

  // 3. re-open while a dialog is pending auto-cancels the previous caller so it never hangs.
  const pA = openPromptDialog({ title: "A" });
  const pB = openPromptDialog({ title: "B" });
  assert.deepEqual(await pA, { confirmed: false, value: "" }, "previous promise must not hang");
  resolvePromptDialog(true);
  assert.deepEqual(await pB, { confirmed: true, value: "" });

  // 4. cancel is ignored while busy (guards against double-submit races).
  const pC = openPromptDialog({ title: "C" });
  promptDialogBusy.value = true;
  cancelPromptDialog();
  assert.equal(promptDialogOpen.value, true, "a busy dialog must ignore cancel");
  promptDialogBusy.value = false;
  cancelPromptDialog();
  assert.deepEqual(await pC, { confirmed: false, value: "" });
}

await main();
console.log(
  "prompt-dialog-checks OK (defaultValue seed, edited-value resolve, pending re-open auto-cancel, busy-cancel guard).",
);
