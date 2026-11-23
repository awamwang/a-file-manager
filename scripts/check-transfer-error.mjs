// SPDX-License-Identifier: GPL-3.0-or-later
// Guard: per-item transfer error Retry / Skip / Abort (LC-006 / LC-007 / LC-008).
//
// Layers (ADR 0025):
//   FE     — scripts/transfer-error-checks.ts executes the dialog state machine
//            (retryable-class filter, open/settle lifecycle, abort default).
//   STATIC — this gate pins the FE↔BE error-class NAME parity, the wiring of the
//            dialog into the paste/drop transfer loop, and the mounted host so a
//            rename or a dropped button fails red instead of silently removing
//            the per-item dialog.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const fe = read("src/state/transferError.ts");
const clip = read("src/state/fileClipboard.ts");
const be = read("src-tauri/src/file_ops/mod.rs");
const hostRel = "src/components/shell/TransferErrorDialogHost.vue";
const shell = read("src/components/shell/AppShell.vue");
const en = JSON.parse(read("src/i18n/locales/en.json"));

// 1. Decision API surface in the FE module.
for (const symbol of [
  "export function isRetryableTransferErrorClass",
  "export function openTransferErrorDialog",
  "export function resolveTransferError",
  "export async function resolveTransferErrorChoice",
]) {
  assert.ok(fe.includes(symbol), `transferError.ts missing: ${symbol}`);
}

// LC-007 / CLIP-008: this dialog is per-item and per-operation only — nothing is
// remembered or persisted. If a localStorage/sessionStorage default sneaks back,
// fail.
assert.ok(!fe.includes("localStorage"), "transferError.ts must not read/write localStorage");
assert.ok(!fe.includes("sessionStorage"), "transferError.ts must not use sessionStorage");

// 2. FE↔BE error-class NAME parity. The three classes the dialog treats as
// retryable MUST be the exact strings the backend emits, or every per-item
// failure would silently fall through to the whole-batch abort (a silent
// regression of LC-007).
const BE_CONSTS = {
  "in-use": /ERROR_CLASS_IN_USE:\s*&str\s*=\s*"in-use"/,
  "access-denied": /ERROR_CLASS_ACCESS_DENIED:\s*&str\s*=\s*"access-denied"/,
  "disk-full": /ERROR_CLASS_DISK_FULL:\s*&str\s*=\s*"disk-full"/,
};
for (const [name, re] of Object.entries(BE_CONSTS)) {
  assert.ok(re.test(be), `backend lost error class constant for "${name}"`);
  assert.ok(fe.includes(`"${name}"`), `FE retryable set missing class "${name}"`);
}
// The FE must recognize exactly these three and reject anything else.
assert.ok(
  /RETRYABLE_CLASSES[\s\S]*"in-use"[\s\S]*"access-denied"[\s\S]*"disk-full"/.test(fe),
  "FE retryable set must list exactly in-use / access-denied / disk-full",
);

// 3. The paste/drop transfer loop (transferEntries) must consume the class:
//    retry re-invokes, skip continues, abort rolls back completed copies (LC-005)
//    and never pushes the undo stack (UN-009).
assert.ok(clip.includes("isRetryableTransferErrorClass"), "transferEntries must gate on retryable class");
assert.ok(clip.includes("resolveTransferErrorChoice"), "transferEntries must await the dialog choice");
assert.ok(
  /while \(!outcome\.ok && isRetryableTransferErrorClass/.test(clip),
  "transferEntries must loop per-item on a retryable failure",
);
assert.ok(/choice === "retry"/.test(clip), "transferEntries must support Retry");
assert.ok(/choice === "skip"/.test(clip), "transferEntries must support Skip");
assert.ok(
  /mode === "copy"[\s\S]*?deleteEntries\(transferred, true\)/.test(clip),
  "Abort must roll back completed copies (LC-005) via permanent delete",
);
assert.ok(clip.includes("errorClass"), "FileOpsResult must carry the per-item errorClass");

// 4. The dialog host exists, wires the three actions, and is mounted in AppShell.
assert.ok(existsSync(join(root, hostRel)), "TransferErrorDialogHost.vue is missing");
const host = read(hostRel);
for (const needle of [
  "resolveTransferError('retry')",
  "resolveTransferError('skip')",
  "resolveTransferError('abort')",
]) {
  assert.ok(host.includes(needle), `dialog host missing action: ${needle}`);
}
assert.ok(
  host.includes('t("dialog.transferErrorRetry")') &&
    host.includes('t("dialog.transferErrorSkip")') &&
    host.includes('t("dialog.transferErrorAbort")'),
  "dialog host buttons must use the transferError i18n keys",
);
assert.ok(shell.includes("import TransferErrorDialogHost"), "AppShell must import the dialog host");
assert.ok(shell.includes("<TransferErrorDialogHost />"), "AppShell must mount the dialog host");

// 5. i18n keys present (check-i18n enforces parity across locales).
for (const key of ["transferErrorTitle", "transferErrorMessage", "transferErrorRetry", "transferErrorSkip", "transferErrorAbort"]) {
  assert.ok(typeof en.dialog[key] === "string" && en.dialog[key].length, `en.json dialog.${key} missing`);
}
assert.ok(en.dialog.transferErrorMessage.includes("{name}"), "transferErrorMessage must interpolate {name}");

// 6. Behavior unit (headless).
const unit = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/transfer-error-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (unit.stdout) process.stdout.write(unit.stdout);
if (unit.stderr) process.stderr.write(unit.stderr);
assert.equal(unit.status, 0, "transfer-error behavior checks failed");

console.log(
  "transfer-error checks OK (per-item Retry/Skip/Abort dialog; FE↔BE error-class parity; abort rolls back copies; dialog host mounted; not persisted).",
);
