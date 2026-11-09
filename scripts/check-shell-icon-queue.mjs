// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AIA-278: unit-ish checks for thumbnail queue cancel / priority without Tauri.
 * Uses dynamic import of the compiled-TS via vite-node? — keep as source contract +
 * lightweight in-process simulation mirrored from shellIcons rules.
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

// --- Contract: Navigator wires folder / scroll cancel ---
assert.ok(/watch\(currentPath/.test(view) && view.includes("clearShellIconQueue()"));
assert.ok(view.includes("cancelShellIconRequests(leave"));
assert.ok(/requestIconsFor\(enter,\s*true\)/.test(view));
assert.ok(view.includes("scheduleViewportIconObservers"));

// --- Lightweight queue behavior model (mirrors shellIcons.ts rules) ---
function simulate() {
  /** @type {{ path: string, size: number }[]} */
  const queue = [];
  const pending = new Set();
  let generation = 0;

  function key(path, size) {
    return `${size}|${path}`;
  }

  function enqueue(path, size, priority) {
    const k = key(path, size);
    const idx = queue.findIndex((q) => key(q.path, q.size) === k);
    if (idx >= 0) {
      if (priority && idx > 0) {
        const [row] = queue.splice(idx, 1);
        queue.unshift(row);
      }
      return;
    }
    if (pending.has(k)) return;
    pending.add(k);
    const item = { path, size };
    if (priority) queue.unshift(item);
    else queue.push(item);
  }

  function cancel(paths, size) {
    const set = new Set(paths);
    for (let i = queue.length - 1; i >= 0; i -= 1) {
      const item = queue[i];
      if (!set.has(item.path)) continue;
      if (size != null && item.size !== size) continue;
      queue.splice(i, 1);
      pending.delete(key(item.path, item.size));
    }
  }

  function clear() {
    generation += 1;
    queue.length = 0;
    pending.clear();
  }

  // Scroll leftovers then prioritize newly visible
  enqueue("a", 64, false);
  enqueue("b", 64, false);
  enqueue("c", 64, false);
  cancel(["a", "b"], 64);
  enqueue("d", 64, true);
  assert.deepEqual(
    queue.map((q) => q.path),
    ["d", "c"],
    "visible priority + OOV cancel",
  );

  clear();
  enqueue("old", 64, false);
  const genBefore = generation;
  clear();
  assert.equal(queue.length, 0);
  assert.ok(generation > genBefore);

  return true;
}

assert.ok(simulate());
console.log("Shell icon queue behavior checks OK (AIA-278).");
