// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AIA-278 / FIL-33: unit-ish checks for thumbnail queue cancel / priority / leave-reenter
 * without Tauri. Source contract + lightweight in-process simulation mirrored from shellIcons.
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const state = readFileSync(join(root, "src/state/shellIcons.ts"), "utf8");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");

// --- Contract: queue module exposes cancel + generation bump ---
assert.ok(/export function clearShellIconQueue/.test(state));
assert.ok(/export function cancelShellIconRequests/.test(state));
assert.ok(/generation \+= 1/.test(state));
assert.ok(/BATCH_THUMB = 8/.test(state));
assert.ok(/options\?\.priority === true/.test(state));
assert.ok(/requestAnimationFrame/.test(state), "coalesce reactive icon applies");
assert.ok(/SHELL_ICON_MAX_ENTRIES/.test(state), "AIA-279: bounded cache");
assert.ok(/trimToBudget/.test(state), "AIA-279: eviction under budget");
assert.ok(/attemptByKey/.test(state), "FIL-33: leave/reenter attempt tokens");
assert.ok(/forgetPendingKey/.test(state), "FIL-33: cancel clears pending even if in-flight");
assert.ok(
  /仍在排队\/in-flight：保留 attempt|保留 attempt/.test(state),
  "FIL-50: repeat request while pending must keep attempt",
);
assert.ok(/promotePriorityWindow/.test(state), "priority window keeps top-to-bottom order");
assert.ok(/takeNextBatch/.test(state), "icon-first drain before thumbnail batches");
assert.ok(
  /不因 priority 反复 unshift|不挪动位置/.test(state),
  "pending queue items must not reshuffle on every viewport sync",
);
assert.ok(
  view.includes("preferThumbnail: iconView && entry.kind !== \"dir\"") ||
    view.includes("preferThumbnail: iconView && entry.kind !== 'dir'"),
  "icon view: directories request shell icons only (no thumbnail)",
);

// --- Contract: Navigator wires folder / scroll cancel ---
assert.ok(/watch\(currentPath/.test(view) && view.includes("clearShellIconQueue()"));
assert.ok(view.includes("cancelShellIconRequests"));
assert.ok(view.includes("computeThumbRequestWindow"), "FIL-50: geometry thumb window");
assert.ok(view.includes("syncThumbRequestsForPane"), "FIL-50: pane thumb sync");
assert.ok(view.includes("scheduleViewportIconObservers"));
assert.ok(view.includes("pathsLeavingWindow"), "FIL-33: window diff cancel");
assert.ok(view.includes("primaryThumbPaths"), "FIL-50: track thumb request window paths");
assert.ok(
  !/new\s+IntersectionObserver/.test(view),
  "FIL-50: no IntersectionObserver thumb scheduling",
);

// --- Lightweight queue behavior model (mirrors shellIcons.ts rules) ---
function simulate() {
  /** @type {{ path: string, size: number, attempt: number }[]} */
  const queue = [];
  const pending = new Set();
  /** @type {Map<string, number>} */
  const attemptByKey = new Map();
  let nextAttempt = 1;
  let generation = 0;
  /** @type {Set<string>} */
  const cache = new Set();

  function key(path, size) {
    return `${size}|${path}`;
  }

  function settle(k) {
    pending.delete(k);
    attemptByKey.delete(k);
  }

  function enqueue(path, size, priority, preferThumbnail = true) {
    const k = key(path, size);
    const attempt = nextAttempt++;
    attemptByKey.set(k, attempt);
    const idx = queue.findIndex((q) => key(q.path, q.size) === k);
    if (idx >= 0) {
      queue[idx].attempt = attempt;
      queue[idx].preferThumbnail = preferThumbnail;
      // Stable: do not unshift already-queued rows.
      return;
    }
    const item = { path, size, attempt, preferThumbnail };
    if (priority) queue.unshift(item);
    else queue.push(item);
  }

  function promotePriorityWindow(items) {
    const keys = new Set(items.map((q) => key(q.path, q.size)));
    for (let i = queue.length - 1; i >= 0; i -= 1) {
      if (keys.has(key(queue[i].path, queue[i].size))) queue.splice(i, 1);
    }
    queue.unshift(...items);
  }

  function requestWindow(paths, size, priority) {
    if (!priority) {
      for (const path of paths) request(path, size, false);
      return;
    }
    const items = [];
    for (const path of paths) {
      const k = key(path, size);
      if (cache.has(k)) continue;
      let attempt;
      if (pending.has(k)) {
        attempt = attemptByKey.get(k) ?? nextAttempt++;
        attemptByKey.set(k, attempt);
      } else {
        attempt = nextAttempt++;
        attemptByKey.set(k, attempt);
        pending.add(k);
      }
      items.push({ path, size, attempt, preferThumbnail: true });
    }
    promotePriorityWindow(items);
  }

  function request(path, size, priority, preferThumbnail = true) {
    const k = key(path, size);
    if (cache.has(k)) return;
    if (pending.has(k)) {
      // FIL-50: keep attempt while pending (viewport geometry may re-request).
      const existing = attemptByKey.get(k) ?? nextAttempt++;
      attemptByKey.set(k, existing);
      const idx = queue.findIndex((q) => key(q.path, q.size) === k);
      if (idx >= 0) {
        queue[idx].attempt = existing;
        queue[idx].preferThumbnail = preferThumbnail;
      } else if (priority) {
        queue.unshift({ path, size, attempt: existing, preferThumbnail });
      } else {
        queue.push({ path, size, attempt: existing, preferThumbnail });
      }
      return;
    }
    pending.add(k);
    enqueue(path, size, priority, preferThumbnail);
  }

  function takeNextBatch(batchIcon = 40, batchThumb = 8) {
    const iconIndices = [];
    for (let i = 0; i < queue.length; i += 1) {
      if (queue[i].preferThumbnail !== true) iconIndices.push(i);
    }
    if (iconIndices.length) {
      const take = Math.min(batchIcon, iconIndices.length);
      const batch = [];
      for (let i = take - 1; i >= 0; i -= 1) {
        batch.unshift(queue.splice(iconIndices[i], 1)[0]);
      }
      return batch;
    }
    return queue.splice(0, batchThumb);
  }

  function cancel(paths, size) {
    const set = new Set(paths);
    for (let i = queue.length - 1; i >= 0; i -= 1) {
      const item = queue[i];
      if (!set.has(item.path)) continue;
      if (size != null && item.size !== size) continue;
      queue.splice(i, 1);
      settle(key(item.path, item.size));
    }
    // FIL-33: also clear pending/attempt for in-flight (not only queued) paths.
    for (const path of set) {
      settle(key(path, size));
    }
  }

  function takeBatch(n) {
    return queue.splice(0, n);
  }

  function complete(item, dataUrl) {
    const k = key(item.path, item.size);
    if (attemptByKey.get(k) !== item.attempt) return false;
    if (dataUrl) cache.add(k);
    settle(k);
    return true;
  }

  function clear() {
    generation += 1;
    queue.length = 0;
    pending.clear();
    attemptByKey.clear();
  }

  function pathsLeavingWindow(prev, next) {
    const nextSet = new Set(next);
    return [...prev].filter((p) => !nextSet.has(p));
  }

  // Scroll leftovers then prioritize newly visible
  request("a", 64, false);
  request("b", 64, false);
  request("c", 64, false);
  cancel(["a", "b"], 64);
  request("d", 64, true);
  assert.deepEqual(
    queue.map((q) => q.path),
    ["d", "c"],
    "visible priority + OOV cancel",
  );

  clear();
  request("old", 64, false);
  const genBefore = generation;
  clear();
  assert.equal(queue.length, 0);
  assert.ok(generation > genBefore);

  // FIL-33: leave while in-flight (queue empty, pending must clear) then reenter.
  clear();
  request("top", 96, true);
  assert.equal(pending.has(key("top", 96)), true);
  const [inFlight] = takeBatch(1);
  assert.equal(queue.length, 0);
  assert.equal(pending.has(key("top", 96)), true, "pending held for in-flight");
  // Window rebind missed IO leave — explicit path diff cancel (Navigator).
  const left = pathsLeavingWindow(new Set(["top", "mid"]), ["mid", "bottom"]);
  assert.deepEqual(left, ["top"]);
  cancel(left, 96);
  assert.equal(pending.has(key("top", 96)), false, "cancel clears in-flight pending");
  assert.equal(attemptByKey.has(key("top", 96)), false);
  // Stale IPC result must not stick after cancel.
  assert.equal(complete(inFlight, "data:stale"), false);
  assert.equal(cache.has(key("top", 96)), false);
  // Reenter viewport → must schedule again.
  request("top", 96, true);
  assert.equal(pending.has(key("top", 96)), true);
  assert.equal(queue.length, 1);
  assert.equal(queue[0].path, "top");
  const [again] = takeBatch(1);
  assert.ok(complete(again, "data:ok"));
  assert.equal(cache.has(key("top", 96)), true);

  // FIL-50: repeat request while still pending must NOT bump attempt (viewport sync).
  clear();
  cache.clear();
  request("row", 96, true);
  const attemptBefore = attemptByKey.get(key("row", 96));
  const [first] = takeBatch(1);
  request("row", 96, true); // geometry sync repeats same window
  assert.equal(attemptByKey.get(key("row", 96)), attemptBefore, "pending repeat keeps attempt");
  assert.equal(queue.length, 1, "in-flight path re-enqueued for safety");
  assert.ok(complete(first, "data:ok"), "in-flight result still accepted");
  assert.equal(cache.has(key("row", 96)), true);

  // FIL-33: cancel then reenter bumps attempt; stale in-flight ignored.
  clear();
  cache.clear();
  request("row", 96, true);
  const [stale] = takeBatch(1);
  cancel(["row"], 96);
  request("row", 96, true);
  assert.equal(complete(stale, "data:old"), false, "stale in-flight ignored after cancel");
  const [fresh] = takeBatch(1);
  assert.ok(complete(fresh, "data:new"));
  assert.equal(cache.has(key("row", 96)), true);

  // FIL-50: visible window larger than one BATCH_THUMB must stay queued (not capped).
  clear();
  cache.clear();
  const batchThumb = 8;
  const viewportPaths = Array.from({ length: 28 }, (_, i) => `v${i}`);
  requestWindow(viewportPaths, 96, true);
  assert.equal(queue.length, 28, "full viewport queued beyond one thumb batch");
  assert.deepEqual(
    queue.slice(0, 3).map((q) => q.path),
    ["v0", "v1", "v2"],
    "priority window keeps top-to-bottom order (not LIFO)",
  );
  const firstBatch = takeBatch(batchThumb);
  assert.equal(firstBatch.length, batchThumb);
  assert.equal(firstBatch[0].path, "v0", "first screen drains first");
  assert.equal(queue.length, 20, "remaining viewport stays queued for later batches");
  // Geometry sync repeats same window — order must stay stable (no scan push).
  requestWindow(viewportPaths, 96, true);
  assert.deepEqual(
    queue.slice(0, 3).map((q) => q.path),
    ["v0", "v1", "v2"],
    "repeat viewport sync does not reverse queue",
  );
  cancel(
    viewportPaths.filter((_, i) => i >= 20),
    96,
  );
  assert.ok(queue.length >= 12, "cancel only drops paths that left the window");

  // Same screen: folder shell icons drain before video thumbnails.
  clear();
  cache.clear();
  request("video-a", 96, false, true);
  request("video-b", 96, false, true);
  request("folder-1", 96, false, false);
  request("folder-2", 96, false, false);
  const iconFirst = takeNextBatch();
  assert.deepEqual(
    iconFirst.map((q) => q.path),
    ["folder-1", "folder-2"],
    "non-thumbnail (folder) icons drain before thumbnails",
  );
  const thumbNext = takeNextBatch();
  assert.deepEqual(
    thumbNext.map((q) => q.path),
    ["video-a", "video-b"],
  );

  return true;
}

assert.ok(simulate());
console.log("Shell icon queue behavior checks OK (AIA-278 / FIL-33).");
