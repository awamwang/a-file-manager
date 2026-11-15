// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 工具源：toolRuntimeIndex 平铺工具与能力；仅匹配工具名 / 能力名（描述不参与）。
 * - Enter：跳工具运行时并预选（?tool=…&capability=…）；
 *   可按 `command-palette.json` 覆盖为 callCapability
 *   （固定 execute: true、allowDestructive: false；破坏性只提示并引导进运行时）。
 * - 网关离线：状态行 + 打开工具运行时；Pack 未启用：源与 chip 一起隐藏。
 */

import type { Router } from "vue-router";
import {
  callCapability,
  invokeErrorMessage,
  isDestructiveBlocked,
} from "../../packs/tool-runtime/client.ts";
import { toolRuntimeRouteContribution } from "../../state/featurePack/contributions.ts";
import type { ToolIndexEntry } from "../../state/toolRuntimeIndex.ts";
import { loadToolIndex } from "../../state/toolRuntimeIndex.ts";
import type {
  SearchActivateOutcome,
  SearchQueryOptions,
  SearchResultItem,
  SearchSource,
  SearchSourceResult,
} from "../types.ts";
import { matchText } from "../match.ts";
import {
  loadSearchPaletteConfig,
  paletteActionFor,
  type PaletteToolAction,
} from "../searchPaletteConfig.ts";

export function createToolsSource(router: Router): SearchSource {
  /** 跳工具运行时并预选；Pack 未启用时无贡献即忽略。 */
  function pushToolRuntime(query: Record<string, string>): void {
    const route = toolRuntimeRouteContribution.value;
    if (!route) return;
    void router.push({ name: route.name, query });
  }

  /** 执行 command-palette.json 配置的 callCapability（固定非破坏性执行）。 */
  async function runConfiguredCapability(
    entry: ToolIndexEntry,
    action: Extract<PaletteToolAction, { enter: "callCapability" }>,
  ): Promise<SearchActivateOutcome> {
    const capability = entry.capabilities.find(
      (item) => item.capabilityId === action.capabilityId,
    );
    if (!capability) {
      // 配置的能力不存在：回落跳转预选该工具。
      pushToolRuntime({ tool: entry.tool.id });
      return;
    }
    try {
      await callCapability({
        toolId: entry.tool.id,
        capabilityId: action.capabilityId,
        arguments: action.arguments ?? {},
        execute: true,
        allowDestructive: false,
      });
      return { toastKey: "searchPalette.executedCapability", kind: "info" };
    } catch (err) {
      if (isDestructiveBlocked(invokeErrorMessage(err))) {
        // 破坏性能力：不在面板内执行，引导进工具运行时人工确认。
        pushToolRuntime({
          tool: entry.tool.id,
          capability: action.capabilityId,
        });
        return { toastKey: "searchPalette.destructiveInRuntime", kind: "info" };
      }
      pushToolRuntime({
        tool: entry.tool.id,
        capability: action.capabilityId,
      });
      return { toastKey: "searchPalette.capabilityFailed", kind: "error" };
    }
  }

  function toolItem(entry: ToolIndexEntry): SearchResultItem {
    return {
      key: `tools:${entry.tool.id}`,
      sourceId: "tools",
      type: "tool",
      title: entry.tool.name,
      subtitle: entry.tool.kind,
      activate: async (): Promise<SearchActivateOutcome> => {
        const action = paletteActionFor(
          await loadSearchPaletteConfig(),
          entry.tool.id,
        );
        if (action.enter === "callCapability") {
          return runConfiguredCapability(entry, action);
        }
        pushToolRuntime({ tool: entry.tool.id });
      },
    };
  }

  function capabilityItem(
    entry: ToolIndexEntry,
    capabilityIndex: number,
  ): SearchResultItem {
    const capability = entry.capabilities[capabilityIndex];
    return {
      key: `tools:${entry.tool.id}:${capability.capabilityId}`,
      sourceId: "tools",
      type: "capability",
      title: capability.name,
      // 能力副标题显示所属工具。
      subtitle: entry.tool.name,
      activate: () => {
        pushToolRuntime({
          tool: entry.tool.id,
          capability: capability.capabilityId,
        });
      },
    };
  }

  return {
    id: "tools",
    labelKey: "searchPalette.scopes.tools",
    enabled: () => toolRuntimeRouteContribution.value !== null,
    unavailableKey: "searchPalette.status.toolsUnavailable",
    unavailableActionLabelKey: "searchPalette.status.openToolRuntime",
    unavailableAction: () => {
      pushToolRuntime({});
    },
    async search(
      query: string,
      options?: SearchQueryOptions,
    ): Promise<SearchSourceResult> {
      const q = query.trim();
      if (!q) return { status: "ready", items: [] };
      const fuzzy = options?.fuzzy ?? false;
      const entries = await loadToolIndex();
      if (!entries) {
        // 网关离线 / 列取失败：状态行。
        return { status: "unavailable", items: [] };
      }
      const items: SearchResultItem[] = [];
      for (const entry of entries) {
        if (matchText(entry.tool.name, q, fuzzy)) {
          items.push(toolItem(entry));
        }
        for (let index = 0; index < entry.capabilities.length; index += 1) {
          if (matchText(entry.capabilities[index].name, q, fuzzy)) {
            items.push(capabilityItem(entry, index));
          }
        }
      }
      return { status: "ready", items };
    },
  };
}
