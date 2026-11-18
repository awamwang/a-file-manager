// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator chrome Work Group 展示层 — busy/error/notice；会话编排在 navigatorBrowse。 */

import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  activeWorkGroupId,
  hydrateWorkGroups,
  rememberHydratedWorkGroups,
  snapshotLiveOpenState,
  type UserWorkGroupsDoc,
} from "../openState/index.ts";
import {
  openOrFocusNavigatorTab,
  restoreWorkGroupOpenState,
  silentSwitchWorkGroup as runSilentWorkGroupSwitch,
  type SilentSwitchResult,
  type SoftSyncNotice,
} from "../navigatorBrowse.ts";

export type WorkGroupSummary = UserWorkGroupsDoc["workGroups"][number];

export const workGroupsDoc = ref<UserWorkGroupsDoc | null>(null);
export const workGroupChromeError = ref<string | null>(null);
export const workGroupSoftNotice = ref<SoftSyncNotice | null>(null);
export const workGroupChromeBusy = ref(false);

export const activeWorkGroup = computed(
  () =>
    workGroupsDoc.value?.workGroups.find(
      (group) => group.id === workGroupsDoc.value?.activeWorkGroupId,
    ) ?? null,
);

/** Load WG doc for chrome; shares startup hydrate with OpenState bootstrap unless `force`. */
export async function loadWorkGroupsChrome(options?: {
  force?: boolean;
}): Promise<UserWorkGroupsDoc | null> {
  workGroupChromeError.value = null;
  try {
    const doc = await hydrateWorkGroups(options);
    workGroupsDoc.value = doc;
    activeWorkGroupId.value = doc.activeWorkGroupId;
    return doc;
  } catch (err) {
    workGroupChromeError.value = err instanceof Error ? err.message : String(err);
    return null;
  }
}

/**
 * Chrome 包装：展示缓存 + busy/error/notice；strip/IPC 编排在会话模块。
 */
export async function silentSwitchWorkGroup(
  targetId: string,
): Promise<SilentSwitchResult | null> {
  const priorDoc = workGroupsDoc.value;
  if (!targetId || targetId === priorDoc?.activeWorkGroupId) {
    return null;
  }
  workGroupChromeBusy.value = true;
  workGroupChromeError.value = null;
  const cachedTarget = priorDoc?.workGroups.find((group) => group.id === targetId);
  if (priorDoc && cachedTarget) {
    workGroupsDoc.value = {
      ...priorDoc,
      activeWorkGroupId: targetId,
    };
  }
  try {
    const { result, error } = await runSilentWorkGroupSwitch(targetId, priorDoc);
    if (error) {
      workGroupChromeError.value = error;
      if (priorDoc && error === "REQUIRE_SAVE_PROMPT") {
        workGroupsDoc.value = priorDoc;
      } else if (priorDoc && !result) {
        workGroupsDoc.value = priorDoc;
      }
    }
    if (result && !result.requireSavePrompt) {
      workGroupsDoc.value = result.document;
      workGroupSoftNotice.value = result.notice;
    }
    return result;
  } finally {
    workGroupChromeBusy.value = false;
  }
}

export async function saveActiveMembersFromTabsHard(): Promise<boolean> {
  const active = activeWorkGroup.value;
  if (!active) return false;
  workGroupChromeBusy.value = true;
  workGroupChromeError.value = null;
  try {
    const doc = await invoke<UserWorkGroupsDoc>(
      "work_groups_save_members_from_open_state",
      {
        workGroupId: active.id,
        liveOpenState: snapshotLiveOpenState(),
      },
    );
    rememberHydratedWorkGroups(doc);
    workGroupsDoc.value = doc;
    return true;
  } catch (err) {
    workGroupChromeError.value = err instanceof Error ? err.message : String(err);
    return false;
  } finally {
    workGroupChromeBusy.value = false;
  }
}

/** Soft-sync active WG hard members into the live Open State strip (Sigma-style). */
/**
 * 新建工作组快捷方式：把当前打开的标签直接作为新组成员，建立并激活新工作组。
 * 返回新工作组 id；失败返回 null（busy/error 状态照常维护）。
 */
export async function createWorkGroupFromLiveTabs(): Promise<string | null> {
  workGroupChromeBusy.value = true;
  workGroupChromeError.value = null;
  try {
    const snapshot = snapshotLiveOpenState();
    const result = await invoke<SilentSwitchResult>("work_groups_create", {
      name: null,
      liveOpenState: snapshot,
    });
    if (result.requireSavePrompt) return null;
    const newId = result.document.activeWorkGroupId;
    // 当前打开的标签写为新工作组的硬成员。
    const doc = await invoke<UserWorkGroupsDoc>(
      "work_groups_save_members_from_open_state",
      { workGroupId: newId, liveOpenState: snapshot },
    );
    rememberHydratedWorkGroups(doc);
    workGroupsDoc.value = doc;
    // 保持当前标签带不丢：把快照刷回运行时 Open State。
    await restoreWorkGroupOpenState(newId, snapshot);
    return newId;
  } catch (err) {
    workGroupChromeError.value = err instanceof Error ? err.message : String(err);
    return null;
  } finally {
    workGroupChromeBusy.value = false;
  }
}

/** 新建工作组后请求 WorkGroupsView 进入改名状态的一次性信号（消费后自动清空）。 */
export const pendingRenameWorkGroupId = ref<string | null>(null);

export function consumePendingRenameWorkGroupId(): string | null {
  const id = pendingRenameWorkGroupId.value;
  pendingRenameWorkGroupId.value = null;
  return id;
}

/**
 * 保证目标工作组是当前运行时工作组（点击成员/项目标签前的第一步）。
 * 以 `activeWorkGroupId`（运行时）为准，先把 chrome 展示缓存对齐，避免
 * silentSwitchWorkGroup 因缓存过期而误判「已经在目标组」跳过切换。
 * 返回 null 表示已就绪，否则返回错误码供调用方提示。
 */
export async function ensureWorkGroupActive(
  targetId: string | null | undefined,
): Promise<string | null> {
  if (!targetId || targetId === activeWorkGroupId.value) return null;
  workGroupChromeError.value = null;
  if (
    workGroupsDoc.value &&
    activeWorkGroupId.value &&
    workGroupsDoc.value.activeWorkGroupId !== activeWorkGroupId.value
  ) {
    workGroupsDoc.value = {
      ...workGroupsDoc.value,
      activeWorkGroupId: activeWorkGroupId.value,
    };
  }
  const result = await silentSwitchWorkGroup(targetId);
  if (workGroupChromeError.value) return workGroupChromeError.value;
  if (result?.requireSavePrompt) return "REQUIRE_SAVE_PROMPT";
  return activeWorkGroupId.value === targetId ? null : "WORK_GROUP_SWITCH_FAILED";
}

/**
 * 项目 / 工作组里的成员标签：先切到对应工作组，再跳到该工作组中已打开的
 * 同名标签；该成员尚未打开时 openOrFocusNavigatorTab 会新建标签。
 */
export async function openWorkGroupMemberTab(
  targetId: string | null | undefined,
  path: string,
): Promise<string | null> {
  const trimmed = path.trim();
  if (!trimmed) return null;
  const error = await ensureWorkGroupActive(targetId);
  if (error) return error;
  await openOrFocusNavigatorTab(trimmed);
  return null;
}

export function dismissWorkGroupSoftNotice(): void {
  workGroupSoftNotice.value = null;
}
