<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "vue-router";
import {
  activeWorkGroupId,
  liveOpenState,
  openMembersIntoOpenState,
  openOrFocusNavigatorTab,
  restoreWorkGroupOpenState,
  snapshotLeavingOpenState,
  snapshotLiveOpenState,
} from "../state/navigatorBrowse";
import { displayNameFromPath } from "../state/openStateModel";
import type { WorkGroupOpenState } from "../state/openStateModel";
import { workGroupDisplayName } from "../state/workGroupName";
import FluentIcon from "../components/icons/FluentIcon.vue";
import { openConfirmDialog } from "../state/confirmDialog";
import { toastError, toastInfo, toastSuccess } from "../state/toast";

const { t } = useI18n();
const router = useRouter();

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

function pathLabel(path: string): string {
  return displayNameFromPath(path);
}

async function openPathInNavigator(path: string) {
  await openOrFocusNavigatorTab(path);
  await router.push("/navigator");
}

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
  error.value = null;
  const result = await invoke<SilentSwitchResult>("work_groups_create", {
    name: newName.value.trim() || null,
    liveOpenState: snapshotLeavingOpenState(),
  });
  newName.value = "";
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

async function silentSwitch(targetId: string) {
  error.value = null;
  const result = await invoke<SilentSwitchResult>("work_groups_silent_switch", {
    targetWorkGroupId: targetId,
    liveOpenState: snapshotLeavingOpenState(),
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

async function removeMember(path: string) {
  if (!active.value) return;
  document.value = await invoke<UserWorkGroups>("work_groups_remove_member", {
    workGroupId: active.value.id,
    memberPath: path,
  });
  toastInfo(t("workGroups.memberRemoved"));
}

const renaming = ref(false);
const renameDraft = ref("");
const moreMenuOpen = ref(false);
const renameInputEl = ref<HTMLInputElement | null>(null);
const moreMenuEl = ref<HTMLElement | null>(null);
const detailTitleRowEl = ref<HTMLElement | null>(null);
const listMoreOpenId = ref<string | null>(null);
const listRenamingId = ref<string | null>(null);
const listRenameDraft = ref("");
const listRenameInputEl = ref<HTMLInputElement | null>(null);
const masterListEl = ref<HTMLElement | null>(null);

function setListRenameInputEl(el: unknown) {
  listRenameInputEl.value = el instanceof HTMLInputElement ? el : null;
}

function startInlineRename() {
  if (!active.value || active.value.isDefault) return;
  renameDraft.value = active.value.name;
  renaming.value = true;
  moreMenuOpen.value = false;
  listMoreOpenId.value = null;
  listRenamingId.value = null;
  void nextTick(() => {
    renameInputEl.value?.focus();
    renameInputEl.value?.select();
  });
}

function cancelInlineRename() {
  renaming.value = false;
  renameDraft.value = "";
}

function onRenameBlur() {
  // Defer so Enter-commit can clear `renaming` first in the same turn.
  queueMicrotask(() => {
    if (renaming.value) cancelInlineRename();
  });
}

async function commitInlineRename() {
  if (!active.value || !renaming.value) return;
  const next = renameDraft.value.trim();
  renaming.value = false;
  renameDraft.value = "";
  if (!next || next === active.value.name) return;
  document.value = await invoke<UserWorkGroups>("work_groups_rename", {
    workGroupId: active.value.id,
    name: next,
  });
  toastSuccess(t("workGroups.renamed"));
}

function toggleListMore(groupId: string, event: Event) {
  event.stopPropagation();
  moreMenuOpen.value = false;
  listMoreOpenId.value = listMoreOpenId.value === groupId ? null : groupId;
}

function startListRename(group: WorkGroup, event?: Event) {
  if (group.isDefault) return;
  event?.stopPropagation();
  listMoreOpenId.value = null;
  moreMenuOpen.value = false;
  renaming.value = false;
  listRenamingId.value = group.id;
  listRenameDraft.value = group.name;
  void nextTick(() => {
    listRenameInputEl.value?.focus();
    listRenameInputEl.value?.select();
  });
}

function cancelListRename() {
  listRenamingId.value = null;
  listRenameDraft.value = "";
}

function onListRenameBlur() {
  queueMicrotask(() => {
    if (listRenamingId.value) cancelListRename();
  });
}

async function commitListRename(groupId: string) {
  if (listRenamingId.value !== groupId) return;
  const group = document.value?.workGroups.find((g) => g.id === groupId);
  const next = listRenameDraft.value.trim();
  listRenamingId.value = null;
  listRenameDraft.value = "";
  if (!group || !next || next === group.name) return;
  document.value = await invoke<UserWorkGroups>("work_groups_rename", {
    workGroupId: groupId,
    name: next,
  });
  toastSuccess(t("workGroups.renamed"));
}

function onDetailPointerDown(event: PointerEvent) {
  const target = event.target as Node | null;
  if (!target) return;
  if (moreMenuOpen.value && moreMenuEl.value && !moreMenuEl.value.contains(target)) {
    moreMenuOpen.value = false;
  }
  if (renaming.value && detailTitleRowEl.value && !detailTitleRowEl.value.contains(target)) {
    cancelInlineRename();
  }
  if (listMoreOpenId.value) {
    const openMenu = masterListEl.value?.querySelector(
      `[data-list-more-id="${listMoreOpenId.value}"]`,
    );
    if (!openMenu?.contains(target)) {
      listMoreOpenId.value = null;
    }
  }
  if (listRenamingId.value) {
    const row = masterListEl.value?.querySelector(
      `[data-list-item-id="${listRenamingId.value}"]`,
    );
    if (!row?.contains(target)) {
      cancelListRename();
    }
  }
}

async function deleteGroup(group: WorkGroup) {
  if (group.isDefault) return;
  const deletingId = group.id;
  const wasActive = deletingId === activeWorkGroupId.value;
  const confirmed = await openConfirmDialog({
    title: t("dialog.deleteTitle"),
    message: t("workGroups.deleteConfirmClearTabs", { name: group.name }),
    confirmLabel: t("common.delete"),
    cancelLabel: t("common.cancel"),
    danger: true,
  });
  if (!confirmed.confirmed) return;
  listMoreOpenId.value = null;
  moreMenuOpen.value = false;
  document.value = await invoke<UserWorkGroups>("work_groups_delete", {
    workGroupId: deletingId,
  });
  activeWorkGroupId.value = document.value.activeWorkGroupId;
  if (wasActive) {
    const next = document.value.workGroups.find(
      (g) => g.id === document.value!.activeWorkGroupId,
    );
    await restoreWorkGroupOpenState(
      document.value.activeWorkGroupId,
      next?.openState ?? null,
    );
  }
  toastSuccess(t("workGroups.deletedSwitched"));
}

async function deleteActiveGroup() {
  if (!active.value) return;
  await deleteGroup(active.value);
}

async function openMembersIntoRuntime() {
  if (!active.value?.memberPaths.length) {
    toastInfo(t("workGroups.openMembersEmpty"));
    return;
  }
  const opened = await openMembersIntoOpenState(active.value.memberPaths);
  if (opened.length) {
    toastSuccess(t("workGroups.openMembersDone", { count: opened.length }));
    await router.push("/navigator");
  } else {
    toastInfo(t("workGroups.openMembersNone"));
  }
}

async function promoteToProject() {
  if (!active.value || active.value.isDefault) return;
  error.value = null;
  try {
    await invoke("projects_workspace_promote_from_work_group", {
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
  window.addEventListener("pointerdown", onDetailPointerDown, true);
});

onUnmounted(() => {
  window.removeEventListener("pointerdown", onDetailPointerDown, true);
});
</script>

<template>
  <section class="fluent-page">
    <header class="fluent-page-header">
      <h1 id="work-groups-page-title" class="fluent-page-title" tabindex="-1">
        {{ t("workGroups.title") }}
      </h1>
      <p class="fluent-page-meta">{{ t("workGroups.meta") }}</p>
      <p class="fluent-page-meta usage-hint">{{ t("workGroups.usageHint") }}</p>
    </header>

    <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>

    <div v-if="document" class="layout">
      <aside ref="masterListEl" class="master-list">
        <div class="master-list-scroll">
          <div
            v-for="group in document.workGroups"
            :key="group.id"
            class="fluent-surface group"
            :class="{ 'group--active': group.id === document.activeWorkGroupId }"
            role="button"
            tabindex="0"
            :data-list-item-id="group.id"
            @click="silentSwitch(group.id)"
            @keydown.enter.prevent="silentSwitch(group.id)"
          >
            <div class="group-top">
              <span class="group-head">
                <span class="fluent-icon-wash">
                  <FluentIcon :name="group.isDefault ? 'home' : 'viewAll'" size="md" />
                </span>
                <input
                  v-if="listRenamingId === group.id"
                  :ref="setListRenameInputEl"
                  v-model="listRenameDraft"
                  class="fluent-control list-inline-rename"
                  @click.stop
                  @keydown.enter.prevent="commitListRename(group.id)"
                  @keydown.escape.prevent="cancelListRename"
                  @blur="onListRenameBlur"
                />
                <strong v-else>{{ workGroupDisplayName(group, t("workGroups.defaultName")) }}</strong>
              </span>
              <div
                class="list-more-menu"
                :data-list-more-id="group.id"
                @click.stop
              >
                <button
                  type="button"
                  class="fluent-control more-trigger list-more-trigger"
                  :aria-expanded="listMoreOpenId === group.id"
                  aria-haspopup="menu"
                  :title="t('common.more')"
                  @click="toggleListMore(group.id, $event)"
                >
                  ···
                </button>
                <div
                  v-if="listMoreOpenId === group.id"
                  class="more-popover fluent-surface"
                  role="menu"
                >
                  <button
                    v-if="!group.isDefault"
                    type="button"
                    class="more-item"
                    role="menuitem"
                    @click="startListRename(group, $event)"
                  >
                    {{ t("workGroups.rename") }}
                  </button>
                  <button
                    v-if="!group.isDefault"
                    type="button"
                    class="more-item more-item--danger"
                    role="menuitem"
                    @click="deleteGroup(group)"
                  >
                    {{ t("workGroups.delete") }}
                  </button>
                </div>
              </div>
            </div>
            <span class="fluent-page-meta">{{
              group.isDefault
                ? t("workGroups.defaultHint")
                : t("workGroups.membersCount", { count: group.memberPaths.length })
            }}</span>
          </div>
        </div>

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
        <div ref="detailTitleRowEl" class="detail-title-row">
          <input
            v-if="renaming"
            ref="renameInputEl"
            v-model="renameDraft"
            class="fluent-control inline-rename"
            @keydown.enter.prevent="commitInlineRename"
            @keydown.escape.prevent="cancelInlineRename"
            @blur="onRenameBlur"
          />
          <h2
            v-else
            class="inline-title"
            :title="t('workGroups.renameHint')"
            @dblclick="active.isDefault ? undefined : startInlineRename"
          >
            {{ workGroupDisplayName(active, t("workGroups.defaultName")) }}
          </h2>
          <div ref="moreMenuEl" class="more-menu">
            <button
              type="button"
              class="fluent-control more-trigger"
              :aria-expanded="moreMenuOpen"
              aria-haspopup="menu"
              :title="t('common.more')"
              @click="
                listMoreOpenId = null;
                moreMenuOpen = !moreMenuOpen;
              "
            >
              ···
            </button>
            <div v-if="moreMenuOpen" class="more-popover fluent-surface" role="menu">
              <button
                v-if="!active.isDefault"
                type="button"
                class="more-item"
                role="menuitem"
                @click="startInlineRename"
              >
                {{ t("workGroups.rename") }}
              </button>
              <button
                v-if="!active.isDefault"
                type="button"
                class="more-item more-item--danger"
                role="menuitem"
                @click="
                  moreMenuOpen = false;
                  deleteActiveGroup();
                "
              >
                {{ t("workGroups.delete") }}
              </button>
            </div>
          </div>
        </div>
        <p class="fluent-page-meta">
          {{ t("workGroups.detailMeta", { id: active.id, count: active.memberPaths.length }) }}
        </p>

        <h3>{{ t("workGroups.hardMembers") }}</h3>
        <div v-if="active.memberPaths.length" class="path-card-grid">
          <div
            v-for="path in active.memberPaths"
            :key="path"
            class="fluent-surface path-card path-card--with-action"
          >
            <button
              type="button"
              class="path-card-main"
              :title="path"
              @click="openPathInNavigator(path)"
            >
              <span class="fluent-icon-wash">
                <FluentIcon name="folder" size="md" />
              </span>
              <span class="path-card-body">
                <span class="path-card-name">{{ pathLabel(path) }}</span>
                <span class="fluent-page-meta path-card-path">{{ path }}</span>
              </span>
            </button>
            <button
              type="button"
              class="fluent-control path-card-remove"
              :title="t('workGroups.removeMember')"
              :aria-label="t('workGroups.removeMember')"
              @click.stop="removeMember(path)"
            >
              <FluentIcon name="close" size="sm" />
            </button>
          </div>
        </div>
        <p v-else class="fluent-page-meta">{{ t("workGroups.noHardMembers") }}</p>

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
        <div v-if="liveTabs.length" class="path-card-grid">
          <button
            v-for="tab in liveTabs"
            :key="tab.id"
            type="button"
            class="fluent-surface path-card"
            :title="tab.path"
            @click="openPathInNavigator(tab.path)"
          >
            <span class="fluent-icon-wash">
              <FluentIcon name="folderOpen" size="md" />
            </span>
            <span class="path-card-body">
              <span class="path-card-name">{{ tab.name || pathLabel(tab.path) }}</span>
              <span class="fluent-page-meta path-card-path">{{ tab.path }}</span>
            </span>
          </button>
        </div>
        <p v-else class="fluent-page-meta">{{ t("workGroups.noOpenTabs") }}</p>

        <div class="actions">
          <button
            class="fluent-control fluent-control--accent"
            type="button"
            :disabled="!active.memberPaths.length"
            @click="openMembersIntoRuntime"
          >
            {{ t("workGroups.openMembersIntoOpenState") }}
          </button>
          <button class="fluent-control" type="button" @click="saveMembersFromTabs">
            {{ t("workGroups.saveMembersFromTabs") }}
          </button>
          <button
            v-if="!active.isDefault"
            class="fluent-control"
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
.fluent-page {
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.detail-title-row {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  flex-wrap: wrap;
  margin-bottom: var(--fluent-space-xs);
}

.inline-title {
  margin: 0;
  cursor: text;
}

.inline-rename {
  min-width: 12rem;
  flex: 1;
  font-size: var(--fluent-font-size-title);
  font-weight: var(--fluent-font-weight-semibold);
}

.more-menu {
  position: relative;
  margin-inline-start: auto;
  flex-shrink: 0;
}

.more-trigger {
  min-width: 2rem;
  padding-inline: var(--fluent-space-sm);
  letter-spacing: 0.08em;
}

.more-popover {
  position: absolute;
  right: 0;
  top: calc(100% + 4px);
  z-index: 5;
  min-width: 9rem;
  padding: var(--fluent-space-xs);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.more-item {
  border: 0;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  cursor: pointer;
}

.more-item:hover {
  background: var(--fluent-color-fill-selected);
}

.more-item--danger {
  color: var(--fluent-color-status-danger);
}

.layout {
  display: grid;
  grid-template-columns: 16rem 1fr;
  gap: var(--fluent-space-lg);
  flex: 1;
  min-height: 0;
  align-items: stretch;
}

.master-list {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  min-height: 0;
  overflow: hidden;
}

.master-list-scroll {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  flex: 1;
  min-height: 0;
  max-height: 100%;
  overflow-x: hidden;
  overflow-y: auto;
}

.group {
  position: relative;
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
  box-sizing: border-box;
}

.group-top {
  display: flex;
  align-items: flex-start;
  gap: var(--fluent-space-sm);
  width: 100%;
  min-width: 0;
}

.group-head {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  min-width: 0;
  flex: 1;
}

.group-head strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.list-more-menu {
  position: relative;
  flex-shrink: 0;
  margin-inline-start: auto;
}

.list-more-trigger {
  min-height: 1.75rem;
  padding-block: 0;
}

.list-inline-rename {
  min-width: 0;
  flex: 1;
  font-weight: var(--fluent-font-weight-semibold);
}

.group--active {
  border-color: var(--fluent-color-stroke-accent-soft);
  background: var(--fluent-color-fill-selected);
  box-shadow: inset 3px 0 0 var(--fluent-color-stroke-accent-soft);
}

.create,
.row,
.actions {
  display: flex;
  gap: var(--fluent-space-sm);
  flex-wrap: wrap;
}

.create {
  flex-shrink: 0;
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
  min-height: 0;
  overflow: auto;
}

.detail h2,
.detail h3 {
  margin: 0;
}

.usage-hint {
  max-width: 42rem;
}

@media (max-width: 960px) {
  .layout {
    grid-template-columns: 1fr;
    overflow: auto;
  }

  .master-list {
    max-height: min(40vh, 20rem);
  }
}
</style>
