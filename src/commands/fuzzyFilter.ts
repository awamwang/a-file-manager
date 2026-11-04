// SPDX-License-Identifier: GPL-3.0-or-later

import type { AppCommand, RankedCommand } from "./types";

/**
 * Case-insensitive subsequence fuzzy score.
 * Higher is better. Returns null when the query does not match.
 */
export function fuzzyScore(query: string, text: string): number | null {
  const q = query.trim().toLowerCase();
  if (!q) return 0;

  const t = text.toLowerCase();
  let ti = 0;
  let score = 0;
  let consecutive = 0;
  let firstIndex = -1;

  for (let qi = 0; qi < q.length; qi++) {
    const ch = q[qi];
    const found = t.indexOf(ch, ti);
    if (found === -1) return null;
    if (firstIndex === -1) firstIndex = found;
    if (found === ti) {
      consecutive += 1;
      score += 8 + consecutive * 2;
    } else {
      consecutive = 0;
      score += 2;
    }
    // Prefer earlier matches and prefix starts.
    score += Math.max(0, 24 - found);
    ti = found + 1;
  }

  if (t.startsWith(q)) score += 40;
  if (t.includes(q)) score += 20;
  score -= firstIndex;
  return score;
}

function commandHaystack(command: AppCommand): string {
  const parts = [command.title, command.group, ...(command.keywords ?? [])];
  return parts.join(" ");
}

/** Rank and filter commands; preserve group order among equal scores via stable sort. */
export function filterCommands(
  commands: readonly AppCommand[],
  query: string,
): RankedCommand[] {
  const trimmed = query.trim();
  if (!trimmed) {
    return commands.map((command) => ({ command, score: 0 }));
  }

  const ranked: RankedCommand[] = [];
  for (const command of commands) {
    const titleScore = fuzzyScore(trimmed, command.title);
    const hayScore = fuzzyScore(trimmed, commandHaystack(command));
    const score =
      titleScore === null && hayScore === null
        ? null
        : Math.max(titleScore ?? Number.NEGATIVE_INFINITY, hayScore ?? Number.NEGATIVE_INFINITY);
    if (score !== null && Number.isFinite(score)) {
      ranked.push({ command, score });
    }
  }

  ranked.sort((a, b) => {
    if (b.score !== a.score) return b.score - a.score;
    if (a.command.group !== b.command.group) {
      return groupOrder(a.command.group) - groupOrder(b.command.group);
    }
    return a.command.title.localeCompare(b.command.title);
  });
  return ranked;
}

const GROUP_ORDER = ["Navigate", "View", "Actions"] as const;

export function groupOrder(group: string): number {
  const idx = GROUP_ORDER.indexOf(group as (typeof GROUP_ORDER)[number]);
  return idx === -1 ? 99 : idx;
}

/** Collapse a ranked list into sections for optional group headers. */
export function groupRankedCommands(
  ranked: readonly RankedCommand[],
): { group: string; items: RankedCommand[] }[] {
  const map = new Map<string, RankedCommand[]>();
  for (const item of ranked) {
    const list = map.get(item.command.group) ?? [];
    list.push(item);
    map.set(item.command.group, list);
  }

  const sections: { group: string; items: RankedCommand[] }[] = [];
  for (const group of GROUP_ORDER) {
    const items = map.get(group);
    if (items?.length) sections.push({ group, items });
  }
  for (const [group, items] of map) {
    if (!GROUP_ORDER.includes(group as (typeof GROUP_ORDER)[number])) {
      sections.push({ group, items });
    }
  }
  return sections;
}
