// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 工具运行时索引：先探活网关，再 `list_tools`，并发拉取能力。
 * 5 分钟 TTL、最多 50 个工具、单工具能力拉取失败忽略（工具保留、能力为空）。
 * 网关不可达返回 null（搜索面板出状态行），且不缓存失败结果。
 */

import {
  gatewayStatus,
  listCapabilities,
  listTools,
  parseCapabilityList,
  parseToolList,
  type RuntimeCapability,
  type RuntimeTool,
} from "../packs/tool-runtime/client.ts";

export type ToolIndexEntry = {
  tool: RuntimeTool;
  capabilities: RuntimeCapability[];
};

const TTL_MS = 5 * 60_000;
const MAX_TOOLS = 50;
const CAPABILITY_CONCURRENCY = 4;

let cachedAt = 0;
let cachedEntries: ToolIndexEntry[] | null = null;
let inflight: Promise<ToolIndexEntry[] | null> | null = null;

/**
 * 读取工具索引（含能力）。null = 网关不可达 / 列取失败。
 * 成功结果缓存 5 分钟；失败不缓存，下次调用重试。
 */
export function loadToolIndex(): Promise<ToolIndexEntry[] | null> {
  if (cachedEntries && Date.now() - cachedAt < TTL_MS) {
    return Promise.resolve(cachedEntries);
  }
  if (!inflight) {
    const pending = buildIndex();
    inflight = pending;
    void pending
      .then(
        (entries) => {
          if (entries) {
            cachedEntries = entries;
            cachedAt = Date.now();
          }
        },
        () => {
          // buildIndex 内部已兜底；此处仅防意外 reject。
        },
      )
      .finally(() => {
        if (inflight === pending) inflight = null;
      });
  }
  return inflight;
}

/** 清空缓存（测试 / 手动刷新用）。 */
export function resetToolIndexCache(): void {
  cachedEntries = null;
  cachedAt = 0;
}

async function buildIndex(): Promise<ToolIndexEntry[] | null> {
  // 1. 探活：不可达 → null（状态行），不缓存。
  try {
    const status = await gatewayStatus();
    if (!status.reachable) return null;
  } catch {
    return null;
  }
  // 2. 工具列表（上限 50）。
  let tools: RuntimeTool[];
  try {
    tools = parseToolList(await listTools()).slice(0, MAX_TOOLS);
  } catch {
    return null;
  }
  // 3. 并发拉取能力；单工具失败忽略。
  const entries: ToolIndexEntry[] = tools.map((tool) => ({
    tool,
    capabilities: [],
  }));
  let cursor = 0;
  const pullWorker = async (): Promise<void> => {
    while (cursor < entries.length) {
      const index = cursor++;
      try {
        entries[index].capabilities = parseCapabilityList(
          await listCapabilities(entries[index].tool.id, false),
        );
      } catch {
        // 单工具能力拉取失败：保留工具、能力留空。
      }
    }
  };
  const workerCount = Math.min(CAPABILITY_CONCURRENCY, entries.length);
  await Promise.all(
    Array.from({ length: workerCount }, () => pullWorker()),
  );
  return entries;
}
