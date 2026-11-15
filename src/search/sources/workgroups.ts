// SPDX-License-Identifier: GPL-3.0-or-later
/** 工作组源：仅匹配名称，Enter 静默切换；REQUIRE_SAVE_PROMPT 关面板 + toast。 */

import {
  activeWorkGroupId,
  hydrateWorkGroups,
  type UserWorkGroupsDoc,
} from "../../state/openState/index.ts";
import {
  silentSwitchWorkGroup,
  workGroupChromeError,
} from "../../state/workGroup/chrome.ts";
import { matchByName } from "../match.ts";
import type {
  SearchSource,
  SearchSourceResult,
  SearchResultItem,
} from "../types.ts";

export function createWorkgroupsSource(): SearchSource {
  return {
    id: "workgroups",
    labelKey: "searchPalette.scopes.workgroups",
    enabled: () => true,
    unavailableKey: "searchPalette.status.workGroupsUnavailable",
    async search(query: string): Promise<SearchSourceResult> {
      const q = query.trim();
      if (!q) return { status: "ready", items: [] };
      let doc: UserWorkGroupsDoc;
      try {
        doc = await hydrateWorkGroups();
      } catch {
        return { status: "unavailable", items: [] };
      }
      const items: SearchResultItem[] = matchByName(
        doc.workGroups,
        (group) => group.name,
        q,
      ).map(({ entry }) => ({
        key: `workgroups:${entry.id}`,
        sourceId: "workgroups",
        type: "workgroup",
        title: entry.name,
        subtitleKey: "searchPalette.workGroupMembers",
        subtitleParams: { count: entry.memberPaths.length },
        activate: async () => {
          if (entry.id === activeWorkGroupId.value) return;
          const result = await silentSwitchWorkGroup(entry.id);
          if (
            result?.requireSavePrompt ||
            workGroupChromeError.value === "REQUIRE_SAVE_PROMPT"
          ) {
            return {
              toastKey: "workGroups.errorSavePrompt",
              kind: "error" as const,
            };
          }
          return undefined;
        },
      }));
      return { status: "ready", items };
    },
  };
}
