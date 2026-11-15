// SPDX-License-Identifier: GPL-3.0-or-later
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFileSync(join(root, path), "utf8");

const commandPanel = read("src/components/command/CommandPanel.vue");
const scope = read("src/search/scope.ts");
const palette = read("src/search/useSearchPalette.ts");
const registry = read("src/search/registry.ts");
const types = read("src/search/types.ts");
const config = read("src/search/searchPaletteConfig.ts");
const tools = read("src/search/sources/tools.ts");
const extensions = read("src/search/sources/extensions.ts");
const files = read("src/search/sources/files.ts");
const libRs = read("src-tauri/src/lib.rs");
const paletteRs = read("src-tauri/src/palette/mod.rs");

for (const [label, text, needle] of [
  ["CommandPanel", commandPanel, "buildSearchSources("],
  ["CommandPanel", commandPanel, "useSearchPalette("],
  ["CommandPanel", commandPanel, "toggleScopeAt("],
  ["CommandPanel", commandPanel, "searchPalette.types."],
  ["scope", scope, "afm.searchPaletteScopes"],
  ["useSearchPalette", palette, "seq !== searchSeq"],
  ["useSearchPalette", palette, "SEARCH_GROUP_LIMIT"],
  ["types", types, "export const SEARCH_SOURCE_ORDER"],
  ["types", types, "export const SEARCH_GROUP_LIMIT = 8;"],
  ["searchPaletteConfig", config, "palette_get_config"],
  ["tools source", tools, "allowDestructive: false"],
  ["tools source", tools, "searchPalette.destructiveInRuntime"],
  ["extensions source", extensions, "getSearchExtras"],
  ["files source", files, "searchPalette.viewAllFiles"],
  ["files source", files, "query: q ? { q } : {}"],
  ["lib.rs", libRs, "palette::palette_get_config,"],
  ["palette/mod.rs", paletteRs, "pub async fn palette_get_config"],
]) {
  if (!text.includes(needle)) {
    console.error(`${label} missing ${needle}`);
    process.exit(1);
  }
}

// 固定分组顺序：注册表里六个源的构造调用必须按
// 命令→文件→项目→工作组→工具→扩展 出现。
const registryOrder = [
  "createCommandsSource(",
  "createFilesSource(",
  "createProjectsSource(",
  "createWorkgroupsSource(",
  "createToolsSource(",
  "createExtensionsSource(",
]
  .map((needle) => registry.indexOf(needle))
  .filter((index) => index >= 0);
if (registryOrder.length !== 6 || registryOrder.some((v, i) => i && v < registryOrder[i - 1])) {
  console.error("registry.ts source order must be commands→files→projects→workgroups→tools→extensions");
  process.exit(1);
}

const result = spawnSync(
  process.execPath,
  ["--experimental-strip-types", join(root, "scripts/search-palette-checks.ts")],
  { cwd: root, encoding: "utf8" },
);
if (result.stdout) process.stdout.write(result.stdout);
if (result.stderr) process.stderr.write(result.stderr);
if (result.status !== 0) process.exit(result.status ?? 1);
