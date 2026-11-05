// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator chrome Work Group switcher — silent switch (AIA-81/90); members save only. */

import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  activeWorkGroupId,
  openMembersIntoOpenState,
  restoreWorkGroupOpenState,
  snapshotLiveOpenState,
  type SilentSwitchResult,
  type SoftSyncNotice,
  type UserWorkGroupsDoc,
} from "./navigatorBrowse";

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

export async function loadWorkGroupsChrome(): Promise<UserWorkGroupsDoc | null> {
  workGroupChromeError.value = null;
  try {
    const doc = await invoke<UserWorkGroupsDoc>("work_groups_get");
    workGroupsDoc.value = doc;
    activeWorkGroupId.value = doc.activeWorkGroupId;
    return doc;
  } catch (err) {
    workGroupChromeError.value = err instanceof Error ? err.message : String(err);
    return null;
  }
}

export async function silentSwitchWorkGroup(
  targetId: string,
): Promise<SilentSwitchResult | null> {
  if (!targetId || targetId === workGroupsDoc.value?.activeWorkGroupId) {
    return null;
  }
  workGroupChromeBusy.value = true;
  workGroupChromeError.value = null;
  try {
    const result = await invoke<SilentSwitchResult>("work_groups_silent_switch", {
      targetWorkGroupId: targetId,
      liveOpenState: snapshotLiveOpenState(),
    });
    if (result.requireSavePrompt) {
      workGroupChromeError.value = "REQUIRE_SAVE_PROMPT";
      return result;
    }
    workGroupsDoc.value = result.document;
    workGroupSoftNotice.value = result.notice;
    await restoreWorkGroupOpenState(
      result.document.activeWorkGroupId,
      result.openStateToRestore,
    );
    return result;
  } catch (err) {
    workGroupChromeError.value = err instanceof Error ? err.message : String(err);
    return null;
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
    workGroupsDoc.value = await invoke<UserWorkGroupsDoc>(
      "work_groups_save_members_from_open_state",
      {
        workGroupId: active.id,
        liveOpenState: snapshotLiveOpenState(),
      },
    );
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
