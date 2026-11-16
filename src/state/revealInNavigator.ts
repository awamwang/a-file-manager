// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 通用「打开目录并选中条目」入口（reveal-and-select）：
 * - 支持单个/多个目标，文件/目录类型不限；
 * - 自动导航到目标父目录（复用已打开标签），跨路由时先切回 Navigator；
 * - listing 加载完成后按实际存在的条目选中（1..n），并滚动到首个选中行；
 * - 目标跨多个父目录时，仅定位第一组（调用方应保证同目录，如任务中心结果）。
 */

import {
  currentPath,
  listing,
  navigateTo,
  openOrFocusNavigatorTab,
} from "./navigatorBrowse.ts";
import { navigableParentPath } from "./navHistory.ts";
import { setSelection } from "./navigatorSelection.ts";
import { normalizePathKey, pathsEqual } from "./pathKey.ts";

export type RevealInNavigatorOptions = {
  /** 默认 true：导航后选中目标条目；false 时仅打开目标所在目录。 */
  select?: boolean;
};

/** 不在 Navigator 路由时切回（动态 import 避免与 router 静态环）。 */
async function ensureNavigatorRoute(): Promise<void> {
  try {
    const { default: router } = await import("../router/index.ts");
    if (router.currentRoute.value.path !== "/navigator") {
      await router.push("/navigator");
    }
  } catch {
    /* router 不可用时继续按当前路由执行 */
  }
}

/** 列表行按 data-entry-path 渲染；滚动首个选中行进入可视区。 */
export function scrollSelectionIntoView(path: string, attempts = 10): void {
  requestAnimationFrame(() => {
    const row = document.querySelector<HTMLElement>(
      `[data-entry-path="${CSS.escape(path)}"]`,
    );
    if (row) {
      row.scrollIntoView({ block: "nearest", inline: "nearest" });
      return;
    }
    // 虚拟列表/新 listing 首帧可能尚未渲染目标行：有界重试。
    if (attempts > 1) scrollSelectionIntoView(path, attempts - 1);
  });
}

/**
 * 防御 epoch 竞态：`navigateTo` 的结果可能被并发导航覆盖（browseEpoch 使其提前返回，
 * listing 停留在旧目录）。有界等待 listing 真正指向目标目录后再做选中。
 */
function waitForListing(dir: string, timeoutMs = 3000): Promise<void> {
  return new Promise((resolve) => {
    const startedAt = Date.now();
    const check = () => {
      const current = listing.value;
      if ((current && pathsEqual(current.path, dir)) || Date.now() - startedAt >= timeoutMs) {
        resolve();
        return;
      }
      setTimeout(check, 40);
    };
    check();
  });
}

/**
 * 在 Navigator 中展示一条或多条目（文件/目录均可）：
 * 导航到其父目录并选中这些条目。
 * - 单条：等价于资源管理器「定位到该文件」；
 * - 多条：一次导航、全部选中（须位于同一父目录）。
 */
export async function revealItemsInNavigator(
  itemPaths: string[],
  options?: RevealInNavigatorOptions,
): Promise<void> {
  const targets = itemPaths
    .map((path) => path.trim())
    .filter((path) => path.length > 0);
  if (!targets.length) return;
  const select = options?.select !== false;

  const parentDir = navigableParentPath(targets[0] ?? "");
  if (!parentDir) {
    // 盘根/引导页等无父目录：退化为直接打开该路径。
    await ensureNavigatorRoute();
    await openOrFocusNavigatorTab(targets[0] ?? "");
    return;
  }

  await ensureNavigatorRoute();
  if (normalizePathKey(currentPath.value) !== normalizePathKey(parentDir)) {
    await openOrFocusNavigatorTab(parentDir);
  }
  // openOrFocus 内部为非阻塞 reveal；此处确保 listing 完成后再选中。
  // 绕过缓存：目标可能是刚移动/新建的条目，旧缓存里不存在。
  await navigateTo(parentDir, {
    syncOpenState: false,
    recordHistory: false,
    bypassListingCache: true,
  });
  if (!select) return;
  // epoch 竞态防御：navigateTo 可能被并发导航覆盖而提前返回，
  // 此时 listing 仍是旧目录内容（新条目不在其中 → 选中必然落空）。
  await waitForListing(parentDir);

  const entries = listing.value?.entries ?? [];
  const byKey = new Map(
    entries.map((entry) => [normalizePathKey(entry.path), entry.path]),
  );
  const inDir: string[] = [];
  for (const target of targets) {
    const hit = byKey.get(normalizePathKey(target));
    if (hit && !inDir.includes(hit)) inDir.push(hit);
  }
  if (!inDir.length) return;
  setSelection(inDir, inDir[0] ?? null);
  scrollSelectionIntoView(inDir[0] ?? "");
}

/** 仅打开某目录（无选中），跨路由时同样切回 Navigator。 */
export async function openDirInNavigator(dirPath: string): Promise<void> {
  const target = dirPath.trim();
  if (!target) return;
  await ensureNavigatorRoute();
  await openOrFocusNavigatorTab(target);
}
