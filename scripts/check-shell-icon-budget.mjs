// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * AIA-279: LRU / byte-budget eviction for shellIcons without Tauri.
 * Mirrors the cache rules in src/state/shellIcons.ts.
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const state = readFileSync(join(root, "src/state/shellIcons.ts"), "utf8");
const rust = readFileSync(join(root, "src-tauri/src/shell_icons/mod.rs"), "utf8");
const pkg = readFileSync(join(root, "package.json"), "utf8");

// --- Contract: hard caps + eviction API ---
assert.ok(/SHELL_ICON_MAX_ENTRIES\s*=\s*512/.test(state));
assert.ok(/SHELL_ICON_MAX_BYTES\s*=\s*48\s*\*\s*1024\s*\*\s*1024/.test(state));
assert.ok(/function trimToBudget/.test(state));
assert.ok(/function pickVictim/.test(state));
assert.ok(/meta\.soft/.test(state) || /soft: true/.test(state));
assert.ok(/revokeObjectURL/.test(state), "eviction must revoke blob: URLs when present");
assert.ok(/shellIconCacheStatsForTests/.test(state));
assert.ok(/shellIconPutForTests/.test(state));
assert.ok(/requestAnimationFrame/.test(state));
assert.ok(/BATCH_THUMB\s*=\s*8/.test(state));

assert.ok(/MAX_ICON_CACHE_ENTRIES:\s*usize\s*=\s*384/.test(rust));
assert.ok(/struct BoundedIconCache/.test(rust));
assert.ok(/bounded_icon_cache_evicts_oldest/.test(rust));
assert.ok(pkg.includes("test:shell-icon-budget"));

// --- Behavior model (same rules as shellIcons.ts) ---
const MAX_ENTRIES = 512;
const MAX_BYTES = 48 * 1024 * 1024;

function simulate() {
  /** @type {Map<string, true>} */
  const lru = new Map();
  /** @type {Map<string, { bytes: number, soft: boolean }>} */
  const meta = new Map();
  /** @type {Record<string, string>} */
  const store = {};
  let liveBytes = 0;

  function drop(key) {
    const m = meta.get(key);
    if (m) liveBytes = Math.max(0, liveBytes - m.bytes);
    meta.delete(key);
    lru.delete(key);
    delete store[key];
  }

  function pickVictim() {
    for (const [key] of lru) {
      if (meta.get(key)?.soft) return key;
    }
    const first = lru.keys().next();
    return first.done ? null : first.value;
  }

  function trim() {
    while (lru.size > MAX_ENTRIES || liveBytes > MAX_BYTES) {
      const v = pickVictim();
      if (!v) break;
      drop(v);
    }
  }

  function put(key, dataUrl, soft = false) {
    if (meta.has(key)) drop(key);
    const bytes = dataUrl.length;
    store[key] = dataUrl;
    meta.set(key, { bytes, soft });
    lru.set(key, true);
    liveBytes += bytes;
    trim();
  }

  function touch(key) {
    if (!lru.has(key)) return;
    lru.delete(key);
    lru.set(key, true);
    const m = meta.get(key);
    if (m) m.soft = false;
  }

  function markSoft(key) {
    const m = meta.get(key);
    if (m) m.soft = true;
  }

  // Entry cap: fill past MAX_ENTRIES
  for (let i = 0; i < MAX_ENTRIES + 20; i += 1) {
    put(`20|/e/${i}`, "x");
  }
  assert.equal(lru.size, MAX_ENTRIES);
  assert.equal(store["20|/e/0"], undefined, "oldest entry evicted");
  assert.ok(store[`20|/e/${MAX_ENTRIES + 19}`]);

  // Soft preferred over hard LRU
  for (const key of [...lru.keys()]) drop(key);
  put("64|/a", "aaaa");
  put("64|/b", "bbbb");
  put("64|/c", "cccc");
  markSoft("64|/b");
  touch("64|/a"); // a becomes newest
  // Force one eviction via entry cap by shrinking conceptually:
  // simulate over-entry by temporarily treating max as 2
  while (lru.size > 2) {
    const v = pickVictim();
    assert.equal(v, "64|/b", "soft victim before older hard");
    drop(v);
  }
  assert.equal(store["64|/b"], undefined);
  assert.ok(store["64|/a"]);
  assert.ok(store["64|/c"]);

  // Byte budget: one huge entry must push others out
  for (const key of [...lru.keys()]) drop(key);
  put("96|/small1", "s".repeat(1000));
  put("96|/small2", "s".repeat(1000));
  put("96|/huge", "H".repeat(MAX_BYTES));
  assert.ok(liveBytes <= MAX_BYTES || lru.size === 1);
  assert.ok(store["96|/huge"], "newest huge entry retained");
  assert.equal(store["96|/small1"], undefined);
  assert.equal(store["96|/small2"], undefined);

  return true;
}

assert.ok(simulate());
console.log("Shell icon budget / LRU checks OK (AIA-279).");
