<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  FIL-70：右下角长任务状态列表条。
  - 任务开始：右下角弹窗展示进度，3 秒后自动折叠为状态条（悬停可暂停折叠）；
  - 折叠条/弹窗承载任务列表：复制/移动进度、完成/失败状态；
  - 完成记录七日一清，保留天数与条数上限可在设置区调整。
-->
<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import FluentIcon from "../icons/FluentIcon.vue";
import { formatBytes } from "../../state/formatBytes";
import {
  clampMaxRecords,
  clampRetentionDays,
  clearFinishedTasks,
  finishedTasks,
  MAX_MAX_RECORDS,
  MAX_RETENTION_DAYS,
  MIN_MAX_RECORDS,
  MIN_RETENTION_DAYS,
  onTaskStarted,
  orderedTasks,
  removeTask,
  runningTasks,
  taskCenterConfig,
  tasks,
  updateTaskCenterConfig,
  type LongTask,
  type LongTaskStatus,
} from "../../state/taskCenter";

const { t } = useI18n();

/** 弹窗自动折叠延时（毫秒）。 */
const AUTO_COLLAPSE_MS = 3000;

const expanded = ref(false);
const configOpen = ref(false);
const hovered = ref(false);
/** 用户手动展开时不自动折叠。 */
const manualExpanded = ref(false);

const retentionDraft = ref(taskCenterConfig.value.retentionDays);
const maxRecordsDraft = ref(taskCenterConfig.value.maxRecords);

const runningCount = computed(() => runningTasks.value.length);
const finishedCount = computed(() => finishedTasks.value.length);

const hasTasks = computed(() => tasks.value.length > 0);

const showPill = computed(() => hasTasks.value && !expanded.value);

/** 折叠条聚合摘要：进行中优先，否则展示最近记录数。 */
const pillSummary = computed(() => {
  if (runningCount.value > 0) {
    return t("taskCenter.runningSummary", { count: runningCount.value });
  }
  return t("taskCenter.recentSummary", { count: finishedCount.value });
});

const pillPercent = computed(() => {
  if (runningCount.value === 0) return null;
  const task = orderedTasks.value.find((item) => item.status === "running");
  if (!task) return null;
  return taskPercent(task);
});

type CollapseTimer = {
  remainingMs: number;
  deadline: number;
  handle: ReturnType<typeof setTimeout> | null;
};

let collapseTimer: CollapseTimer | null = null;

function clearCollapseTimer(): void {
  if (collapseTimer?.handle) clearTimeout(collapseTimer.handle);
  collapseTimer = null;
}

function collapseNow(): void {
  clearCollapseTimer();
  expanded.value = false;
  manualExpanded.value = false;
}

function armAutoCollapse(): void {
  clearCollapseTimer();
  collapseTimer = {
    remainingMs: AUTO_COLLAPSE_MS,
    deadline: Date.now() + AUTO_COLLAPSE_MS,
    handle: setTimeout(() => {
      if (hovered.value || manualExpanded.value) return;
      collapseNow();
    }, AUTO_COLLAPSE_MS),
  };
}

function pauseAutoCollapse(): void {
  if (!collapseTimer?.handle) return;
  clearTimeout(collapseTimer.handle);
  collapseTimer.handle = null;
  collapseTimer.remainingMs = Math.max(
    0,
    collapseTimer.deadline - Date.now(),
  );
}

function resumeAutoCollapse(): void {
  if (!collapseTimer || collapseTimer.handle) return;
  if (collapseTimer.remainingMs <= 0) {
    if (!manualExpanded.value) collapseNow();
    else collapseTimer = null;
    return;
  }
  const remaining = collapseTimer.remainingMs;
  collapseTimer.deadline = Date.now() + remaining;
  collapseTimer.handle = setTimeout(() => {
    if (hovered.value || manualExpanded.value) return;
    collapseNow();
  }, remaining);
}

/** 任务开始 → 右下角弹窗展示；3 秒后折叠（手动展开时保持不动）。 */
function handleTaskStarted(): void {
  if (manualExpanded.value || (expanded.value && collapseTimer)) {
    if (collapseTimer && !manualExpanded.value) {
      // 既有弹窗倒计时进行中：刷新折叠倒计时。
      armAutoCollapse();
    }
    return;
  }
  expanded.value = true;
  armAutoCollapse();
}

function expandManually(): void {
  expanded.value = true;
  manualExpanded.value = true;
  clearCollapseTimer();
}

function toggleConfig(): void {
  retentionDraft.value = taskCenterConfig.value.retentionDays;
  maxRecordsDraft.value = taskCenterConfig.value.maxRecords;
  configOpen.value = !configOpen.value;
}

function applyConfig(): void {
  updateTaskCenterConfig({
    retentionDays: clampRetentionDays(retentionDraft.value),
    maxRecords: clampMaxRecords(maxRecordsDraft.value),
  });
  retentionDraft.value = taskCenterConfig.value.retentionDays;
  maxRecordsDraft.value = taskCenterConfig.value.maxRecords;
}

function taskPercent(task: LongTask): number | null {
  if (task.bytesTotal <= 0) return null;
  return Math.min(100, Math.floor((task.bytesDone / task.bytesTotal) * 100));
}

function kindLabel(task: LongTask): string {
  return task.kind === "move"
    ? t("taskCenter.kindMove")
    : t("taskCenter.kindCopy");
}

function statusLabel(status: LongTaskStatus): string {
  switch (status) {
    case "running":
      return t("taskCenter.statusRunning");
    case "completed":
      return t("taskCenter.statusCompleted");
    case "failed":
      return t("taskCenter.statusFailed");
    default:
      return t("taskCenter.statusInterrupted");
  }
}

function statusIcon(status: LongTaskStatus): "refresh" | "check" | "close" {
  switch (status) {
    case "completed":
      return "check";
    case "failed":
      return "close";
    default:
      return "refresh";
  }
}

function taskTitleText(task: LongTask): string {
  const suffix =
    task.sourceCount > 1
      ? ` ${t("taskCenter.more", { count: task.sourceCount - 1 })}`
      : "";
  return `${kindLabel(task)} ${task.firstSourceName}${suffix}`;
}

function taskTimeText(task: LongTask): string {
  const stamp = task.endedAt ?? task.startedAt;
  try {
    return new Date(stamp).toLocaleTimeString();
  } catch {
    return "";
  }
}

function taskProgressText(task: LongTask): string {
  const percent = taskPercent(task);
  if (percent === null) {
    return task.bytesTotal > 0
      ? `${formatBytes(task.bytesDone)} / ${formatBytes(task.bytesTotal)}`
      : formatBytes(task.bytesDone);
  }
  return `${percent}% · ${formatBytes(task.bytesDone)} / ${formatBytes(task.bytesTotal)}`;
}

/** Toast 在任务条可见时整体上移，避免右下角互相遮挡。 */
function syncToastInset(): void {
  const inset = showPill.value
    ? "calc(2.4rem + var(--fluent-space-sm))"
    : "0px";
  document.documentElement.style.setProperty("--fluent-taskbar-inset", inset);
}

let offTaskStarted: (() => void) | null = null;

onMounted(() => {
  offTaskStarted = onTaskStarted(handleTaskStarted);
  syncToastInset();
});

onBeforeUnmount(() => {
  clearCollapseTimer();
  offTaskStarted?.();
  document.documentElement.style.setProperty("--fluent-taskbar-inset", "0px");
});

watch(showPill, syncToastInset);

watch(hovered, (value) => {
  if (value) pauseAutoCollapse();
  else resumeAutoCollapse();
});
</script>

<template>
  <div
    v-if="hasTasks"
    id="afm-task-center-host"
    class="task-center-host"
    @mouseenter="hovered = true"
    @mouseleave="hovered = false"
  >
    <!-- 折叠状态条：点击展开任务列表 -->
    <button
      v-if="showPill"
      type="button"
      class="task-pill"
      :aria-label="t('taskCenter.expandTitle')"
      :title="t('taskCenter.expandTitle')"
      @click="expandManually"
    >
      <span v-if="runningCount > 0" class="task-pill-spinner" aria-hidden="true"></span>
      <FluentIcon v-else name="check" size="sm" />
      <span class="task-pill-summary">{{ pillSummary }}</span>
      <span v-if="pillPercent !== null" class="task-pill-percent">{{ pillPercent }}%</span>
    </button>

    <!-- 展开弹窗：任务列表 -->
    <div v-else class="task-panel" role="dialog" :aria-label="t('taskCenter.title')">
      <header class="task-panel-header">
        <span class="task-panel-title">{{ t("taskCenter.title") }}</span>
        <span class="task-panel-header-actions">
          <button
            type="button"
            class="task-icon-button"
            :title="t('taskCenter.clearFinished')"
            :aria-label="t('taskCenter.clearFinished')"
            @click="clearFinishedTasks"
          >
            <FluentIcon name="recycle" size="sm" />
          </button>
          <button
            type="button"
            class="task-icon-button"
            :class="{ 'task-icon-button--active': configOpen }"
            :title="t('taskCenter.settings')"
            :aria-label="t('taskCenter.settings')"
            @click="toggleConfig"
          >
            <FluentIcon name="settings" size="sm" />
          </button>
          <button
            type="button"
            class="task-icon-button"
            :title="t('taskCenter.collapse')"
            :aria-label="t('taskCenter.collapse')"
            @click="collapseNow"
          >
            <FluentIcon name="chevronDown" size="sm" />
          </button>
        </span>
      </header>

      <section v-if="configOpen" class="task-config">
        <label class="task-config-row">
          <span class="task-config-label">{{ t("taskCenter.retentionDays") }}</span>
          <input
            v-model.number="retentionDraft"
            class="fluent-control task-config-input"
            type="number"
            :min="MIN_RETENTION_DAYS"
            :max="MAX_RETENTION_DAYS"
            @change="applyConfig"
          />
        </label>
        <p class="task-config-hint">{{ t("taskCenter.retentionDaysHint") }}</p>
        <label class="task-config-row">
          <span class="task-config-label">{{ t("taskCenter.maxRecords") }}</span>
          <input
            v-model.number="maxRecordsDraft"
            class="fluent-control task-config-input"
            type="number"
            :min="MIN_MAX_RECORDS"
            :max="MAX_MAX_RECORDS"
            @change="applyConfig"
          />
        </label>
        <p class="task-config-hint">{{ t("taskCenter.maxRecordsHint") }}</p>
      </section>

      <ul v-if="orderedTasks.length" class="task-list">
        <li v-for="task in orderedTasks" :key="task.id" class="task-row">
          <span class="task-row-icon" :class="`task-row-icon--${task.status}`">
            <span
              v-if="task.status === 'running'"
              class="task-row-spinner"
              aria-hidden="true"
            ></span>
            <FluentIcon v-else :name="statusIcon(task.status)" size="sm" />
          </span>
          <div class="task-row-main">
            <div class="task-row-line">
              <span class="task-row-title" :title="taskTitleText(task)">
                {{ taskTitleText(task) }}
              </span>
              <span class="task-row-time">{{ taskTimeText(task) }}</span>
            </div>
            <div class="task-row-detail" :title="task.error ?? task.destDir">
              {{ task.error ? task.error : `→ ${task.destDir}` }}
            </div>
            <div v-if="task.status === 'running'" class="task-row-progress">
              <span class="task-progress-track" aria-hidden="true">
                <span
                  class="task-progress-fill"
                  :class="{ 'task-progress-fill--indeterminate': taskPercent(task) === null }"
                  :style="{ width: taskPercent(task) === null ? '100%' : `${taskPercent(task)}%` }"
                ></span>
              </span>
              <span class="task-progress-text">{{ taskProgressText(task) }}</span>
            </div>
            <div v-else class="task-row-status">
              <span
                class="task-status-chip"
                :class="`task-status-chip--${task.status}`"
              >
                {{ statusLabel(task.status) }}
              </span>
            </div>
          </div>
          <button
            type="button"
            class="task-icon-button task-row-remove"
            :title="t('common.remove')"
            :aria-label="t('common.remove')"
            @click="removeTask(task.id)"
          >
            <FluentIcon name="close" size="sm" />
          </button>
        </li>
      </ul>
      <p v-else class="task-empty">{{ t("taskCenter.empty") }}</p>
    </div>
  </div>
</template>

<style scoped>
.task-center-host {
  position: fixed;
  right: var(--fluent-space-lg);
  bottom: calc(var(--fluent-statusbar-height, 2rem) + var(--fluent-space-md));
  z-index: 91;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  max-width: min(24rem, calc(100vw - 2rem));
  pointer-events: none;
}

.task-pill,
.task-panel {
  pointer-events: auto;
}

.task-pill {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  padding: 0.35rem var(--fluent-space-md);
  border-radius: 999px;
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
  color: var(--fluent-color-text-primary);
  box-shadow: var(--fluent-shadow-flyout, 0 8px 24px rgba(0, 0, 0, 0.18));
  font-size: var(--fluent-font-size-body);
  line-height: 1.4;
  cursor: pointer;
  max-width: 100%;
}

.task-pill:hover {
  background: var(--fluent-color-fill-control-secondary, var(--fluent-color-fill-control));
}

.task-pill-summary {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.task-pill-percent {
  flex-shrink: 0;
  color: var(--fluent-color-text-secondary);
  font-variant-numeric: tabular-nums;
}

.task-pill-spinner,
.task-row-spinner {
  flex-shrink: 0;
  width: 0.85rem;
  height: 0.85rem;
  border-radius: 50%;
  border: 2px solid var(--fluent-color-stroke-divider);
  border-top-color: var(--fluent-color-accent-default, #0067c0);
  animation: task-spin 0.9s linear infinite;
}

@keyframes task-spin {
  to {
    transform: rotate(360deg);
  }
}

.task-panel {
  display: flex;
  flex-direction: column;
  width: min(23rem, calc(100vw - 2rem));
  max-height: min(24rem, calc(100vh - 8rem));
  border-radius: var(--fluent-radius-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
  color: var(--fluent-color-text-primary);
  box-shadow: var(--fluent-shadow-flyout, 0 8px 24px rgba(0, 0, 0, 0.18));
  overflow: hidden;
}

.task-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.task-panel-title {
  font-weight: 600;
  font-size: var(--fluent-font-size-body);
}

.task-panel-header-actions {
  display: flex;
  align-items: center;
  gap: 0.15rem;
}

.task-icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.6rem;
  height: 1.6rem;
  border: 0;
  border-radius: var(--fluent-radius-sm, 4px);
  background: transparent;
  color: var(--fluent-color-text-secondary);
  cursor: pointer;
}

.task-icon-button:hover {
  background: var(--fluent-color-fill-subtle);
  color: var(--fluent-color-text-primary);
}

.task-icon-button--active {
  background: var(--fluent-color-fill-subtle);
  color: var(--fluent-color-text-primary);
}

.task-config {
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-subtle);
}

.task-config-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-sm);
}

.task-config-label {
  font-size: var(--fluent-font-size-body);
}

.task-config-input {
  width: 6rem;
}

.task-config-hint {
  margin: 0.15rem 0 0.35rem;
  font-size: var(--fluent-font-size-caption, 0.75rem);
  color: var(--fluent-color-text-secondary);
}

.task-list {
  flex: 1;
  margin: 0;
  padding: 0.2rem 0;
  list-style: none;
  overflow-y: auto;
}

.task-row {
  position: relative;
  display: flex;
  align-items: flex-start;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-sm) var(--fluent-space-md);
}

.task-row + .task-row {
  border-top: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.task-row-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.2rem;
  height: 1.2rem;
  margin-top: 0.1rem;
  flex-shrink: 0;
}

.task-row-icon--completed {
  color: var(--fluent-color-status-success);
}

.task-row-icon--failed {
  color: var(--fluent-color-status-danger);
}

.task-row-icon--interrupted {
  color: var(--fluent-color-text-secondary);
}

.task-row-main {
  flex: 1;
  min-width: 0;
}

.task-row-line {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--fluent-space-sm);
}

.task-row-title {
  min-width: 0;
  font-size: var(--fluent-font-size-body);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.task-row-time {
  flex-shrink: 0;
  font-size: var(--fluent-font-size-caption, 0.75rem);
  color: var(--fluent-color-text-secondary);
  font-variant-numeric: tabular-nums;
}

.task-row-detail {
  margin-top: 0.1rem;
  font-size: var(--fluent-font-size-caption, 0.75rem);
  color: var(--fluent-color-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.task-row-progress {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  margin-top: 0.3rem;
}

.task-progress-track {
  flex: 1;
  height: 0.3rem;
  border-radius: 999px;
  background: var(--fluent-color-stroke-divider);
  overflow: hidden;
}

.task-progress-fill {
  display: block;
  height: 100%;
  border-radius: 999px;
  background: var(--fluent-color-accent-default, #0067c0);
  transition: width 120ms ease-out;
}

.task-progress-fill--indeterminate {
  animation: task-indeterminate 1.1s ease-in-out infinite;
}

@keyframes task-indeterminate {
  0% {
    transform: translateX(-60%);
    opacity: 0.4;
  }
  50% {
    transform: translateX(0);
    opacity: 1;
  }
  100% {
    transform: translateX(60%);
    opacity: 0.4;
  }
}

.task-progress-text {
  flex-shrink: 0;
  font-size: var(--fluent-font-size-caption, 0.75rem);
  color: var(--fluent-color-text-secondary);
  font-variant-numeric: tabular-nums;
}

.task-row-status {
  margin-top: 0.2rem;
}

.task-status-chip {
  display: inline-block;
  padding: 0 0.4rem;
  border-radius: 999px;
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  font-size: var(--fluent-font-size-caption, 0.75rem);
  color: var(--fluent-color-text-secondary);
}

.task-status-chip--completed {
  border-color: var(--fluent-color-status-success);
  color: var(--fluent-color-status-success);
}

.task-status-chip--failed {
  border-color: var(--fluent-color-status-danger);
  color: var(--fluent-color-status-danger);
}

.task-row-remove {
  flex-shrink: 0;
  visibility: hidden;
}

.task-row:hover .task-row-remove,
.task-row-remove:focus-visible {
  visibility: visible;
}

.task-empty {
  margin: 0;
  padding: var(--fluent-space-md);
  text-align: center;
  font-size: var(--fluent-font-size-body);
  color: var(--fluent-color-text-secondary);
}
</style>
