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
  openMembersIntoOpenState,
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
export async function openActiveMembersIntoOpenState(): Promise<string[] | null> {
  const active = activeWorkGroup.value;
  if (!active?.memberPaths.length) return [];
  workGroupChromeBusy.value = true;
  workGroupChromeError.value = null;
  try {
    return await openMembersIntoOpenState(active.memberPaths);
  } catch (err) {
    workGroupChromeError.value = err instanceof Error ? err.message : String(err);
    return null;
  } finally {
    workGroupChromeBusy.value = false;
  }
}

export function dismissWorkGroupSoftNotice(): void {
  workGroupSoftNotice.value = null;
}
