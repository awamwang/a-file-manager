// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 跨标签页投放会话（AIA-242 续）。
 *
 * HTML5 `dragover` 阶段读不到 `dataTransfer.getData`，而 hover 切换标签会清空
 * 全局选中项（`selectedPaths`）——只用选中项预判会误判成跨盘复制。这里在按下/
 * 拖起时缓存源路径，供 dragover 预览与 drop 兜底共用，保证「提示」与「实际
 * 动作」一致：同盘剪切、跨盘复制。
 */
import { ref } from "vue";
import type { DropMode } from "./mode.ts";

/** 拖动会话最长有效期；OLE 未回传 dragend 时兜底，避免残留路径被误用。 */
export const DRAG_SESSION_TTL_MS = 60_000;

export type DropHint = {
  /** 屏幕坐标（已做过窗口边界钳制）。 */
  x: number;
  y: number;
  mode: DropMode;
  /** 目标显示名（标签名 / 文件名 / 目录名）。 */
  target: string;
  /** 目标唯一键，用于 dragleave 时只隐藏自己那条提示。 */
  key: string;
};

/** 跟随光标的投放提示；由 DropHintOverlay 渲染。 */
export const dropHint = ref<DropHint | null>(null);

let sources: string[] = [];
let startedAt = 0;

function clean(paths: string[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const raw of paths) {
    const path = raw.trim();
    if (!path) continue;
    const key = path.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(path);
  }
  return out;
}

/** 登记本次拖动的源路径（每次按下/拖起都会重置，天然幂等）。 */
export function beginDragSources(paths: string[]): void {
  sources = clean(paths);
  startedAt = Date.now();
  dropHint.value = null;
}

export function endDragSession(): void {
  sources = [];
  startedAt = 0;
  dropHint.value = null;
}

/** 仍在有效期内的源路径；外部应用拖入时为空。 */
export function activeDragSources(): string[] {
  if (!sources.length) return [];
  if (Date.now() - startedAt > DRAG_SESSION_TTL_MS) {
    endDragSession();
    return [];
  }
  return [...sources];
}

const HINT_OFFSET_X = 16;
const HINT_OFFSET_Y = 18;
const HINT_WIDTH = 236;
const HINT_HEIGHT = 54;

/** 在光标旁显示「松开剪切/复制到此处」，mode 与实际 drop 使用同一套规则。 */
export function showDropHint(
  event: DragEvent,
  mode: DropMode,
  target: string,
  key: string,
): void {
  let pointerX = event.clientX;
  let pointerY = event.clientY;
  // 少数原生 OLE 拖入事件不带坐标：退化到目标元素中心。
  if (!pointerX && !pointerY) {
    const el = event.currentTarget as HTMLElement | null;
    const rect = el?.getBoundingClientRect?.();
    if (rect) {
      pointerX = rect.left + rect.width / 2;
      pointerY = rect.top + rect.height / 2;
    }
  }
  const vw = globalThis.innerWidth || 0;
  const vh = globalThis.innerHeight || 0;
  let x = pointerX + HINT_OFFSET_X;
  let y = pointerY + HINT_OFFSET_Y;
  if (vw && x + HINT_WIDTH > vw) x = Math.max(8, pointerX - HINT_WIDTH - HINT_OFFSET_X);
  if (vh && y + HINT_HEIGHT > vh) y = Math.max(8, pointerY - HINT_HEIGHT - HINT_OFFSET_Y);
  const current = dropHint.value;
  // dragover 每秒可触发数十次：同一目标、同一模式下只在光标明显移动时重绘。
  if (
    current &&
    current.key === key &&
    current.mode === mode &&
    current.target === target &&
    Math.abs(current.x - x) < 3 &&
    Math.abs(current.y - y) < 3
  ) {
    return;
  }
  dropHint.value = { x, y, mode, target, key };
}

/** 隐藏提示；传 key 时只隐藏该目标的提示，避免相邻目标的 dragleave 抢清。 */
export function hideDropHint(key?: string): void {
  if (key && dropHint.value && dropHint.value.key !== key) return;
  dropHint.value = null;
}
