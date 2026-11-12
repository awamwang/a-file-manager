// SPDX-License-Identifier: GPL-3.0-or-later
/** 构建/运行期依赖表面：声明、WebView2 安装策略、健康页探测接线。 */
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const pkg = JSON.parse(read("package.json"));
assert.ok(pkg.engines?.node, "package.json engines.node required");
assert.match(String(pkg.engines.node), /20/, "engines.node should require Node 20+");

assert.ok(existsSync(join(root, "rust-toolchain.toml")), "rust-toolchain.toml required");

const tauri = JSON.parse(read("src-tauri/tauri.conf.json"));
assert.equal(
  tauri.bundle?.windows?.webviewInstallMode?.type,
  "embedBootstrapper",
  "NSIS must embed WebView2 bootstrapper for silent auto-install",
);

const lib = read("src-tauri/src/lib.rs");
assert.ok(lib.includes("mod deps_probe"), "deps_probe module wired");
assert.ok(lib.includes("deps_probe::deps_probe_get"), "deps_probe_get command registered");

const health = read("src/views/HealthView.vue");
assert.ok(health.includes('invoke<DependencyReport>("deps_probe_get")'), "HealthView loads deps probe");
assert.ok(health.includes("data-health-deps"), "HealthView renders deps list");

const knowledge = read("src-tauri/src/providers/knowledge.rs");
assert.ok(
  !knowledge.includes(String.raw`G:\Knowledge\data`),
  "knowledge must not hardcode G:\\Knowledge\\data",
);

const mcp = read("src-tauri/src/ai/mcp.rs");
assert.ok(
  !mcp.includes("G:\\\\Projects\\\\19AI"),
  "MCP seed must not embed local absolute paths",
);
assert.ok(mcp.includes("const SEED_MCP_CATALOG: &[SeedMcpDef] = &[]"), "public MCP seed empty");

const en = JSON.parse(read("src/i18n/locales/en.json"));
assert.ok(en.health?.deps?.title, "en health.deps i18n");
assert.ok(en.health.deps.items?.webview2?.title, "en webview2 title");

console.log("Dependency surface OK (engines, toolchain, WebView2 embed, Health probe, no local seeds).");
