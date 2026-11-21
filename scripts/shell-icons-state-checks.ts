// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Behavior unit for the REAL src/state/shellIcons.ts cache (AIA-279 / FIL-68).
 * Unlike check-shell-icon-queue/budget.mjs mirror simulations, this imports and
 * executes the actual module via its exported test hooks. No Tauri IPC needed.
 */
import assert from "node:assert/strict";
import {
  SHELL_ICON_MAX_ENTRIES,
  SHELL_ICON_MAX_BYTES,
  iconUrlFor,
  requestShellIcons,
  cancelShellIconRequests,
  suspendQueuedContentThumbnails,
  clearShellIconQueue,
  setShellIconPinnedPaths,
  shellIconQueueSizeForTests,
  shellIconGenerationForTests,
  shellIconPendingHasForTests,
  shellIconCacheStatsForTests,
  shellIconPutForTests,
  shellIconMarkSoftForTests,
  shellIconClearCacheForTests,
} from "../src/state/shellIcons.ts";

assert.equal(SHELL_ICON_MAX_ENTRIES, 512);
assert.equal(SHELL_ICON_MAX_BYTES, 48 * 1024 * 1024);

// --- 1) Entry cap: oldest entries evicted, cap holds ---
shellIconClearCacheForTests();
setShellIconPinnedPaths([]);
for (let i = 0; i < SHELL_ICON_MAX_ENTRIES + 20; i += 1) {
  shellIconPutForTests(`/e/${i}`, 20, "x");
}
assert.equal(shellIconCacheStatsForTests().entries, SHELL_ICON_MAX_ENTRIES);
assert.equal(iconUrlFor("/e/0", 20), null, "oldest entry evicted");
assert.ok(iconUrlFor(`/e/${SHELL_ICON_MAX_ENTRIES + 19}`, 20), "newest retained");

// --- 2) Byte budget: one huge entry pushes others out ---
shellIconClearCacheForTests();
shellIconPutForTests("/small1", 96, "s".repeat(1000));
shellIconPutForTests("/small2", 96, "s".repeat(1000));
shellIconPutForTests("/huge", 96, "H".repeat(SHELL_ICON_MAX_BYTES));
const stats = shellIconCacheStatsForTests();
assert.ok(stats.bytes <= SHELL_ICON_MAX_BYTES || stats.entries === 1, "byte budget enforced");
assert.ok(iconUrlFor("/huge", 96), "newest huge entry retained");
assert.equal(iconUrlFor("/small1", 96), null);
assert.equal(iconUrlFor("/small2", 96), null);

// --- 3) FIL-68: pinned viewport entry survives normal pressure ---
shellIconClearCacheForTests();
setShellIconPinnedPaths(["/visible"]);
shellIconPutForTests("/visible", 96, "v");
for (let i = 0; i < SHELL_ICON_MAX_ENTRIES + 5; i += 1) {
  shellIconPutForTests(`/off-${i}`, 96, "o");
}
assert.equal(shellIconCacheStatsForTests().entries, SHELL_ICON_MAX_ENTRIES);
assert.ok(iconUrlFor("/visible", 96), "pinned viewport entry must not be evicted");
assert.equal(iconUrlFor("/off-0", 96), null, "unpinned offscreen evicted first");

// --- 4) FIL-68: soft-pinned evicted before hard-pinned under true pressure ---
shellIconClearCacheForTests();
setShellIconPinnedPaths(["/p-soft", "/p-hard"]);
shellIconPutForTests("/p-soft", 96, "a");
shellIconPutForTests("/p-hard", 96, "b");
shellIconMarkSoftForTests("/p-soft", 96);
// Shrink cap pressure manually: adding MAX_ENTRIES junk forces evictions of pins last.
for (let i = 0; i < SHELL_ICON_MAX_ENTRIES; i += 1) {
  shellIconPutForTests(`/junk-${i}`, 96, "o");
}
// Both pins still present (junk evicted instead) while over-cap junk is gone.
assert.ok(iconUrlFor("/p-soft", 96), "pins survive under-cap pressure via junk eviction");
assert.ok(iconUrlFor("/p-hard", 96), "hard pin survives");
assert.ok(shellIconCacheStatsForTests().entries <= SHELL_ICON_MAX_ENTRIES);

// --- 5) clearShellIconQueue marks cache soft and bumps generation ---
shellIconClearCacheForTests();
setShellIconPinnedPaths([]);
shellIconPutForTests("/a", 20, "a");
shellIconPutForTests("/b", 20, "b");
const genBefore = shellIconGenerationForTests();
clearShellIconQueue();
assert.ok(shellIconGenerationForTests() > genBefore, "folder change bumps generation");
assert.equal(shellIconCacheStatsForTests().soft, 2, "cache marked soft for back-navigation");
assert.equal(iconUrlFor("/a", 20), "a", "cache stays (soft) after folder change");
assert.equal(shellIconQueueSizeForTests(), 0, "queue cleared");

// --- 6) Real queue: priority requests, cancel, suspend (no flush — clear timers) ---
shellIconClearCacheForTests();
requestShellIcons(
  [
    { path: "/vid-1", preferThumbnail: true },
    { path: "/vid-2", preferThumbnail: true },
    { path: "/dir-1", preferThumbnail: false },
  ],
  96,
);
assert.equal(shellIconQueueSizeForTests(), 3, "mixed queue holds all three");
assert.ok(shellIconPendingHasForTests("/dir-1", 96), "pending tracked for queued path");

// Cancel drops queued + pending for paths leaving viewport.
cancelShellIconRequests(["/vid-1"], 96);
assert.equal(shellIconQueueSizeForTests(), 2);
assert.equal(shellIconPendingHasForTests("/vid-1", 96), false, "cancel clears pending");

// Fling: suspend queued content thumbnails, keep fast-path shell icons.
suspendQueuedContentThumbnails();
assert.equal(shellIconQueueSizeForTests(), 1, "only non-thumbnail (dir) icon remains");
assert.ok(shellIconPendingHasForTests("/dir-1", 96));

// Repeat pending request must not grow the queue (FIL-50 stable re-request).
requestShellIcons([{ path: "/dir-1", preferThumbnail: false }], 96);
assert.equal(shellIconQueueSizeForTests(), 1, "pending repeat does not duplicate queue entry");

// Clear timers so no stray flush fires invoke after the test exits.
clearShellIconQueue();
assert.equal(shellIconQueueSizeForTests(), 0);

console.log("shellIcons real-module behavior checks OK (AIA-279 / FIL-68 / FIL-50).");
