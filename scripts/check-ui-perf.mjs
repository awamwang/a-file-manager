// SPDX-License-Identifier: GPL-3.0-or-later
/** AIA-269: cheap UI perf wins — focus debounce + cached modified formatter. */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const nav = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");

assert.ok(nav.includes("FOCUS_REFRESH_DEBOUNCE_MS"), "focus debounce constant");
assert.ok(nav.includes("Intl.DateTimeFormat"), "cached modified date formatter");
assert.ok(nav.includes("modifiedDateFormatter"), "reuse DateTimeFormat across rows");

const focusStart = nav.indexOf("function onWindowFocus");
assert.ok(focusStart >= 0, "onWindowFocus required");
const focusEnd = nav.indexOf("\nonMounted", focusStart);
const focusBody = nav.slice(focusStart, focusEnd > focusStart ? focusEnd : focusStart + 500);
assert.ok(focusBody.includes("refreshClipboardFromSystem"), "focus refreshes clipboard");
assert.ok(focusBody.includes("refreshActiveListing"), "focus re-lists");
assert.ok(!focusBody.includes("loadNavigatorViewMode"), "must not hydrate view-mode on focus");
assert.ok(!focusBody.includes("refreshPreviewSetting"), "must not hydrate preview on focus");

console.log("UI perf cheap-win checks OK (AIA-269).");

// AIA-278: thumbnail scroll queue — see check-shell-icons.mjs / check-shell-icon-queue.mjs.
assert.ok(nav.includes("clearShellIconQueue"), "folder change cancels thumb queue");
assert.ok(nav.includes("cancelShellIconRequests"), "out-of-view thumb cancel");
assert.ok(nav.includes("ShellIconImg"), "per-row shell icon isolate");
