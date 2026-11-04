<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";

const { t } = useI18n();

type WorkGroupOpenStateTab = {
  id: string;
  name: string;
  path: string;
  type: string;
};

type WorkGroupOpenState = {
  tabGroups: WorkGroupOpenStateTab[][];
  currentTabGroupIndex: number;
  currentTabIndex: number;
};

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
const liveOpenState = ref<WorkGroupOpenState | null>(null);
const notice = ref<SoftNotice | null>(null);
const error = ref<string | null>(null);
const newName = ref("");
const memberPath = ref("");
const loading = ref(false);

const active = computed(() =>
  document.value?.workGroups.find((g) => g.id === document.value?.activeWorkGroupId) ?? null,
);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    document.value = await invoke<UserWorkGroups>("work_groups_get");
    const current = document.value.workGroups.find(
      (g) => g.id === document.value!.activeWorkGroupId,
    );
    liveOpenState.value = current?.openState
      ? structuredClone(current.openState)
      : { tabGroups: [], currentTabGroupIndex: 0, currentTabIndex: 0 };
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
    liveOpenState: liveOpenState.value,
  });
  if (result.requireSavePrompt) {
    error.value = t("workGroups.errorSavePrompt");
    return;
  }
  document.value = result.document;
  liveOpenState.value = result.openStateToRestore;
  notice.value = result.notice;
}

async function addMember() {
  if (!active.value || !memberPath.value.trim()) return;
  const [nextDoc, nextOpen, soft] = await invoke<
    [UserWorkGroups, WorkGroupOpenState, SoftNotice | null]
  >("work_groups_add_member", {
    workGroupId: active.value.id,
    memberPath: memberPath.value.trim(),
    liveOpenState: liveOpenState.value,
  });
  document.value = nextDoc;
  liveOpenState.value = nextOpen;
  notice.value = soft;
  memberPath.value = "";
}

async function saveOpenState() {
  if (!active.value) return;
  document.value = await invoke<UserWorkGroups>("work_groups_save_open_state", {
    workGroupId: active.value.id,
    liveOpenState: liveOpenState.value,
  });
}

async function saveMembersFromTabs() {
  if (!active.value) return;
  document.value = await invoke<UserWorkGroups>("work_groups_save_members_from_open_state", {
    workGroupId: active.value.id,
    liveOpenState: liveOpenState.value,
  });
}

const liveTabs = computed(() =>
  (liveOpenState.value?.tabGroups ?? []).flat(),
);

onMounted(() => {
  void refresh();
});
</script>

<template>
  <section class="panel">
    <header class="header">
      <h1>{{ t("workGroups.title") }}</h1>
      <p class="meta">{{ t("workGroups.meta") }}</p>
    </header>

    <p v-if="loading" class="banner">{{ t("common.loading") }}</p>
    <p v-else-if="error" class="banner banner--error">{{ error }}</p>
    <p v-if="notice" class="banner banner--soft">
      {{ t("workGroups.softSync", { paths: notice.openedMemberPaths.join(", ") }) }}
      <button type="button" class="linkish" @click="notice = null">{{ t("common.dismiss") }}</button>
    </p>

    <div v-if="document" class="layout">
      <aside class="list">
        <button
          v-for="group in document.workGroups"
          :key="group.id"
          type="button"
          class="group"
          :class="{ 'group--active': group.id === document.activeWorkGroupId }"
          @click="silentSwitch(group.id)"
        >
          <strong>{{ group.name }}</strong>
          <span>{{
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

      <div v-if="active" class="detail">
        <h2>{{ active.name }}</h2>
        <p class="meta">
          {{ t("workGroups.detailMeta", { id: active.id, count: active.memberPaths.length }) }}
        </p>

        <h3>{{ t("workGroups.hardMembers") }}</h3>
        <ul>
          <li v-for="path in active.memberPaths" :key="path">{{ path }}</li>
          <li v-if="!active.memberPaths.length" class="meta">
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
          <li v-if="!liveTabs.length" class="meta">{{ t("workGroups.noOpenTabs") }}</li>
        </ul>

        <div class="actions">
          <button class="fluent-control" type="button" @click="saveOpenState">
            {{ t("workGroups.saveOpenState") }}
          </button>
          <button class="fluent-control" type="button" @click="saveMembersFromTabs">
            {{ t("workGroups.saveMembersFromTabs") }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.panel {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}

.header h1 {
  margin: 0;
  font-size: var(--fluent-font-size-title);
  font-weight: var(--fluent-font-weight-semibold);
}

.meta {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.banner {
  margin: 0;
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
}

.banner--error {
  color: var(--fluent-color-status-danger);
}

.banner--soft {
  display: flex;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  border-color: var(--fluent-color-fill-accent);
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

.list {
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
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.group--active {
  border-color: var(--fluent-color-fill-accent);
  font-weight: var(--fluent-font-weight-semibold);
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
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
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
