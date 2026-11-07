// SPDX-License-Identifier: GPL-3.0-or-later
/** Navigator chrome Work Group switcher — silent switch (AIA-81/90); members save only. */

import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  activeWorkGroupId,
  cancelOpenStatePersist,
  hydrateWorkGroups,
  openMembersIntoOpenState,
  rememberHydratedWorkGroups,
  restoreWorkGroupOpenState,
  snapshotLeavingOpenState,
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

async function rollbackOptimisticSwitch(
  priorDoc: UserWorkGroupsDoc,
  leavingFallback: ReturnType<typeof snapshotLeavingOpenState>,
): Promise<void> {
  workGroupsDoc.value = priorDoc;
  activeWorkGroupId.value = priorDoc.activeWorkGroupId;
  const prior = priorDoc.workGroups.find(
    (group) => group.id === priorDoc.activeWorkGroupId,
  );
  await restoreWorkGroupOpenState(
    priorDoc.activeWorkGroupId,
    prior?.openState ?? leavingFallback,
  );
}

/**
 * Silent WG switch. AIA-259: paint strip/path from the local WG cache first,
 * then await disk load+save; reconcile soft-sync tabs without a second list when
 * the active path is unchanged.
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
  // Silent switch owns the leaving-group disk write — drop debounced persist.
  cancelOpenStatePersist();
  const leaving = snapshotLeavingOpenState();
  const cachedTarget = priorDoc?.workGroups.find((group) => group.id === targetId);
  let paintedOptimistic = false;
  if (priorDoc && cachedTarget) {
    workGroupsDoc.value = {
      ...priorDoc,
      activeWorkGroupId: targetId,
    };
    activeWorkGroupId.value = targetId;
    await restoreWorkGroupOpenState(targetId, cachedTarget.openState);
    paintedOptimistic = true;
  }
  try {
    const result = await invoke<SilentSwitchResult>("work_groups_silent_switch", {
      targetWorkGroupId: targetId,
      liveOpenState: leaving,
    });
    if (result.requireSavePrompt) {
      if (paintedOptimistic && priorDoc) {
        await rollbackOptimisticSwitch(priorDoc, leaving);
      }
      workGroupChromeError.value = "REQUIRE_SAVE_PROMPT";
      return result;
    }
    rememberHydratedWorkGroups(result.document);
    workGroupsDoc.value = result.document;
    workGroupSoftNotice.value = result.notice;
    // reveal:false skips a duplicate navigator_list when optimistic path matches.
    await restoreWorkGroupOpenState(
      result.document.activeWorkGroupId,
      result.openStateToRestore,
      { reveal: !paintedOptimistic },
    );
    return result;
  } catch (err) {
    if (paintedOptimistic && priorDoc) {
      try {
        await rollbackOptimisticSwitch(priorDoc, leaving);
      } catch {
        /* keep switch error */
      }
    }
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
