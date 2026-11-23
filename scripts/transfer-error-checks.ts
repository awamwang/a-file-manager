// SPDX-License-Identifier: GPL-3.0-or-later
// Behavioral unit for the per-item transfer-error Retry / Skip / Abort dialog
// (LC-006 / LC-007 / LC-008). Pinned by scripts/check-transfer-error.mjs.
// transferError.ts only depends on `vue`'s reactivity `ref`, which resolves
// headless here.
import assert from "node:assert/strict";

const {
  isRetryableTransferErrorClass,
  openTransferErrorDialog,
  resolveTransferError,
  resolveTransferErrorChoice,
  transferErrorClass,
  transferErrorName,
  transferErrorOpen,
  transferErrorPending,
} = await import("../src/state/transferError.ts");

// The three recognized classes are the ONLY ones that get a dialog. These exact
// strings are the contract with the backend
// (file_ops/mod.rs ERROR_CLASS_IN_USE / _ACCESS_DENIED / _DISK_FULL).
assert.equal(isRetryableTransferErrorClass("in-use"), true);
assert.equal(isRetryableTransferErrorClass("access-denied"), true);
assert.equal(isRetryableTransferErrorClass("disk-full"), true);

// Hard refusals / successes / missing class are NOT retryable → whole batch aborts
// exactly as before (folder-into-itself, volume root, validation, etc.).
assert.equal(isRetryableTransferErrorClass(undefined), false);
assert.equal(isRetryableTransferErrorClass(null), false);
assert.equal(isRetryableTransferErrorClass(""), false);
assert.equal(isRetryableTransferErrorClass("into-itself"), false);
assert.equal(isRetryableTransferErrorClass("validation"), false);

// Dialog open / settle lifecycle.
assert.equal(transferErrorOpen.value, false, "starts closed");
assert.equal(transferErrorPending(), false, "no pending resolve at start");

const first = openTransferErrorDialog("in-use", "locked.txt");
assert.equal(transferErrorOpen.value, true, "opening shows the dialog");
assert.equal(transferErrorClass.value, "in-use", "class surfaced to the dialog");
assert.equal(transferErrorName.value, "locked.txt", "failed item name surfaced");
assert.equal(transferErrorPending(), true, "awaiting a user choice");

resolveTransferError("skip");
assert.equal(await first, "skip", "skip choice resolves the pending dialog");
assert.equal(transferErrorOpen.value, false, "closed after resolve");
assert.equal(transferErrorClass.value, "", "class cleared on settle");
assert.equal(transferErrorName.value, "", "name cleared on settle");
assert.equal(transferErrorPending(), false, "no pending resolve after settle");

// A dialog replaced before the user answers auto-settles the previous one as
// `abort` (safe default: an unanswered batch stops, never silently proceeds).
const replacedFirst = openTransferErrorDialog("access-denied", "a.txt");
const replacedSecond = openTransferErrorDialog("disk-full", "b.txt");
assert.equal(await replacedFirst, "abort", "replaced dialog defaults to abort");
assert.equal(transferErrorClass.value, "disk-full", "latest dialog stays open");
resolveTransferError("retry");
assert.equal(await replacedSecond, "retry", "latest dialog honors the user choice");

// resolveTransferErrorChoice is the promise wrapper transferEntries awaits.
const viaChoice = resolveTransferErrorChoice("in-use", "c.txt");
assert.equal(transferErrorOpen.value, true, "wrapper opens the dialog");
resolveTransferError("abort");
assert.equal(await viaChoice, "abort", "wrapper resolves the choice");
assert.equal(transferErrorOpen.value, false, "wrapper closes on settle");

console.log(
  "transfer-error-checks OK (three retryable classes only; hard refusals not retryable; open/settle lifecycle clears state; replaced dialog defaults to abort; wrapper resolves the choice).",
);
