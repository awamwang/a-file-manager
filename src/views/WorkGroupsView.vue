<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import {
  activeWorkGroupId,
  liveOpenState,
  restoreWorkGroupOpenState,
  snapshotLiveOpenState,
} from "../state/navigatorBrowse";
import type { WorkGroupOpenState } from "../state/openStateModel";
import FluentIcon from "../components/icons/FluentIcon.vue";
import { toastError, toastInfo, toastSuccess } from "../state/toast";

const { t } = useI18n();

type WorkGroup = {
  id: string;
  name: string;
  isDefault: boolean;
  memberPaths: string[];
  openState: WorkGroupOpenState | null;
};

type UserWorkGroups = {
  workGroups: WorkGroup[];
  activeWorkGroupId: string;
};

type SoftNotice = {
  kind: string;
  workGroupId: string;
  openedMemberPaths: string[];
};

type SilentSwitchResult = {
  document: UserWorkGroups;
  openStateToRestore: WorkGroupOpenState | null;
  requireSavePrompt: boolean;
  relativeMemberPathsToOpen: string[];
  notice: SoftNotice | null;
};

const document = ref<UserWorkGroups | null>(null);
const error = ref<string | null>(null);
const newName = ref("");
const memberPath = ref("");
const loading = ref(false);

const active = computed(
  () =>
    document.value?.workGroups.find((g) => g.id === document.value?.activeWorkGroupId) ?? null,
);

const liveTabs = computed(() => (liveOpenState.value?.tabGroups ?? []).flat());

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    document.value = await invoke<UserWorkGroups>("work_groups_get");
    activeWorkGroupId.value = document.value.activeWorkGroupId;
    const current = document.value.workGroups.find(
      (g) => g.id === document.value!.activeWorkGroupId,
    );
    // Prefer live Navigator strip; seed from disk only when strip is empty.
    if (!liveOpenState.value.tabGroups.length && current?.openState) {
      await restoreWorkGroupOpenState(document.value.activeWorkGroupId, current.openState);
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    loading.value = false;
  }
}

async function createGroup() {
  document.value = await invoke<UserWorkGroups>("work_groups_create", {
    name: newName.value.trim() || null,
  });
  newName.value = "";
}

async function silentSwitch(targetId: string) {
  error.value = null;
  const result = await invoke<SilentSwitchResult>("work_groups_silent_switch", {
    targetWorkGroupId: targetId,
    liveOpenState: snapshotLiveOpenState(),
  });
  if (result.requireSavePrompt) {
    const msg = t("workGroups.errorSavePrompt");
    error.value = msg;
    toastError(msg);
    return;
  }
  document.value = result.document;
  if (result.notice) {
    toastInfo(
      t("workGroups.softSync", { paths: result.notice.openedMemberPaths.join(", ") }),
    );
  }
  await restoreWorkGroupOpenState(
    result.document.activeWorkGroupId,
    result.openStateToRestore,
  );
}

async function addMember() {
  if (!active.value || !memberPath.value.trim()) return;
  const [nextDoc, nextOpen, soft] = await invoke<
    [UserWorkGroups, WorkGroupOpenState, SoftNotice | null]
  >("work_groups_add_member", {
    workGroupId: active.value.id,
    memberPath: memberPath.value.trim(),
    liveOpenState: snapshotLiveOpenState(),
  });
  document.value = nextDoc;
  if (soft) {
    toastInfo(t("workGroups.softSync", { paths: soft.openedMemberPaths.join(", ") }));
  }
  memberPath.value = "";
  await restoreWorkGroupOpenState(active.value.id, nextOpen);
}

async function saveMembersFromTabs() {
  if (!active.value) return;
  document.value = await invoke<UserWorkGroups>("work_groups_save_members_from_open_state", {
    workGroupId: active.value.id,
    liveOpenState: snapshotLiveOpenState(),
  });
}

async function promoteToProject() {
  if (!active.value || active.value.isDefault) return;
  error.value = null;
  try {
    await invoke("projects_promote_from_work_group", {
      workGroupId: active.value.id,
    });
    toastSuccess(t("workGroups.promoteDone", { name: active.value.name }));
  } catch (err) {
    const raw = err instanceof Error ? err.message : String(err);
    let msg = raw;
    if (raw.includes("PROJECT_DEFAULT_WORK_GROUP_FORBIDDEN")) {
      msg = t("projects.errorDefaultWorkGroup");
    } else if (raw.includes("PROJECT_UNKNOWN_WORK_GROUP")) {
      msg = t("projects.errorUnknownWorkGroup");
    }
    error.value = msg;
    toastError(msg);
  }
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <section class="fluent-page">
    <header class="fluent-page-header">
      <h1 class="fluent-page-title">{{ t("workGroups.title") }}</h1>
      <p class="fluent-page-meta">{{ t("workGroups.meta") }}</p>
    </header>

    <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>

    <div v-if="document" class="layout">
      <aside class="master-list">
        <button
          v-for="group in document.workGroups"
          :key="group.id"
          type="button"
          class="fluent-surface group"
          :class="{ 'group--active': group.id === document.activeWorkGroupId }"
          @click="silentSwitch(group.id)"
        >
          <span class="group-head">
            <span class="fluent-icon-wash">
              <FluentIcon :name="group.isDefault ? 'home' : 'viewAll'" size="md" />
            </span>
            <strong>{{ group.name }}</strong>
          </span>
          <span class="fluent-page-meta">{{
            group.isDefault
              ? t("workGroups.default")
              : t("workGroups.membersCount", { count: group.memberPaths.length })
          }}</span>
        </button>

        <form class="create" @submit.prevent="createGroup">
          <input
            v-model="newName"
            class="fluent-control"
            :placeholder="t('workGroups.newNamePlaceholder')"
          />
          <button class="fluent-control fluent-control--accent" type="submit">
            {{ t("common.create") }}
          </button>
        </form>
      </aside>

      <div v-if="active" class="fluent-surface detail">
        <h2>{{ active.name }}</h2>
        <p class="fluent-page-meta">
          {{ t("workGroups.detailMeta", { id: active.id, count: active.memberPaths.length }) }}
        </p>

        <h3>{{ t("workGroups.hardMembers") }}</h3>
        <ul>
          <li v-for="path in active.memberPaths" :key="path">{{ path }}</li>
          <li v-if="!active.memberPaths.length" class="fluent-page-meta">
            {{ t("workGroups.noHardMembers") }}
          </li>
        </ul>

        <form class="row" @submit.prevent="addMember">
          <input
            v-model="memberPath"
            class="fluent-control"
            :placeholder="t('workGroups.addMemberPlaceholder')"
          />
          <button class="fluent-control fluent-control--accent" type="submit">
            {{ t("workGroups.addSync") }}
          </button>
        </form>

        <h3>{{ t("workGroups.runtimeOpenState") }}</h3>
        <ul>
          <li v-for="tab in liveTabs" :key="tab.id">{{ tab.path }}</li>
          <li v-if="!liveTabs.length" class="fluent-page-meta">{{ t("workGroups.noOpenTabs") }}</li>
        </ul>

        <div class="actions">
          <button class="fluent-control" type="button" @click="saveMembersFromTabs">
            {{ t("workGroups.saveMembersFromTabs") }}
          </button>
          <button
            v-if="!active.isDefault"
            class="fluent-control fluent-control--accent"
            type="button"
            @click="promoteToProject"
          >
            {{ t("workGroups.promoteToProject") }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.banner--soft {
  display: flex;
  justify-content: space-between;
  gap: var(--fluent-space-md);
}

.linkish {
  border: 0;
  background: transparent;
  color: var(--fluent-color-fill-accent);
  cursor: pointer;
  font: inherit;
}

.layout {
  display: grid;
  grid-template-columns: 16rem 1fr;
  gap: var(--fluent-space-lg);
}

.master-list {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.group {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--fluent-space-xs);
  min-height: var(--fluent-control-height-comfortable);
  padding: var(--fluent-space-md);
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
  width: 100%;
}

.group-head {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  min-width: 0;
}

.group--active {
  border-color: var(--fluent-color-stroke-accent-soft);
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
  box-shadow: inset 3px 0 0 var(--fluent-color-stroke-accent-soft);
}

.create,
.row,
.actions {
  display: flex;
  gap: var(--fluent-space-sm);
  flex-wrap: wrap;
}

.create .fluent-control:first-child,
.row .fluent-control:first-child {
  flex: 1;
}

.detail {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  padding: var(--fluent-space-lg);
}

.detail h2,
.detail h3 {
  margin: 0;
}

.detail ul {
  margin: 0;
  padding-left: var(--fluent-space-lg);
}
</style>
