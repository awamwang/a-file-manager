// SPDX-License-Identifier: GPL-3.0-or-later
// Run via: node --experimental-strip-types scripts/search-palette-checks.ts
// 覆盖：范围归一化/持久化/切换；仅名称匹配；分组顺序与每组上限；
// 类型标签；异步源过期结果丢弃；工具动作配置解析与非法回落；Pack 条目过滤。

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { nextTick } from "vue";
import {
  loadScopes,
  normalizePersistedScopes,
  persistScopes,
  SEARCH_PALETTE_SCOPES_STORAGE_KEY,
  toggleScope,
} from "../src/search/scope.ts";
import { matchByName } from "../src/search/match.ts";
import { useSearchPalette } from "../src/search/useSearchPalette.ts";
import {
  SEARCH_GROUP_LIMIT,
  SEARCH_RESULT_GROUP_ORDER,
  SEARCH_SOURCE_ORDER,
  type SearchSource,
  type SearchSourceId,
  type SearchSourceResult,
  type SearchResultItem,
} from "../src/search/types.ts";
import {
  EMPTY_PALETTE_CONFIG,
  normalizeSearchPaletteConfig,
  paletteActionFor,
} from "../src/search/searchPaletteConfig.ts";
import {
  toFeaturePackSearchEntries,
  type FeaturePackSearchEntryContribution,
} from "../src/state/featurePack/contributions.ts";

// ---------- 1. 范围归一化 / 持久化 / 切换 ----------

assert.equal(normalizePersistedScopes(null), null, "non-array → null");
assert.equal(normalizePersistedScopes("commands"), null, "string → null");
assert.equal(normalizePersistedScopes([]), null, "empty → null");
assert.equal(
  normalizePersistedScopes(["commands", "bogus"]),
  null,
  "unknown id → null",
);
assert.deepEqual(
  normalizePersistedScopes(["tools", "commands", "tools"]),
  ["tools", "commands"],
  "dedupe preserved order",
);
assert.deepEqual(normalizePersistedScopes(["files"]), ["files"]);

class MemoryStorage {
  private map = new Map<string, string>();
  get length(): number {
    return this.map.size;
  }
  getItem(key: string): string | null {
    return this.map.get(key) ?? null;
  }
  setItem(key: string, value: string): void {
    this.map.set(key, value);
  }
  removeItem(key: string): void {
    this.map.delete(key);
  }
  clear(): void {
    this.map.clear();
  }
  key(index: number): string | null {
    return [...this.map.keys()][index] ?? null;
  }
}

assert.equal(loadScopes(undefined), null, "no storage → null");
assert.equal(
  loadScopes(new MemoryStorage()),
  null,
  "missing key → null（按「全部」处理）",
);

const corrupt = new MemoryStorage();
corrupt.setItem(SEARCH_PALETTE_SCOPES_STORAGE_KEY, "{not json");
assert.equal(loadScopes(corrupt), null, "corrupt JSON → null");

const stored = new MemoryStorage();
persistScopes(["workgroups", "workgroups", "projects"], stored);
assert.deepEqual(
  JSON.parse(stored.getItem(SEARCH_PALETTE_SCOPES_STORAGE_KEY) ?? "[]"),
  ["workgroups", "projects"],
  "persist dedupes",
);
assert.deepEqual(loadScopes(stored), ["workgroups", "projects"], "roundtrip");
persistScopes(["files"], undefined); // 无 storage 不抛错

const all: SearchSourceId[] = [...SEARCH_SOURCE_ORDER];
assert.deepEqual(
  toggleScope(["commands", "files"], all, "files"),
  ["commands"],
  "toggle off",
);
assert.deepEqual(
  toggleScope(["commands"], all, "files"),
  ["commands", "files"],
  "toggle on",
);
assert.deepEqual(
  toggleScope(["commands"], all, "commands"),
  all,
  "关掉最后一个源回落「全部」",
);

// ---------- 2. 仅名称匹配（matchByName） ----------

type Named = { name: string; description: string };
const named: Named[] = [
  { name: "Alpha Notes", description: "scratch pad" },
  { name: "Notepad", description: "zzz plain" },
  { name: "Terminal", description: "shell" },
];

assert.deepEqual(matchByName(named, (e) => e.name, "   "), [], "空查询 → []");
assert.deepEqual(
  matchByName(named, (e) => e.name, "zzz"),
  [],
  "描述不参与匹配（zzz 只出现在 description）",
);
const notesHits = matchByName(named, (e) => e.name, "notes");
assert.ok(
  notesHits.some((hit) => hit.entry.name === "Alpha Notes"),
  "按名称命中",
);
assert.ok(
  notesHits.every((hit) => hit.score >= (notesHits[0]?.score ?? 0) - 0),
  "score 存在",
);
assert.ok(
  matchByName(named, (e) => e.name, "notepad").length === 1,
  "精确名称命中",
);

// ---------- 3. 分组顺序 / 每组上限 / 异步过期丢弃 ----------

function fakeItem(
  sourceId: SearchSourceId,
  title: string,
  index: number,
): SearchResultItem {
  return {
    key: `${sourceId}:${index}`,
    sourceId,
    type: "command",
    title,
    activate: () => undefined,
  };
}

function fakeSource(id: SearchSourceId, titles: string[]): SearchSource {
  return {
    id,
    labelKey: `searchPalette.scopes.${id}`,
    enabled: () => true,
    unavailableKey: `searchPalette.status.${id}Unavailable`,
    search(query: string): SearchSourceResult {
      const q = query.trim().toLowerCase();
      return {
        status: "ready",
        items: titles
          .filter((title) => !q || title.toLowerCase().includes(q))
          .map((title, index) => fakeItem(id, title, index)),
      };
    },
  };
}

// 注册乱序：显示顺序仍须固定为 SEARCH_SOURCE_ORDER。
const scrambled: SearchSource[] = [
  fakeSource("extensions", ["ext-1"]),
  fakeSource("commands", Array.from({ length: 12 }, (_, i) => `cmd-${i}`)),
  fakeSource("tools", ["tool-1"]),
  fakeSource("files", ["file-1", "file-2"]),
  fakeSource("workgroups", ["wg-1"]),
  fakeSource("projects", ["proj-1"]),
];

const palette = useSearchPalette(() => scrambled);
palette.reset(); // query 为空 → 立即检索
await nextTick();

assert.deepEqual(
  palette.groups.value.map((group) => group.source.id),
  [...SEARCH_RESULT_GROUP_ORDER],
  "结果分组顺序：命令→项目→工作组→工具→扩展→文件（文件组殿后）",
);
assert.equal(
  palette.groups.value[palette.groups.value.length - 1]?.source.id,
  "files",
  "文件组必须是结果中的最后一组",
);
const commandsGroup = palette.groups.value.find(
  (group) => group.source.id === "commands",
);
assert.ok(commandsGroup, "commands group exists");
assert.equal(
  commandsGroup!.rows.length,
  SEARCH_GROUP_LIMIT,
  `每组上限 ${SEARCH_GROUP_LIMIT}（12 条只显示 8 条）`,
);
assert.deepEqual(
  palette.flat.value.map((row) => row.index),
  palette.flat.value.map((_, index) => index),
  "flat 索引连续",
);
assert.deepEqual(
  palette.scopeChips.value.map((chip) => chip.source.id),
  [...SEARCH_SOURCE_ORDER],
  "chip 顺序（配置面）保持：命令→文件→项目→工作组→工具→扩展",
);
assert.notDeepEqual(
  palette.scopeChips.value.map((chip) => chip.source.id),
  palette.groups.value.map((group) => group.source.id),
  "chip 与结果分组顺序解耦（文件 chip 不在末位）",
);
assert.deepEqual(
  palette.scopeChips.value.map((chip) => chip.index),
  [1, 2, 3, 4, 5, 6],
  "Alt+N 提示从 1 起",
);
assert.ok(
  palette.scopeChips.value.every((chip) => chip.active),
  "未持久化 → 全部生效",
);

// 面板内 Alt+数字：toggleScopeAt 切换第一个 chip。
palette.toggleScopeAt(1);
assert.deepEqual(
  [...palette.activeScopes.value].sort(),
  ["extensions", "files", "projects", "tools", "workgroups"],
  "Alt+1 关掉命令源",
);
palette.toggleScopeAt(1);
assert.deepEqual(
  [...palette.activeScopes.value].sort(),
  [...SEARCH_SOURCE_ORDER].sort(),
  "Alt+1 再开命令源（追加在尾部）",
);

// 异步源过期结果丢弃。
let resolver: ((result: SearchSourceResult) => void) | null = null;
const slowSource: SearchSource = {
  id: "commands",
  labelKey: "searchPalette.scopes.commands",
  enabled: () => true,
  unavailableKey: "searchPalette.status.commandsUnavailable",
  search: () =>
    new Promise<SearchSourceResult>((resolve) => {
      resolver = resolve;
    }),
};
const slowPalette = useSearchPalette(() => [slowSource]);
slowPalette.query.value = "a";
await nextTick();
const staleResolver = resolver!;
slowPalette.query.value = "ab";
await nextTick();
const freshResolver = resolver!;
assert.notEqual(staleResolver, freshResolver, "两次查询各自挂起");

staleResolver({ status: "ready", items: [fakeItem("commands", "stale", 0)] });
await nextTick();
assert.equal(slowPalette.groups.value.length, 1);
assert.equal(
  slowPalette.groups.value[0]?.state.status,
  "loading",
  "过期结果被丢弃，最新查询仍在 loading",
);
assert.equal(
  slowPalette.groups.value[0]?.rows.length,
  0,
  "过期查询的条目不出现",
);

freshResolver({ status: "ready", items: [fakeItem("commands", "fresh", 0)] });
await nextTick();
assert.equal(slowPalette.groups.value.length, 1);
assert.equal(
  slowPalette.groups.value[0]?.rows[0]?.item.title,
  "fresh",
  "最新查询结果生效",
);

// 检索抛错 → 状态行（unavailable），不崩面板。
const throwingSource: SearchSource = {
  id: "projects",
  labelKey: "searchPalette.scopes.projects",
  enabled: () => true,
  unavailableKey: "searchPalette.status.projectsUnavailable",
  search: () => {
    throw new Error("boom");
  },
};
const throwingPalette = useSearchPalette(() => [throwingSource]);
throwingPalette.query.value = "x";
await nextTick();
assert.equal(throwingPalette.groups.value.length, 1);
assert.equal(
  throwingPalette.groups.value[0]?.state.status,
  "unavailable",
  "源抛错 → unavailable 状态行",
);
assert.equal(throwingPalette.groups.value[0]?.rows.length, 0);

// ---------- 4. 类型标签（i18n key 齐全） ----------

const en = JSON.parse(
  readFileSync(
    join(dirname(fileURLToPath(import.meta.url)), "../src/i18n/locales/en.json"),
    "utf8",
  ),
) as Record<string, unknown>;
const searchPalette = en.searchPalette as Record<string, unknown>;

const types = searchPalette.types as Record<string, unknown>;
assert.equal(
  Object.keys(types).length,
  8,
  "8 个行尾类型标签",
);
for (const type of [
  "command",
  "file",
  "folder",
  "project",
  "workgroup",
  "tool",
  "capability",
  "extension",
]) {
  assert.equal(typeof types[type], "string", `types.${type} 缺失`);
}

const scopes = searchPalette.scopes as Record<string, unknown>;
for (const id of SEARCH_SOURCE_ORDER) {
  assert.equal(typeof scopes[id], "string", `scopes.${id} 缺失`);
}
for (const statusKey of [
  "loading",
  "commandsUnavailable",
  "filesUnavailable",
  "openDiscovery",
  "projectsUnavailable",
  "openProjects",
  "workGroupsUnavailable",
  "toolsUnavailable",
  "openToolRuntime",
  "extensionsUnavailable",
]) {
  assert.equal(
    typeof (searchPalette.status as Record<string, unknown>)[statusKey],
    "string",
    `status.${statusKey} 缺失`,
  );
}
assert.equal(typeof searchPalette.viewAllFiles, "string");
for (const noticeKey of [
  "executedCapability",
  "destructiveInRuntime",
  "capabilityFailed",
]) {
  assert.equal(
    typeof searchPalette[noticeKey],
    "string",
    `${noticeKey} 缺失`,
  );
}

// ---------- 5. 工具动作配置解析与非法回落 ----------

assert.deepEqual(normalizeSearchPaletteConfig(null), EMPTY_PALETTE_CONFIG);
assert.deepEqual(
  normalizeSearchPaletteConfig("junk"),
  EMPTY_PALETTE_CONFIG,
  "非对象 → 空配置",
);

const parsed = normalizeSearchPaletteConfig({
  schemaVersion: 2,
  toolActions: {
    "awam.tool-runtime": { enter: "navigate" },
    "tool.b": {
      enter: "callCapability",
      capabilityId: "cap-1",
      arguments: { deep: true },
    },
    "tool.c": { enter: "callCapability" }, // 缺 capabilityId → 丢弃
    "tool.d": { enter: "explode" }, // 未知 enter → 回落 navigate
  },
});
assert.equal(parsed.schemaVersion, 2);
assert.equal(
  paletteActionFor(parsed, "tool.missing").enter,
  "navigate",
  "未配置工具 → navigate",
);
assert.deepEqual(paletteActionFor(parsed, "awam.tool-runtime"), {
  enter: "navigate",
});
assert.deepEqual(paletteActionFor(parsed, "tool.b"), {
  enter: "callCapability",
  capabilityId: "cap-1",
  arguments: { deep: true },
});
assert.equal(
  paletteActionFor(parsed, "tool.c").enter,
  "navigate",
  "非法 callCapability 回落 navigate",
);
assert.deepEqual(paletteActionFor(parsed, "tool.d"), { enter: "navigate" });

const noArgs = normalizeSearchPaletteConfig({
  toolActions: { t: { enter: "callCapability", capabilityId: "x" } },
});
assert.deepEqual(paletteActionFor(noArgs, "t"), {
  enter: "callCapability",
  capabilityId: "x",
  arguments: undefined,
});
assert.deepEqual(
  normalizeSearchPaletteConfig({ schemaVersion: "3", toolActions: [] }),
  EMPTY_PALETTE_CONFIG,
  "schemaVersion 非数字 → 1；toolActions 数组 → 忽略",
);

// ---------- 6. Pack 条目过滤 ----------

const packEntries: FeaturePackSearchEntryContribution[] = [
  {
    id: "entry-a",
    labelKey: "packA.search.entryA",
    keywords: ["alpha"],
    route: "/ai",
    packId: "pack.a",
    packName: "Pack A",
  },
  { id: "", labelKey: "x", keywords: [], route: "/ai" }, // 缺 id
  { id: "b", labelKey: "", keywords: [], route: "/ai" }, // 缺 labelKey
  { id: "c", labelKey: "x", keywords: [], route: "" }, // 缺 route
  { id: "d", labelKey: "x", keywords: [], route: "/unknown" }, // 不可解析路由
  {
    id: "entry-e",
    labelKey: "packB.search.entryE",
    keywords: [],
    route: "/tools",
  }, // packId/packName 缺省
];
const filtered = toFeaturePackSearchEntries(
  packEntries,
  (route) => route === "/ai" || route === "/tools",
);
assert.deepEqual(
  filtered.map((entry) => entry.id),
  ["entry-a", "entry-e"],
  "缺字段 / 未知路由被过滤",
);
assert.deepEqual(filtered[0], {
  id: "entry-a",
  labelKey: "packA.search.entryA",
  keywords: ["alpha"],
  route: "/ai",
  packId: "pack.a",
  packName: "Pack A",
});
assert.deepEqual(filtered[1], {
  id: "entry-e",
  labelKey: "packB.search.entryE",
  keywords: [],
  route: "/tools",
  packId: "",
  packName: "",
});

// keywords 非数组 → 空数组兜底。
const badKeywords = toFeaturePackSearchEntries(
  [
    {
      id: "k",
      labelKey: "k",
      keywords: "oops" as unknown as string[],
      route: "/ai",
    },
  ],
  () => true,
);
assert.deepEqual(badKeywords[0]?.keywords, []);

console.log(
  "Search palette checks OK (scopes / name-match / groups / stale-discard / config / pack filter).",
);
