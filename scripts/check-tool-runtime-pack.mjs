// SPDX-License-Identifier: GPL-3.0-or-later
/** 工具管理运行时 Pack：清单 id、builtins、Core 与 AI Add-on 术语边界。 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const pack = JSON.parse(read("packs/tool-runtime/pack.json"));
const builtinEntries = read("src/state/featurePack/builtinEntries.ts");
const configRs = read("src-tauri/src/tool_runtime/config.rs");
const sideNav = read("src/shell/sideNav.ts");
const router = read("src/router/index.ts");
const registry = read("src/commands/registry.ts");
const main = read("src/main.ts");
const aiIndex = read("src/ai/index.ts");
const rustAi = read("src-tauri/src/ai/mod.rs");

const PACK_ID = "awam.tool-runtime";
const BUILTIN_ENTRY = "builtin:tool-runtime";

assert.equal(pack.id, PACK_ID, "pack.json id must be awam.tool-runtime");
assert.ok(
  configRs.includes(`TOOL_RUNTIME_PACK_ID: &str = "${PACK_ID}"`),
  "Rust TOOL_RUNTIME_PACK_ID must match pack.json",
);

assert.ok(
  builtinEntries.includes(`"${BUILTIN_ENTRY}"`),
  "builtinEntries must register builtin:tool-runtime",
);
assert.ok(
  builtinEntries.includes("packs/tool-runtime/ToolRuntimeView.vue"),
  "builtin entry must lazy-load ToolRuntimeView",
);

const routes = pack.contributions?.routes ?? [];
assert.ok(
  routes.some((r) => r.path === "/tools" && r.entry === BUILTIN_ENTRY),
  "pack.json must contribute /tools with builtin:tool-runtime",
);

for (const [label, src] of [
  ["sideNav.ts", sideNav],
  ["router/index.ts", router],
  ["commands/registry.ts", registry],
  ["main.ts", main],
]) {
  assert.ok(!src.includes("packs/tool-runtime"), `${label} must not import Pack UI directly`);
  assert.ok(!src.includes("/tools"), `${label} must not hard-code /tools (Pack contribution)`);
}

assert.ok(!rustAi.includes("tool_runtime"), "AI Add-on Rust must not reference tool_runtime");
assert.ok(
  !aiIndex.includes("tool-runtime") && !aiIndex.includes("tool_runtime"),
  "src/ai must not reference tool-runtime Pack",
);

assert.ok(
  !sideNav.includes("awam-tool-manager") && !router.includes("awam-tool-manager"),
  "Core shell must not embed awam-tool-manager sidecar wiring",
);

console.log("tool-runtime-pack: ok");
