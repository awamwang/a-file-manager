// SPDX-License-Identifier: GPL-3.0-or-later
/** FIL-81: 设置→健康检查不得阻塞 UI；deps 探测须 async+spawn_blocking；离开页作废结果。 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const depsRs = read("src-tauri/src/deps_probe.rs");
const health = read("src/views/HealthView.vue");
const settings = read("src/views/SettingsView.vue");

// --- backend: 慢探测离开 async runtime ---
assert.match(
  depsRs,
  /#\[tauri::command\][\s\S]*?pub async fn deps_probe_get[\s\S]*?spawn_blocking/,
  "deps_probe_get must be async and use spawn_blocking",
);
assert.ok(
  depsRs.includes("std::thread::scope"),
  "independent probes should run in parallel (thread::scope)",
);

// --- frontend: 壳立刻可交互，骨架/轻量 hint，非整页等结果 ---
assert.ok(health.includes("onUnmounted"), "HealthView must invalidate on leave");
assert.ok(health.includes("loadEpoch"), "stale probe results must be superseded by epoch");
assert.ok(
  health.includes("data-health-deps-skeleton") || health.includes("data-health-skeleton"),
  "dependency/identity loading must show skeleton placeholders",
);
assert.ok(health.includes("data-health-loading"), "lightweight loading hint required");
assert.ok(health.includes("aria-busy"), "health panel exposes aria-busy while probing");
assert.ok(
  health.includes("healthLoading") && health.includes("depsLoading"),
  "identity and deps load independently for progressive paint",
);
assert.ok(
  health.includes('class="deps-heading"') || health.includes("deps-heading"),
  "deps section heading must be in the template for immediate paint",
);
assert.ok(
  !/<p v-if="loading"[\s\S]*?<\/p>\s*<p v-else-if="error"[\s\S]*?<\/p>\s*<template v-else>/.test(
    health,
  ),
  "must not hide the whole health body behind loading/error/v-else",
);

// --- Settings 嵌入槽：切走即卸载 HealthView（触发 onUnmounted 作废）---
assert.ok(
  settings.includes('v-if="activeSection === \'health\'"') ||
    settings.includes('v-if="activeSection === \"health\""'),
  "Settings health slot must unmount when leaving the section",
);
assert.ok(settings.includes("<HealthView"), "Settings embeds HealthView");

console.log("health non-blocking checks OK (FIL-81)");
