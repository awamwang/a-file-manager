// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import {
  QUICK_ACCESS_CAP,
  rankQuickAccess,
  resetVisitStatsForTests,
  visitScore,
  type VisitItem,
} from "../src/state/visitStats.ts";

resetVisitStatsForTests();

const now = 30 * 86_400_000;
const visits: VisitItem[] = [
  {
    path: String.raw`C:\Rare`,
    openCount: 1,
    lastOpenedAt: now,
    isDir: true,
  },
  {
    path: String.raw`C:\Hot`,
    openCount: 20,
    lastOpenedAt: now - 2 * 86_400_000,
    isDir: true,
  },
  {
    path: String.raw`C:\Warm`,
    openCount: 5,
    lastOpenedAt: now,
    isDir: true,
  },
];

assert.ok(visitScore(visits[1], now) > visitScore(visits[2], now));
assert.ok(visitScore(visits[2], now) > visitScore(visits[0], now));

const ranked = rankQuickAccess(visits, now, QUICK_ACCESS_CAP);
assert.equal(ranked[0].path, String.raw`C:\Hot`);
assert.equal(ranked[1].path, String.raw`C:\Warm`);
assert.equal(ranked[2].path, String.raw`C:\Rare`);
assert.ok(ranked.length <= QUICK_ACCESS_CAP);

const capped = rankQuickAccess(visits, now, 2);
assert.equal(capped.length, 2);

console.log("Visit stats ranking checks OK (AIA-74).");
