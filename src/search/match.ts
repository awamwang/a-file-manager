// SPDX-License-Identifier: GPL-3.0-or-later
/** 仅名称匹配（项目 / 工作组 / 工具）：大小写不敏感子序列，描述不参与。 */

import { fuzzyScore } from "../commands/fuzzyFilter.ts";

export type NameMatch<T> = { entry: T; score: number };

/** 源内单文本匹配：模糊=子序列（fuzzyScore），否则大小写不敏感子串。 */
export function matchText(
  text: string,
  query: string,
  fuzzy: boolean,
): boolean {
  const q = query.trim();
  if (!q) return false;
  if (fuzzy) return fuzzyScore(q, text) !== null;
  return text.toLowerCase().includes(q.toLowerCase());
}

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
