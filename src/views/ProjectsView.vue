<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import {
  applyProjectSilentSwitch,
  openMembersIntoOpenState,
  openOrFocusNavigatorTab,
  openPathsAsNavigatorTabs,
  snapshotLiveOpenState,
  type SilentSwitchResult,
} from "../state/navigatorBrowse";
import { toastInfo, toastSuccess } from "../state/toast";
import FluentIcon from "../components/icons/FluentIcon.vue";
import type { FluentGlyphId } from "../components/icons/glyphs";

const { t } = useI18n();

const STATUS_ICONS: Record<string, FluentGlyphId> = {
  backlog: "board",
  doing: "flash",
  review: "eye",
  done: "check",
};

type WorkspaceLink = {
  id: string;
  path: string;
  label: string;
  kind: string;
  addedAt: number;
};

type ProjectTask = {
  id: string;
  title: string;
  body: string;
  statusId: string;
  sortKey: number;
  workspaceLinkId: string | null;
  outcomeUrl: string | null;
  createdAt: number;
  updatedAt: number;
  completedAt: number | null;
};

type UserProject = {
  id: string;
  name: string;
  description: string;
  workGroupId?: string | null;
  workspaceLinks: WorkspaceLink[];
  tasks: ProjectTask[];
  contextNotes: string;
  createdAt: number;
  updatedAt: number;
};

type UserProjects = {
  schemaVersion: number;
  projects: UserProject[];
  activeProjectId: string | null;
};

type WorkGroupOption = {
  id: string;
  name: string;
  isDefault: boolean;
  memberPaths?: string[];
};

type ActivateResult = {
  projects: UserProjects;
  silentSwitch: SilentSwitchResult | null;
};

type OpenAllResult = {
  projectId: string;
  paths: string[];
  workGroupMutated: boolean;
};

const PROJECT_ERROR_KEYS: Record<string, string> = {
  PROJECT_DEFAULT_WORK_GROUP_FORBIDDEN: "projects.errorDefaultWorkGroup",
  PROJECT_UNKNOWN_WORK_GROUP: "projects.errorUnknownWorkGroup",
  PROJECT_UNBOUND: "projects.errorUnbound",
  PROJECT_SILENT_SWITCH_SAVE_PROMPT: "workGroups.errorSavePrompt",
};

function mapProjectError(err: unknown): string {
  const raw = err instanceof Error ? err.message : String(err);
  for (const [code, key] of Object.entries(PROJECT_ERROR_KEYS)) {
    if (raw.includes(code)) return t(key);
  }
  return raw;
}

const STATUS_LABEL_KEYS = {
  backlog: "projects.statusBacklog",
  doing: "projects.statusDoing",
  review: "projects.statusReview",
  done: "projects.statusDone",
} as const;

const STATUSES = computed(() =>
  (Object.keys(STATUS_LABEL_KEYS) as Array<keyof typeof STATUS_LABEL_KEYS>).map((id) => ({
    id,
    label: t(STATUS_LABEL_KEYS[id]),
  })),
);

const router = useRouter();
const document = ref<UserProjects | null>(null);
const workGroups = ref<WorkGroupOption[]>([]);
const selectedId = ref<string | null>(null);
const viewMode = ref<"list" | "board">("board");
const error = ref<string | null>(null);
const loading = ref(false);

const newName = ref("");
const bindWorkGroupId = ref("");
const linkPath = ref("");
const taskTitle = ref("");
const contextDraft = ref("");

const selected = computed(
  () => document.value?.projects.find((p) => p.id === selectedId.value) ?? null,
);

const bindableWorkGroups = computed(() =>
  workGroups.value.filter((group) => !group.isDefault && group.id !== "default-work-group"),
);

const boundWorkGroupName = computed(() => {
  const id = selected.value?.workGroupId;
  if (!id) return null;
  return workGroups.value.find((group) => group.id === id)?.name ?? id;
});

const openTaskCount = computed(
  () => selected.value?.tasks.filter((t) => t.statusId !== "done").length ?? 0,
);

function tasksForStatus(statusId: string): ProjectTask[] {
  return (selected.value?.tasks ?? [])
    .filter((t) => t.statusId === statusId)
    .slice()
    .sort((a, b) => a.sortKey - b.sortKey);
}

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    const [projectsDoc, wgDoc] = await Promise.all([
      invoke<UserProjects>("projects_get"),
      invoke<{ workGroups: WorkGroupOption[] }>("work_groups_get"),
    ]);
    document.value = projectsDoc;
    workGroups.value = wgDoc.workGroups;
    if (
      selectedId.value &&
      !document.value.projects.some((p) => p.id === selectedId.value)
    ) {
      selectedId.value = document.value.activeProjectId;
    }
    if (!selectedId.value) {
      selectedId.value =
        document.value.activeProjectId ?? document.value.projects[0]?.id ?? null;
    }
    syncBindDraft();
    syncContextDraft();
  } catch (err) {
    error.value = mapProjectError(err);
  } finally {
    loading.value = false;
  }
}

function syncContextDraft() {
  contextDraft.value = selected.value?.contextNotes ?? "";
}

function syncBindDraft() {
  bindWorkGroupId.value = selected.value?.workGroupId ?? "";
}

async function selectProject(id: string) {
  error.value = null;
  selectedId.value = id;
  syncBindDraft();
  syncContextDraft();
  try {
    const result = await invoke<ActivateResult>("projects_activate", {
      projectId: id,
      liveOpenState: snapshotLiveOpenState(),
    });
    document.value = result.projects;
    const soft = await applyProjectSilentSwitch(result.silentSwitch);
    if (soft?.openedMemberPaths?.length) {
      toastInfo(
        t("workGroups.softSync", { paths: soft.openedMemberPaths.join(", ") }),
      );
    }
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

async function createProject() {
  error.value = null;
  try {
    document.value = await invoke<UserProjects>("projects_create", {
      name: newName.value.trim() || null,
      description: null,
      workGroupId: null,
    });
    newName.value = "";
    selectedId.value =
      document.value.activeProjectId ??
      document.value.projects[document.value.projects.length - 1]?.id ??
      null;
    syncBindDraft();
    syncContextDraft();
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

const renamingProject = ref(false);
const projectRenameDraft = ref("");
const projectMoreOpen = ref(false);

function startProjectInlineRename() {
  if (!selected.value) return;
  projectRenameDraft.value = selected.value.name;
  renamingProject.value = true;
  projectMoreOpen.value = false;
  requestAnimationFrame(() => {
    document.getElementById("project-inline-rename")?.focus();
  });
}

function cancelProjectInlineRename() {
  renamingProject.value = false;
  projectRenameDraft.value = "";
}

async function commitProjectInlineRename() {
  if (!selected.value || !renamingProject.value) return;
  const next = projectRenameDraft.value.trim();
  renamingProject.value = false;
  if (!next || next === selected.value.name) return;
  error.value = null;
  try {
    document.value = await invoke<UserProjects>("projects_rename", {
      projectId: selected.value.id,
      name: next,
    });
    toastSuccess(t("projects.renamed"));
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

async function bindSelectedWorkGroup() {
  if (!selected.value) return;
  error.value = null;
  try {
    document.value = await invoke<UserProjects>("projects_bind_work_group", {
      projectId: selected.value.id,
      workGroupId: bindWorkGroupId.value.trim() || null,
    });
    syncBindDraft();
    toastSuccess(
      bindWorkGroupId.value
        ? t("projects.noticeBound")
        : t("projects.noticeUnbound"),
    );
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

async function saveMembersFromTabs() {
  if (!selected.value?.workGroupId) {
    error.value = t("projects.errorUnbound");
    return;
  }
  error.value = null;
  try {
    await invoke("projects_save_members_from_open_state", {
      projectId: selected.value.id,
      liveOpenState: snapshotLiveOpenState(),
    });
    toastSuccess(t("projects.noticeMembersSaved"));
    await refresh();
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

const boundMemberPaths = computed(() => {
  const id = selected.value?.workGroupId;
  if (!id) return [] as string[];
  return workGroups.value.find((group) => group.id === id)?.memberPaths ?? [];
});

async function openBoundMembersIntoRuntime() {
  if (!selected.value?.workGroupId) {
    error.value = t("projects.errorUnbound");
    return;
  }
  error.value = null;
  const paths = boundMemberPaths.value;
  if (!paths.length) {
    toastInfo(t("workGroups.openMembersEmpty"));
    return;
  }
  const opened = await openMembersIntoOpenState(paths);
  if (opened.length) {
    toastSuccess(t("workGroups.openMembersDone", { count: opened.length }));
    await router.push("/navigator");
  } else {
    toastInfo(t("workGroups.openMembersNone"));
  }
}

async function deleteSelected() {
  if (!selected.value) return;
  if (!window.confirm(t("projects.confirmDelete", { name: selected.value.name }))) return;
  document.value = await invoke<UserProjects>("projects_delete", {
    projectId: selected.value.id,
  });
  selectedId.value = document.value.activeProjectId;
  syncContextDraft();
}

async function addLink() {
  if (!selected.value || !linkPath.value.trim()) return;
  document.value = await invoke<UserProjects>("projects_add_workspace_link", {
    projectId: selected.value.id,
    path: linkPath.value.trim(),
    label: null,
  });
  linkPath.value = "";
}

async function removeLink(linkId: string) {
  if (!selected.value) return;
  document.value = await invoke<UserProjects>("projects_remove_workspace_link", {
    projectId: selected.value.id,
    linkId,
  });
}

async function saveContext() {
  if (!selected.value) return;
  document.value = await invoke<UserProjects>("projects_set_context_notes", {
    projectId: selected.value.id,
    contextNotes: contextDraft.value,
  });
  toastSuccess(t("projects.noticeContextSaved"));
}

async function addTask() {
  if (!selected.value || !taskTitle.value.trim()) return;
  const [doc] = await invoke<[UserProjects, ProjectTask]>("projects_add_task", {
    projectId: selected.value.id,
    title: taskTitle.value.trim(),
    statusId: "backlog",
  });
  document.value = doc;
  taskTitle.value = "";
}

async function moveTask(task: ProjectTask, statusId: string) {
  if (!selected.value) return;
  const peers = tasksForStatus(statusId);
  const sortKey = peers.length ? peers[peers.length - 1].sortKey + 1 : 0;
  document.value = await invoke<UserProjects>("projects_move_task", {
    projectId: selected.value.id,
    taskId: task.id,
    statusId,
    sortKey,
  });
}

async function deleteTask(taskId: string) {
  if (!selected.value) return;
  document.value = await invoke<UserProjects>("projects_delete_task", {
    projectId: selected.value.id,
    taskId,
  });
}

async function openAllLinks() {
  if (!selected.value) return;
  error.value = null;
  const result = await invoke<OpenAllResult>("projects_open_all_links", {
    projectId: selected.value.id,
  });
  if (result.workGroupMutated) {
    error.value = t("projects.errorWorkGroupMutation");
    return;
  }
  if (!result.paths.length) {
    toastInfo(t("projects.noticeNoLinks"));
    return;
  }
  await openPathsAsNavigatorTabs(result.paths);
  toastSuccess(t("projects.noticeOpened", { count: result.paths.length }));
  await router.push("/navigator");
}

async function openLink(path: string) {
  // Sigma openOrFocusTabGroup: select existing page group or append one.
  await openOrFocusNavigatorTab(path);
  await router.push("/navigator");
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <section class="fluent-page">
    <header class="fluent-page-header">
      <h1 class="fluent-page-title">{{ t("projects.title") }}</h1>
      <p class="fluent-page-meta">{{ t("projects.meta") }}</p>
      <p class="fluent-page-meta usage-hint">{{ t("projects.usageHint") }}</p>
    </header>

    <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>

    <div v-if="document" class="layout">
      <aside class="master-list">
        <button
          v-for="project in document.projects"
          :key="project.id"
          type="button"
          class="fluent-surface group"
          :class="{ 'group--active': project.id === selectedId }"
          @click="selectProject(project.id)"
        >
          <span class="group-head">
            <span class="fluent-icon-wash">
              <FluentIcon name="board" size="md" />
            </span>
            <strong>{{ project.name }}</strong>
          </span>
          <span class="fluent-page-meta">{{
            project.workGroupId
              ? t("projects.listSummaryBound", {
                  links: project.workspaceLinks.length,
                  open: project.tasks.filter((task) => task.statusId !== "done").length,
                })
              : t("projects.listSummary", {
                  links: project.workspaceLinks.length,
                  open: project.tasks.filter((task) => task.statusId !== "done").length,
                })
          }}</span>
        </button>

        <form class="create" @submit.prevent="createProject">
          <input
            v-model="newName"
            class="fluent-control"
            :placeholder="t('projects.newNamePlaceholder')"
          />
          <button class="fluent-control fluent-control--accent" type="submit">
            {{ t("common.create") }}
          </button>
        </form>
      </aside>

      <div v-if="selected" class="fluent-surface detail">
        <div class="detail-head">
          <div>
            <div class="detail-title-row">
              <input
                v-if="renamingProject"
                id="project-inline-rename"
                v-model="projectRenameDraft"
                class="fluent-control inline-rename"
                @keydown.enter.prevent="commitProjectInlineRename"
                @keydown.escape.prevent="cancelProjectInlineRename"
                @blur="commitProjectInlineRename"
              />
              <h2
                v-else
                class="inline-title"
                :title="t('projects.renameHint')"
                @dblclick="startProjectInlineRename"
              >
                {{ selected.name }}
              </h2>
              <button type="button" class="name-tag name-tag--danger" @click="deleteSelected">
                {{ t("common.delete") }}
              </button>
              <div class="more-menu">
                <button
                  type="button"
                  class="fluent-control more-trigger"
                  :aria-expanded="projectMoreOpen"
                  aria-haspopup="menu"
                  :title="t('common.more')"
                  @click="projectMoreOpen = !projectMoreOpen"
                >
                  ···
                </button>
                <div v-if="projectMoreOpen" class="more-popover fluent-surface" role="menu">
                  <button
                    type="button"
                    class="more-item"
                    role="menuitem"
                    @click="startProjectInlineRename"
                  >
                    {{ t("projects.rename") }}
                  </button>
                  <button
                    type="button"
                    class="more-item more-item--danger"
                    role="menuitem"
                    @click="
                      projectMoreOpen = false;
                      deleteSelected();
                    "
                  >
                    {{ t("common.delete") }}
                  </button>
                </div>
              </div>
            </div>
            <p class="fluent-page-meta">
              {{ selected.description || t("projects.noDescription") }} ·
              {{ t("projects.openTasks", { count: openTaskCount }) }}
            </p>
          </div>
          <div class="actions">
            <button class="fluent-control fluent-control--accent" type="button" @click="openAllLinks">
              {{ t("projects.openAllLinks") }}
            </button>
          </div>
        </div>

        <h3>{{ t("projects.sessionBinding") }}</h3>
        <p class="fluent-page-meta">
          {{
            boundWorkGroupName
              ? t("projects.boundTo", { name: boundWorkGroupName })
              : t("projects.unbound")
          }}
        </p>
        <form class="row" @submit.prevent="bindSelectedWorkGroup">
          <select v-model="bindWorkGroupId" class="fluent-control">
            <option value="">{{ t("projects.bindNone") }}</option>
            <option
              v-for="group in bindableWorkGroups"
              :key="group.id"
              :value="group.id"
            >
              {{ group.name }}
            </option>
          </select>
          <button class="fluent-control fluent-control--accent" type="submit">
            {{ t("projects.bindSave") }}
          </button>
          <button
            v-if="selected.workGroupId"
            class="fluent-control fluent-control--accent"
            type="button"
            :disabled="!boundMemberPaths.length"
            @click="openBoundMembersIntoRuntime"
          >
            {{ t("projects.openMembersIntoOpenState") }}
          </button>
          <button
            v-if="selected.workGroupId"
            class="fluent-control"
            type="button"
            @click="saveMembersFromTabs"
          >
            {{ t("projects.saveMembersFromTabs") }}
          </button>
        </form>

        <h3>{{ t("projects.workspaceLinks") }}</h3>
        <div v-if="selected.workspaceLinks.length" class="path-card-grid">
          <div
            v-for="link in selected.workspaceLinks"
            :key="link.id"
            class="fluent-surface path-card path-card--with-action"
          >
            <button
              type="button"
              class="path-card-main"
              :title="link.path"
              @click="openLink(link.path)"
            >
              <span class="fluent-icon-wash">
                <FluentIcon name="folder" size="md" />
              </span>
              <span class="path-card-body">
                <span class="path-card-name">{{ link.label || link.path }}</span>
                <span class="fluent-page-meta path-card-path">{{ link.path }}</span>
              </span>
            </button>
            <button
              type="button"
              class="fluent-control path-card-remove"
              :title="t('common.remove')"
              :aria-label="t('common.remove')"
              @click="removeLink(link.id)"
            >
              {{ t("common.remove") }}
            </button>
          </div>
        </div>
        <p v-else class="fluent-page-meta">{{ t("projects.noFolderLinks") }}</p>
        <form class="row" @submit.prevent="addLink">
          <input
            v-model="linkPath"
            class="fluent-control"
            :placeholder="t('projects.addLinkPlaceholder')"
          />
          <button class="fluent-control fluent-control--accent" type="submit">
            {{ t("projects.addLink") }}
          </button>
        </form>

        <h3>{{ t("projects.contextNotes") }}</h3>
        <textarea
          v-model="contextDraft"
          class="fluent-control context"
          rows="4"
          :placeholder="t('projects.contextPlaceholder')"
        />
        <button class="fluent-control" type="button" @click="saveContext">
          {{ t("projects.saveContext") }}
        </button>

        <div class="section-title-row">
          <h3>{{ t("projects.outcomes") }}</h3>
          <div class="toggle" role="group" :aria-label="t('projects.viewMode')">
            <button
              type="button"
              class="fluent-control"
              :class="{ 'fluent-control--accent': viewMode === 'list' }"
              @click="viewMode = 'list'"
            >
              {{ t("projects.list") }}
            </button>
            <button
              type="button"
              class="fluent-control"
              :class="{ 'fluent-control--accent': viewMode === 'board' }"
              @click="viewMode = 'board'"
            >
              {{ t("projects.board") }}
            </button>
          </div>
        </div>
        <form class="row" @submit.prevent="addTask">
          <input
            v-model="taskTitle"
            class="fluent-control"
            :placeholder="t('projects.addOutcomePlaceholder')"
          />
          <button class="fluent-control fluent-control--accent" type="submit">
            {{ t("projects.addTask") }}
          </button>
        </form>

        <div v-if="viewMode === 'list'" class="task-list">
          <article
            v-for="task in selected.tasks.slice().sort((a, b) => a.sortKey - b.sortKey)"
            :key="task.id"
            class="task-row"
          >
            <strong>{{ task.title }}</strong>
            <select
              class="fluent-control"
              :value="task.statusId"
              @change="moveTask(task, ($event.target as HTMLSelectElement).value)"
            >
              <option v-for="status in STATUSES" :key="status.id" :value="status.id">
                {{ status.label }}
              </option>
            </select>
            <button type="button" class="linkish danger" @click="deleteTask(task.id)">
              {{ t("common.delete") }}
            </button>
          </article>
          <p v-if="!selected.tasks.length" class="fluent-page-meta">{{ t("projects.noOutcomes") }}</p>
        </div>

        <div v-else class="board">
          <section
            v-for="status in STATUSES"
            :key="status.id"
            class="column"
            :class="`column--${status.id}`"
          >
            <h4 class="column-title">
              <FluentIcon :name="STATUS_ICONS[status.id] ?? 'board'" size="sm" />
              {{ status.label }}
            </h4>
            <article
              v-for="task in tasksForStatus(status.id)"
              :key="task.id"
              class="fluent-surface card"
            >
              <strong>{{ task.title }}</strong>
              <div class="card-actions">
                <select
                  class="fluent-control"
                  :value="task.statusId"
                  @change="moveTask(task, ($event.target as HTMLSelectElement).value)"
                >
                  <option v-for="s in STATUSES" :key="s.id" :value="s.id">{{ s.label }}</option>
                </select>
                <button type="button" class="linkish danger" @click="deleteTask(task.id)">
                  {{ t("common.delete") }}
                </button>
              </div>
            </article>
            <p v-if="!tasksForStatus(status.id).length" class="fluent-page-meta empty">
              {{ t("common.empty") }}
            </p>
          </section>
        </div>
      </div>

      <div v-else class="fluent-surface detail detail--empty">
        <p class="fluent-page-meta">{{ t("projects.emptyStart") }}</p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.linkish {
  border: 0;
  background: transparent;
  color: var(--fluent-color-fill-accent);
  cursor: pointer;
  font: inherit;
}

.linkish.danger {
  color: var(--fluent-color-status-danger);
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
  box-shadow: inset 3px 0 0 var(--fluent-color-stroke-accent-soft);
}

.create,
.row,
.actions,
.toggle,
.card-actions {
  display: flex;
  gap: var(--fluent-space-sm);
  flex-wrap: wrap;
  align-items: center;
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

.detail--empty {
  align-items: center;
  justify-content: center;
  min-height: 12rem;
}

.detail-head {
  display: flex;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  flex-wrap: wrap;
}

.detail-title-row {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  flex-wrap: wrap;
}

.inline-title {
  margin: 0;
  cursor: text;
}

.inline-rename {
  min-width: 12rem;
  font-size: var(--fluent-font-size-title);
  font-weight: var(--fluent-font-weight-semibold);
}

.name-tag {
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: 999px;
  padding: 2px 10px;
  background: var(--fluent-color-fill-subtle);
  color: var(--fluent-color-text-secondary);
  font: inherit;
  font-size: var(--fluent-font-size-caption);
  cursor: pointer;
}

.name-tag--danger {
  color: var(--fluent-color-status-danger);
  border-color: color-mix(in srgb, var(--fluent-color-status-danger) 35%, transparent);
  background: var(--fluent-color-status-danger-bg);
}

.more-menu {
  position: relative;
  margin-inline-start: auto;
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

.section-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  flex-wrap: wrap;
}

.detail h2,
.detail h3,
.detail h4 {
  margin: 0;
}

.usage-hint {
  max-width: 42rem;
}

.context {
  width: 100%;
  min-height: 6rem;
  resize: vertical;
  font: inherit;
}

.task-list {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.task-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
  align-items: center;
  padding: var(--fluent-space-sm) 0;
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.board {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--fluent-space-md);
}

.column {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  min-height: 12rem;
  padding: var(--fluent-space-sm);
  border-radius: var(--fluent-radius-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-layer-alt);
}

.column--backlog {
  background: color-mix(in srgb, var(--fluent-color-fill-layer-alt) 85%, var(--fluent-color-fill-subtle));
}

.column--doing {
  background: color-mix(in srgb, var(--fluent-color-status-info-bg) 45%, var(--fluent-color-fill-layer-alt));
  border-color: color-mix(in srgb, var(--fluent-color-status-info) 35%, var(--fluent-color-stroke-divider));
}

.column--review {
  background: color-mix(in srgb, var(--fluent-color-status-warning-bg) 40%, var(--fluent-color-fill-layer-alt));
  border-color: color-mix(in srgb, var(--fluent-color-status-warning) 30%, var(--fluent-color-stroke-divider));
}

.column--done {
  background: color-mix(in srgb, var(--fluent-color-status-success-bg) 40%, var(--fluent-color-fill-layer-alt));
  border-color: color-mix(in srgb, var(--fluent-color-status-success) 30%, var(--fluent-color-stroke-divider));
}

.column-title {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
}

.card {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-md);
}

.empty {
  opacity: 0.7;
}

@media (max-width: 960px) {
  .layout {
    grid-template-columns: 1fr;
  }

  .board {
    grid-template-columns: 1fr 1fr;
  }
}
</style>
