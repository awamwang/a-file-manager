// SPDX-License-Identifier: GPL-3.0-or-later
/** 命令源：标题 + 关键词匹配（group 不参与），Enter 执行。 */

import { fuzzyScore } from "../../commands/fuzzyFilter.ts";
import type { AppCommand } from "../../commands/types.ts";
import type {
  SearchSource,
  SearchSourceResult,
  SearchResultItem,
} from "../types.ts";

const COMMAND_GROUP_LABEL_KEY: Record<string, string> = {
  Navigate: "commands.groups.navigate",
  View: "commands.groups.view",
  Actions: "commands.groups.actions",
};

/** 标题 + 关键词打分；group 不参与匹配。 */
function scoreCommand(command: AppCommand, query: string): number | null {
  const title = fuzzyScore(query, command.title);
  const haystack = fuzzyScore(
    query,
    [command.title, ...(command.keywords ?? [])].join(" "),
  );
  if (title === null && haystack === null) return null;
  return Math.max(
    title ?? Number.NEGATIVE_INFINITY,
    haystack ?? Number.NEGATIVE_INFINITY,
  );
}

export function createCommandsSource(
  commands: () => readonly AppCommand[],
): SearchSource {
  return {
    id: "commands",
    labelKey: "searchPalette.scopes.commands",
    enabled: () => true,
    unavailableKey: "searchPalette.status.commandsUnavailable",
    search(query: string): SearchSourceResult {
      const q = query.trim();
      const items: SearchResultItem[] = [];
      if (q) {
        const matches: Array<{ command: AppCommand; score: number }> = [];
        for (const command of commands()) {
          const score = scoreCommand(command, q);
          if (score !== null) matches.push({ command, score });
        }
        matches.sort(
          (a, b) =>
            b.score - a.score ||
            a.command.title.localeCompare(b.command.title),
        );
        for (const { command } of matches) items.push(toItem(command));
      } else {
        for (const command of commands()) items.push(toItem(command));
      }
      return { status: "ready", items };
    },
  };
}

function toItem(command: AppCommand): SearchResultItem {
  return {
    key: `commands:${command.id}`,
    sourceId: "commands",
    type: "command",
    title: command.title,
    subtitleKey: COMMAND_GROUP_LABEL_KEY[command.group],
    group: command.group,
    activate: () => command.run(),
  };
}
