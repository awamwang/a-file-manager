// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Navigator 列表「键入定位」(type-ahead) 缓冲状态机与前缀匹配。
 *
 * 纯模块：无 Vue / IPC / DOM 依赖，供视图接线与 Node strip-types 守卫共用。
 * 匹配语义对齐 Windows「在视图中选择键入的项目」：大小写/重音不敏感前缀匹配，
 * 连续快敲累积前缀，重复单字符循环候选，超时重置。
 *
 * 多语言边界（本期）：字符集为拉丁字母 + 数字；不做 IME 提交串匹配。
 * IME 组字期的按键拦截在视图层完成（isComposing / keyCode 229 / "Process"）。
 */

/** 无输入多久后重置缓冲（ms）。 */
export const TYPE_AHEAD_TIMEOUT_MS = 1200;

export type TypeAheadState = {
  /** 归一化（小写）后的累积前缀，如 "app"。 */
  buffer: string;
  /** 本次按键是否为「循环步进」（缓冲单字符重复）。 */
  cycle: boolean;
  /** 上次输入时间戳（performance.now() 或 Date.now()，单位 ms）。 */
  lastAt: number;
  /** 上次命中索引，循环时从它之后继续；-1 表示尚无命中。 */
  lastMatchedIndex: number;
};

export function initialTypeAheadState(): TypeAheadState {
  return { buffer: "", cycle: false, lastAt: 0, lastMatchedIndex: -1 };
}

export function resetTypeAhead(): TypeAheadState {
  return initialTypeAheadState();
}

/**
 * 输入一个可打印字符，返回新缓冲状态（不改入参）。
 *
 * - 超时（now - lastAt > TIMEOUT）→ 丢弃旧缓冲，从 ch 重新开始。
 * - 缓冲恰为单字符且再次敲同一字符 → buffer 不变、cycle = true。
 * - 其余 → 扩展 buffer，cycle = false。
 */
export function feedChar(
  state: TypeAheadState,
  ch: string,
  now: number,
): TypeAheadState {
  const c = ch.toLocaleLowerCase();
  const timedOut = now - state.lastAt > TYPE_AHEAD_TIMEOUT_MS;
  if (timedOut) {
    return { buffer: c, cycle: false, lastAt: now, lastMatchedIndex: -1 };
  }
  if (state.buffer.length === 1 && state.buffer === c) {
    return { buffer: state.buffer, cycle: true, lastAt: now, lastMatchedIndex: state.lastMatchedIndex };
  }
  return {
    buffer: state.buffer + c,
    cycle: false,
    lastAt: now,
    lastMatchedIndex: -1,
  };
}

/**
 * 判断 name 是否以 buffer 为前缀（base-sensitivity，大小写/重音不敏感）。
 *
 * 码点级切片（Array.from）避免代理对被截断；比较交给注入的 collator，
 * 使定位顺序与 `navigatorSort.ts` 的排序顺序一致。
 */
export function nameStartsWith(
  name: string,
  buffer: string,
  collator: Intl.Collator,
): boolean {
  if (!buffer) return false;
  const namePts = Array.from(name);
  const bufPts = Array.from(buffer);
  if (bufPts.length > namePts.length) return false;
  const head = namePts.slice(0, bufPts.length).join("");
  return collator.compare(head, buffer) === 0;
}

export type FindTypeAheadMatchArgs = {
  /** 与 visibleEntries 同序的显示名。 */
  names: readonly string[];
  buffer: string;
  /** 当前选中索引，cycle 时从其后开始；-1 表示从头。 */
  fromIndex: number;
  collator: Intl.Collator;
  /** 循环步进（从 fromIndex+1 起环形扫描）。 */
  cycle: boolean;
};

/**
 * 在有序显示名中找前缀命中的索引；-1 表示无匹配。
 *
 * - 非循环：从 0 起找第一个命中（"跳到第一个 A 打头"）。
 * - 循环：从 fromIndex + 1 起环形扫描（含 wrap 回 0）。
 */
export function findTypeAheadMatch(args: FindTypeAheadMatchArgs): number {
  const { names, buffer, fromIndex, collator, cycle } = args;
  if (!buffer || names.length === 0) return -1;
  if (!cycle) {
    for (let i = 0; i < names.length; i += 1) {
      if (nameStartsWith(names[i] ?? "", buffer, collator)) return i;
    }
    return -1;
  }
  const n = names.length;
  const start = fromIndex >= 0 && fromIndex < n ? fromIndex + 1 : 0;
  for (let k = 0; k < n; k += 1) {
    const i = (start + k) % n;
    if (nameStartsWith(names[i] ?? "", buffer, collator)) return i;
  }
  return -1;
}
