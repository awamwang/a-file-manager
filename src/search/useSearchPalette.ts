// SPDX-License-Identifier: GPL-3.0-or-later
/** 搜索面板编排：范围组合、跨源检索（过期丢弃）、固定分组 + 组内上限。 */

import { computed, ref, watch } from "vue";
import {
  loadScopes,
  persistScopes,
  toggleScope as applyScopeToggle,
} from "./scope.ts";
import {
  SEARCH_GROUP_LIMIT,
  SEARCH_SOURCE_ORDER,
  type SearchSource,
  type SearchSourceId,
  type SearchSourceResult,
  type SearchResultItem,
} from "./types.ts";

export type SearchSourceState = {
  status: "ready" | "loading" | "unavailable";
  items: SearchResultItem[];
};

export type PaletteRow = {
  item: SearchResultItem;
  /** flat 列表内的索引（键盘选中用）。 */
  index: number;
};

export type PaletteGroup = {
  source: SearchSource;
  state: SearchSourceState;
  rows: PaletteRow[];
};

export type ScopeChip = {
  source: SearchSource;
  /** Alt+N 提示（按显示顺序 1 起）。 */
  index: number;
  active: boolean;
};

export function useSearchPalette(sources: () => readonly SearchSource[]) {
  const storage =
    typeof window === "undefined" ? undefined : window.localStorage;

  const query = ref("");
  const activeIndex = ref(0);
  /** null = 未持久化 / 按「全部」处理（全选，不是独立模式）。 */
  const storedScopes = ref<SearchSourceId[] | null>(loadScopes(storage));
  const sourceStates = ref<Partial<Record<SearchSourceId, SearchSourceState>>>(
    {},
  );

  const availableSources = computed(() =>
    sources().filter((source) => source.enabled()),
  );

  /** 生效范围：持久化选择 ∩ 可用源；交集为空回落「全部」。 */
  const activeScopes = computed<SearchSourceId[]>(() => {
    const available = availableSources.value.map((source) => source.id);
    const stored = storedScopes.value;
    if (!stored) return available;
    const effective = stored.filter((id) => available.includes(id));
    return effective.length ? effective : available;
  });

  const scopeChips = computed<ScopeChip[]>(() => {
    const chips: ScopeChip[] = [];
    let index = 1;
    for (const id of SEARCH_SOURCE_ORDER) {
      const source = availableSources.value.find((entry) => entry.id === id);
      if (!source) continue;
      chips.push({
        source,
        index,
        active: activeScopes.value.includes(id),
      });
      index += 1;
    }
    return chips;
  });

  let searchSeq = 0;

  function runSearch(): void {
    const seq = ++searchSeq;
    const q = query.value;
    const active = new Set(activeScopes.value);
    const nextStates: Partial<Record<SearchSourceId, SearchSourceState>> = {};
    for (const source of availableSources.value) {
      if (!active.has(source.id)) continue;
      nextStates[source.id] = { status: "loading", items: [] };
    }
    sourceStates.value = nextStates;
    for (const source of availableSources.value) {
      if (!active.has(source.id)) continue;
      void (async () => {
        let result: SearchSourceResult;
        try {
          result = await source.search(q);
        } catch {
          result = { status: "unavailable", items: [] };
        }
        if (seq !== searchSeq) return; // 过期结果丢弃
        sourceStates.value = {
          ...sourceStates.value,
          [source.id]: { status: result.status, items: result.items },
        };
      })();
    }
  }

  /** 固定分组顺序 + 每组上限；ready 且空的分组隐藏。 */
  const groups = computed<PaletteGroup[]>(() => {
    const out: PaletteGroup[] = [];
    const active = new Set(activeScopes.value);
    let index = 0;
    for (const id of SEARCH_SOURCE_ORDER) {
      if (!active.has(id)) continue;
      const source = availableSources.value.find((entry) => entry.id === id);
      const state = sourceStates.value[id];
      if (!source || !state) continue;
      const rows: PaletteRow[] = [];
      for (const item of state.items.slice(0, SEARCH_GROUP_LIMIT)) {
        rows.push({ item, index });
        index += 1;
      }
      if (!rows.length && state.status === "ready") continue;
      out.push({ source, state, rows });
    }
    return out;
  });

  const flat = computed<PaletteRow[]>(() =>
    groups.value.flatMap((group) => group.rows),
  );

  watch(query, () => {
    activeIndex.value = 0;
    runSearch();
  });

  watch(activeScopes, () => {
    runSearch();
  });

  watch(flat, (rows) => {
    if (activeIndex.value >= rows.length) {
      activeIndex.value = Math.max(0, rows.length - 1);
    }
  });

  function toggleScope(id: SearchSourceId): void {
    const available = availableSources.value.map((source) => source.id);
    const next = applyScopeToggle(activeScopes.value, available, id);
    storedScopes.value = next;
    persistScopes(next, storage);
  }

  /** 面板内 Alt+N：按 chip 显示顺序切换；超出数量则忽略。 */
  function toggleScopeAt(index1Based: number): void {
    const chip = scopeChips.value[index1Based - 1];
    if (chip) toggleScope(chip.source.id);
  }

  /** 面板打开时复位并检索（query 已为空时也要刷新）。 */
  function reset(): void {
    activeIndex.value = 0;
    if (query.value === "") {
      runSearch();
    } else {
      query.value = "";
    }
  }

  /** 命令列表 / locale 变化后重跑当前查询。 */
  function rerun(): void {
    runSearch();
  }

  return {
    query,
    activeIndex,
    activeScopes,
    scopeChips,
    groups,
    flat,
    toggleScope,
    toggleScopeAt,
    reset,
    rerun,
  };
}
