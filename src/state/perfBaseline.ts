// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * `--afm-perf-baseline` / `AFM_PERF_BASELINE=1` 自驱性能基线（S1/S5）。
 *
 * 启动带该参数（或环境变量）时：自动采集冷启动里程碑链（S1）与两标签
 * 往返切换墙钟（S5），逐行经 `perf_baseline_report` 打回启动终端
 * （`[AFM_BASELINE] …`，debug/`tauri dev` 构建继承终端），随后自动退出。
 * 诊断模式（ADR 0025），不进门禁链；release GUI exe 无控制台，一律用
 * dev 构建跑。
 */

import { invoke } from "@tauri-apps/api/core";
import { liveOpenState } from "./openState/index.ts";
import {
  closeNavigatorTab,
  currentPath,
  listing,
  listLoading,
  newNavigatorTab,
  switchNavigatorTab,
} from "./navigatorBrowse.ts";
import { getRecordedSpans, setSpanRecording } from "./perfProbe.ts";
import { startupMark, startupMarks } from "./startupTiming.ts";

const SETTLE_TIMEOUT_MS = 20_000;
const S5_ROUNDS = 6;
const TAB_B_CANDIDATES = ["C:\\Windows", "C:\\Program Files", "/Applications", "/etc"];

function report(line: string): void {
  void invoke("perf_baseline_report", { line }).catch(() => undefined);
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitFor(cond: () => boolean, timeoutMs: number): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (cond()) return true;
    await sleep(50);
  }
  return cond();
}

/** 文件系统路径（盘符/UNC/posix 根）；afm:// 等虚拟页不算。 */
function isFsPath(path: string): boolean {
  return /^[A-Za-z]:[\\/]/.test(path) || path.startsWith("\\\\") || path.startsWith("/");
}

/** 等当前路径的 listing 就绪（含 LRU 缓存命中路径；虚拟页不要求 listing）。 */
function listingReady(target: string): boolean {
  if (currentPath.value !== target || listLoading.value) return false;
  return isFsPath(target) ? listing.value !== null : true;
}

async function waitListingReady(target: string): Promise<boolean> {
  return waitFor(() => listingReady(target), SETTLE_TIMEOUT_MS);
}

function median(values: number[]): number {
  if (!values.length) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0
    ? Math.round(((sorted[mid - 1] + sorted[mid]) / 2) * 10) / 10
    : sorted[mid];
}

export async function runPerfBaselineIfEnabled(): Promise<void> {
  let on = false;
  try {
    on = await invoke<boolean>("perf_baseline_enabled");
  } catch {
    return; // IPC 不可用（纯 vite dev）：诊断模式无意义
  }
  if (!on) return;
  setSpanRecording(true);

  // S1 尾：等首个（恢复会话）目录的 listing 就绪，补齐里程碑链。
  const initialReady = await waitFor(() => currentPath.value !== "", SETTLE_TIMEOUT_MS)
    .then(() => waitListingReady(currentPath.value));
  if (initialReady) {
    startupMark("first_listing_ready");
  }
  const chain: Record<string, number> = {};
  for (const mark of startupMarks()) {
    chain[mark.label] = Math.round(mark.ms * 10) / 10;
  }
  report(`s1 ${JSON.stringify(chain)}`);

  // S5：两标签往返切换墙钟。临时开第二个标签测完即关，会话状态不残留。
  const tabAPath = currentPath.value;
  const groupA = liveOpenState.value.currentTabGroupIndex;
  const tabBPath = TAB_B_CANDIDATES.find(
    (p) => p.toLowerCase() !== tabAPath.toLowerCase(),
  );
  if (!tabBPath) {
    report("s5 skipped no-candidate-path");
  } else {
    await newNavigatorTab(tabBPath);
    if (await waitListingReady(tabBPath)) {
      const groupB = liveOpenState.value.currentTabGroupIndex;
      const samples: number[] = [];
      for (let i = 0; i < S5_ROUNDS; i++) {
        // 建完标签停在 B：先切回 A，再切 B，往复。
        const targetGroup = i % 2 === 0 ? groupA : groupB;
        const targetPath = i % 2 === 0 ? tabAPath : tabBPath;
        const t0 = performance.now();
        await switchNavigatorTab(targetGroup);
        await waitListingReady(targetPath);
        samples.push(Math.round((performance.now() - t0) * 10) / 10);
        await sleep(150);
      }
      report(
        `s5 ${JSON.stringify({
          tabA: tabAPath,
          tabB: tabBPath,
          rounds: S5_ROUNDS,
          samples,
          median: median(samples),
        })}`,
      );
      await closeNavigatorTab(groupB);
    } else {
      report(`s5 error tabB-not-ready ${tabBPath}`);
    }
  }

  report(`spans ${JSON.stringify(getRecordedSpans())}`);
  report("done");
  await sleep(400); // 让 report 行经 IPC 落到终端再退出
  try {
    await invoke("app_quit");
  } catch {
    // 退出失败不重试：窗口留着，用户手动关即可。
  }
}
