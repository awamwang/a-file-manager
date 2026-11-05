// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const app = readFileSync(join(root, "src/App.vue"), "utf8");
const state = readFileSync(join(root, "src/state/providerCache.ts"), "utf8");
const providersMod = readFileSync(join(root, "src-tauri/src/providers/mod.rs"), "utf8");
const cache = readFileSync(join(root, "src-tauri/src/providers/cache.rs"), "utf8");
const lib = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const providersView = readFileSync(join(root, "src/views/ProvidersView.vue"), "utf8");

assert.ok(state.includes("provider_preload"), "frontend must invoke provider_preload");
assert.ok(
  state.includes("idle after app start") || state.includes("scheduleIdle"),
  "must document idle-after-start wire choice (AIA-181)",
);
assert.ok(app.includes("preloadProviderCache"), "App.vue must schedule cache warm");
assert.ok(app.includes("scheduleIdle"), "App.vue must defer preload via scheduleIdle");
assert.ok(
  app.includes("AIA-181") || app.includes("navigator_list"),
  "App.vue must document deferred preload rationale",
);

assert.ok(cache.includes("TTL_OK"), "cache must define ok TTL");
assert.ok(cache.includes("TTL_ERR"), "cache must define error TTL");
assert.ok(cache.includes("depth1_cache_key"), "depth-1 key helper required");
assert.ok(providersMod.includes("provider_preload"), "preload command required");
assert.ok(providersMod.includes("provider_cache_invalidate"), "invalidate command required");
assert.ok(providersMod.includes("interface_only"), "preload must skip interface-only");
assert.ok(lib.includes("providers::provider_preload"));
assert.ok(lib.includes("providers::provider_cache_invalidate"));

// Providers route uses provider_list (cache hit) and may re-warm if idle preload deferred.
assert.ok(providersView.includes("provider_list"));
assert.ok(
  providersView.includes("preloadProviderCache"),
  "ProvidersView must warm on first visit when idle preload has not finished",
);

const cargo = spawnSync(
  "cargo",
  ["test", "-p", "a-file-manager", "--lib", "providers::", "--", "--nocapture"],
  { cwd: join(root, "src-tauri"), encoding: "utf8" },
);
if (cargo.stdout) process.stdout.write(cargo.stdout);
if (cargo.stderr) process.stderr.write(cargo.stderr);
assert.equal(cargo.status, 0, "provider cache rust tests failed");

console.log("Provider depth-1 cache checks OK (AIA-76 / AIA-181).");
