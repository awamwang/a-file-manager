// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 长任务中心（右下角状态列表条）：
 * - copy/move 任务进度进入列表（Rust `file-ops-progress` 事件驱动）；
 * - 完成记录按保留天数（默认七日一清）+ 条数上限自动清理，两者均可调整；
 * - 任务开始时通知宿主弹窗展示（3 秒后折叠由宿主组件控制）。
 */

import { computed, ref } from "vue";
import { listen } from "@tauri-apps/api/event";

export type LongTaskKind = "copy" | "move";

export type LongTaskStatus = "running" | "completed" | "failed" | "interrupted";

export type LongTask = {
  id: string;
  kind: LongTaskKind;
  status: LongTaskStatus;
  /** 首个来源名（列表行主标题用）。 */
  firstSourceName: string;
  sourceCount: number;
  destDir: string;
  bytesDone: number;
  bytesTotal: number;
  startedAt: number;
  endedAt: number | null;
  error: string | null;
  resultCount: number;
};

export type TaskCenterConfig = {
  /** 完成记录保留天数（七日一清，可调）。 */
  retentionDays: number;
  /** 历史最多保留条数（可调）。 */
  maxRecords: number;
};

export type FileOpsProgressPayload = {
  taskId: string;
  phase: "start" | "progress" | "done";
  bytesDone: number;
  bytesTotal: number;
  ok: boolean;
  error: string | null;
};

const TASKS_STORAGE_KEY = "afm.taskCenter.tasks.v1";
const CONFIG_STORAGE_KEY = "afm.taskCenter.config.v1";

export const DEFAULT_CONFIG: TaskCenterConfig = {
  retentionDays: 7,
  maxRecords: 200,
};

export const MIN_RETENTION_DAYS = 1;
export const MAX_RETENTION_DAYS = 365;
export const MIN_MAX_RECORDS = 10;
export const MAX_MAX_RECORDS = 1000;

const DAY_MS = 86_400_000;

export const tasks = ref<LongTask[]>([]);
export const taskCenterConfig = ref<TaskCenterConfig>({ ...DEFAULT_CONFIG });

/** 同一任务多次 invoke（逐个来源处理）之间的字节累加基线；仅会话内有效。 */
type SegmentBase = { base: number };
const segmentBases = new Map<string, SegmentBase>();

const taskStartListeners = new Set<(id: string) => void>();
let initialized = false;
let taskIdSeq = 0;

export const runningTasks = computed(() =>
  tasks.value.filter((task) => task.status === "running"),
);

export const finishedTasks = computed(() =>
  tasks.value.filter((task) => task.status !== "running"),
);

/** 展示顺序：进行中在前（新的在前），完成后按结束/开始时间倒序。 */
export const orderedTasks = computed(() =>
  [...tasks.value].sort((a, b) => {
    const aRunning = a.status === "running" ? 1 : 0;
    const bRunning = b.status === "running" ? 1 : 0;
    if (aRunning !== bRunning) return bRunning - aRunning;
    const aTime = a.endedAt ?? a.startedAt;
    const bTime = b.endedAt ?? b.startedAt;
    if (bTime !== aTime) return bTime - aTime;
    return a.id.localeCompare(b.id);
  }),
);

export function onTaskStarted(listener: (id: string) => void): () => void {
  taskStartListeners.add(listener);
  return () => {
    taskStartListeners.delete(listener);
  };
}

function clampInt(value: number, min: number, max: number, fallback: number): number {
  if (!Number.isFinite(value)) return fallback;
  return Math.min(max, Math.max(min, Math.round(value)));
}

export function clampRetentionDays(value: number): number {
  return clampInt(value, MIN_RETENTION_DAYS, MAX_RETENTION_DAYS, DEFAULT_CONFIG.retentionDays);
}

export function clampMaxRecords(value: number): number {
  return clampInt(value, MIN_MAX_RECORDS, MAX_MAX_RECORDS, DEFAULT_CONFIG.maxRecords);
}

function readStoredJson<T>(key: string): T | null {
  try {
    const raw = window.localStorage.getItem(key);
    if (!raw) return null;
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

function persistTasks(): void {
  try {
    window.localStorage.setItem(TASKS_STORAGE_KEY, JSON.stringify(tasks.value));
  } catch {
    /* 存储被禁用/超限时保留内存态 */
  }
}

function persistConfig(): void {
  try {
    window.localStorage.setItem(CONFIG_STORAGE_KEY, JSON.stringify(taskCenterConfig.value));
  } catch {
    /* ignore */
  }
}

/** 保留天数/条数上限裁剪；进行中的任务始终保留。 */
function pruneTasks(nowMs: number): void {
  const cutoff = nowMs - taskCenterConfig.value.retentionDays * DAY_MS;
  const kept: LongTask[] = [];
  for (const task of tasks.value) {
    if (task.status === "running") {
      kept.push(task);
      continue;
    }
    const stamp = task.endedAt ?? task.startedAt;
    if (stamp >= cutoff) kept.push(task);
  }
  kept.sort((a, b) => {
    const aTime = a.endedAt ?? a.startedAt;
    const bTime = b.endedAt ?? b.startedAt;
    return bTime - aTime;
  });
  const runningKept = kept.filter((task) => task.status === "running");
  const finishedKept = kept
    .filter((task) => task.status !== "running")
    .slice(0, taskCenterConfig.value.maxRecords);
  tasks.value = [...runningKept, ...finishedKept];
}

function normalizeStatus(value: string): LongTaskStatus {
  if (value === "completed" || value === "failed" || value === "interrupted") return value;
  // 未知状态按已中断处理，避免遗留假的进行中任务。
  return "interrupted";
}

/** 载入持久化任务：清理过期记录，上一会话遗留的 running 标记为已中断。 */
export function loadTaskCenter(): void {
  const config = readStoredJson<Partial<TaskCenterConfig>>(CONFIG_STORAGE_KEY);
  taskCenterConfig.value = {
    retentionDays: clampRetentionDays(Number(config?.retentionDays ?? DEFAULT_CONFIG.retentionDays)),
    maxRecords: clampMaxRecords(Number(config?.maxRecords ?? DEFAULT_CONFIG.maxRecords)),
  };
  const stored = readStoredJson<LongTask[]>(TASKS_STORAGE_KEY);
  const now = Date.now();
  if (Array.isArray(stored)) {
    tasks.value = stored
      .filter((task) => task && typeof task.id === "string")
      .map((task) => ({
        ...task,
        kind: task.kind === "move" ? "move" : "copy",
        status:
          task.status === "running"
            ? "interrupted"
            : normalizeStatus(task.status),
        bytesDone: Math.max(0, Number(task.bytesDone) || 0),
        bytesTotal: Math.max(0, Number(task.bytesTotal) || 0),
        sourceCount: Math.max(0, Number(task.sourceCount) || 0),
        firstSourceName: String(task.firstSourceName ?? ""),
        destDir: String(task.destDir ?? ""),
        startedAt: Number(task.startedAt) || now,
        endedAt: typeof task.endedAt === "number" ? task.endedAt : null,
        error: task.error ?? null,
        resultCount: Math.max(0, Number(task.resultCount) || 0),
      }));
  }
  pruneTasks(now);
  persistTasks();
}

export function updateTaskCenterConfig(partial: Partial<TaskCenterConfig>): void {
  const next: TaskCenterConfig = {
    retentionDays: clampRetentionDays(
      partial.retentionDays ?? taskCenterConfig.value.retentionDays,
    ),
    maxRecords: clampMaxRecords(partial.maxRecords ?? taskCenterConfig.value.maxRecords),
  };
  taskCenterConfig.value = next;
  persistConfig();
  pruneTasks(Date.now());
  persistTasks();
}

function baseName(path: string): string {
  const norm = path.replace(/\//g, "\\").replace(/\\+$/, "");
  const idx = norm.lastIndexOf("\\");
  return idx >= 0 ? norm.slice(idx + 1) : norm;
}

function createTaskId(): string {
  taskIdSeq += 1;
  const random =
    typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
  return `task-${random}-${taskIdSeq}`;
}

/** 任务开始：进入列表并通知宿主弹窗展示（三秒后折叠由宿主控制）。 */
export function beginLongTask(
  kind: LongTaskKind,
  sources: string[],
  destDir: string,
): string {
  const id = createTaskId();
  const now = Date.now();
  const task: LongTask = {
    id,
    kind,
    status: "running",
    firstSourceName: baseName(sources[0] ?? ""),
    sourceCount: sources.length,
    destDir,
    bytesDone: 0,
    bytesTotal: 0,
    startedAt: now,
    endedAt: null,
    error: null,
    resultCount: 0,
  };
  segmentBases.set(id, { base: 0 });
  tasks.value = [task, ...tasks.value];
  persistTasks();
  for (const listener of taskStartListeners) listener(id);
  return id;
}

export function finishLongTask(
  id: string,
  ok: boolean,
  error: string | null,
  resultCount: number,
): void {
  const task = tasks.value.find((item) => item.id === id);
  if (!task) return;
  if (task.status !== "running") return;
  task.status = ok ? "completed" : "failed";
  task.endedAt = Date.now();
  task.error = error;
  task.resultCount = resultCount;
  if (ok) {
    // 跳过冲突等场景下事件不会走满；完成时以总量收口到 100%。
    if (task.bytesTotal > 0) task.bytesDone = task.bytesTotal;
  }
  segmentBases.delete(id);
  persistTasks();
}

export function removeTask(id: string): void {
  segmentBases.delete(id);
  tasks.value = tasks.value.filter((task) => task.id !== id);
  persistTasks();
}

export function clearFinishedTasks(): void {
  tasks.value = tasks.value.filter((task) => task.status === "running");
  persistTasks();
}

/**
 * 套用 Rust `file-ops-progress` 事件。
 * 同一任务跨多次 invoke 时以 bytesDone 为基线累加（start 冻结基线）。
 */
export function applyProgressEvent(payload: FileOpsProgressPayload): void {
  const task = tasks.value.find((item) => item.id === payload.taskId);
  if (!task || task.status !== "running") return;
  const done = Math.max(0, Number(payload.bytesDone) || 0);
  const total = Math.max(0, Number(payload.bytesTotal) || 0);
  switch (payload.phase) {
    case "start": {
      const base = segmentBases.get(task.id) ?? { base: task.bytesDone };
      base.base = task.bytesDone;
      segmentBases.set(task.id, base);
      task.bytesTotal = task.bytesTotal > 0 ? task.bytesTotal + total : total;
      task.bytesDone = base.base;
      break;
    }
    case "progress": {
      const base = segmentBases.get(task.id) ?? { base: 0 };
      segmentBases.set(task.id, base);
      task.bytesDone = base.base + done;
      if (total > 0 && task.bytesTotal < base.base + total) {
        task.bytesTotal = base.base + total;
      }
      break;
    }
    case "done": {
      const base = segmentBases.get(task.id) ?? { base: 0 };
      segmentBases.set(task.id, base);
      task.bytesDone = Math.max(task.bytesDone, base.base + done);
      if (!payload.ok && payload.error && !task.error) task.error = payload.error;
      break;
    }
  }
}

/**
 * 初始化任务中心：载入持久化记录 + 订阅 Rust 进度事件。幂等，AppShell 启动时调用。
 */
export function initTaskCenter(): void {
  if (initialized) return;
  initialized = true;
  loadTaskCenter();
  void listen<FileOpsProgressPayload>("file-ops-progress", (event) => {
    applyProgressEvent(event.payload);
  }).catch(() => undefined);
}

/** 测试辅助：重置内存态。 */
export function resetTaskCenterForTests(): void {
  tasks.value = [];
  taskCenterConfig.value = { ...DEFAULT_CONFIG };
  segmentBases.clear();
  taskStartListeners.clear();
  initialized = false;
}
