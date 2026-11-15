// SPDX-License-Identifier: GPL-3.0-or-later
/** 仅名称匹配（项目 / 工作组 / 工具）：大小写不敏感子序列，描述不参与。 */

import { fuzzyScore } from "../commands/fuzzyFilter.ts";

export type NameMatch<T> = { entry: T; score: number };

export function matchByName<T>(
  entries: readonly T[],
  name: (entry: T) => string,
  query: string,
): NameMatch<T>[] {
  const q = query.trim();
  if (!q) return [];
  const matches: NameMatch<T>[] = [];
  for (const entry of entries) {
    const score = fuzzyScore(q, name(entry));
    if (score !== null) matches.push({ entry, score });
  }
  matches.sort(
    (a, b) => b.score - a.score || name(a.entry).localeCompare(name(b.entry)),
  );
  return matches;
}
