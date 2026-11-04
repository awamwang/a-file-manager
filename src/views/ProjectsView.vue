<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { openPathsAsNavigatorTabs } from "../state/navigatorBrowse";

const { t } = useI18n();

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

type OpenAllResult = {
  projectId: string;
  paths: string[];
  workGroupMutated: boolean;
};

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
const selectedId = ref<string | null>(null);
const viewMode = ref<"list" | "board">("board");
const error = ref<string | null>(null);
const notice = ref<string | null>(null);
const loading = ref(false);

const newName = ref("");
const linkPath = ref("");
const taskTitle = ref("");
const contextDraft = ref("");

const selected = computed(
  () => document.value?.projects.find((p) => p.id === selectedId.value) ?? null,
);

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
    document.value = await invoke<UserProjects>("projects_get");
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
    syncContextDraft();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    loading.value = false;
  }
}

function syncContextDraft() {
  contextDraft.value = selected.value?.contextNotes ?? "";
}

function selectProject(id: string) {
  selectedId.value = id;
  syncContextDraft();
  void invoke("projects_set_active", { projectId: id }).then((doc) => {
    document.value = doc as UserProjects;
  });
}

async function createProject() {
  document.value = await invoke<UserProjects>("projects_create", {
    name: newName.value.trim() || null,
    description: null,
  });
  newName.value = "";
  selectedId.value =
    document.value.activeProjectId ??
    document.value.projects[document.value.projects.length - 1]?.id ??
    null;
  syncContextDraft();
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
  notice.value = t("projects.noticeContextSaved");
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
  notice.value = null;
  const result = await invoke<OpenAllResult>("projects_open_all_links", {
    projectId: selected.value.id,
  });
  if (result.workGroupMutated) {
    error.value = t("projects.errorWorkGroupMutation");
    return;
  }
  if (!result.paths.length) {
    notice.value = t("projects.noticeNoLinks");
    return;
  }
  await openPathsAsNavigatorTabs(result.paths);
  notice.value = t("projects.noticeOpened", { count: result.paths.length });
  await router.push("/navigator");
}

async function openLink(path: string) {
  await openPathsAsNavigatorTabs([path]);
  await router.push("/navigator");
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <section class="panel">
    <header class="header">
      <h1>{{ t("projects.title") }}</h1>
      <p class="meta">{{ t("projects.meta") }}</p>
    </header>

    <p v-if="loading" class="banner">{{ t("common.loading") }}</p>
    <p v-else-if="error" class="banner banner--error">{{ error }}</p>
    <p v-if="notice" class="banner banner--soft">
      {{ notice }}
      <button type="button" class="linkish" @click="notice = null">{{ t("common.dismiss") }}</button>
    </p>

    <div v-if="document" class="layout">
      <aside class="list">
        <button
          v-for="project in document.projects"
          :key="project.id"
          type="button"
          class="group"
          :class="{ 'group--active': project.id === selectedId }"
          @click="selectProject(project.id)"
        >
          <strong>{{ project.name }}</strong>
          <span>{{
            t("projects.listSummary", {
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

      <div v-if="selected" class="detail">
        <div class="detail-head">
          <div>
            <h2>{{ selected.name }}</h2>
            <p class="meta">
              {{ selected.description || t("projects.noDescription") }} ·
              {{ t("projects.openTasks", { count: openTaskCount }) }}
            </p>
          </div>
          <div class="actions">
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
            <button class="fluent-control fluent-control--accent" type="button" @click="openAllLinks">
              {{ t("projects.openAllLinks") }}
            </button>
            <button class="fluent-control" type="button" @click="deleteSelected">
              {{ t("common.delete") }}
            </button>
          </div>
        </div>

        <h3>{{ t("projects.workspaceLinks") }}</h3>
        <ul class="links">
          <li v-for="link in selected.workspaceLinks" :key="link.id">
            <button type="button" class="linkish" @click="openLink(link.path)">
              {{ link.label }}
            </button>
            <span class="meta path">{{ link.path }}</span>
            <button type="button" class="linkish danger" @click="removeLink(link.id)">
              {{ t("common.remove") }}
            </button>
          </li>
          <li v-if="!selected.workspaceLinks.length" class="meta">
            {{ t("projects.noFolderLinks") }}
          </li>
        </ul>
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

        <h3>{{ t("projects.outcomes") }}</h3>
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
          <p v-if="!selected.tasks.length" class="meta">{{ t("projects.noOutcomes") }}</p>
        </div>

        <div v-else class="board">
          <section v-for="status in STATUSES" :key="status.id" class="column">
            <h4>{{ status.label }}</h4>
            <article v-for="task in tasksForStatus(status.id)" :key="task.id" class="card">
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
            <p v-if="!tasksForStatus(status.id).length" class="meta empty">
              {{ t("common.empty") }}
            </p>
          </section>
        </div>
      </div>

      <div v-else class="detail detail--empty">
        <p class="meta">{{ t("projects.emptyStart") }}</p>
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

.linkish.danger {
  color: var(--fluent-color-status-danger);
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
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
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

.detail h2,
.detail h3,
.detail h4 {
  margin: 0;
}

.links {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.links li {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
  align-items: center;
}

.path {
  flex: 1;
  min-width: 8rem;
  word-break: break-all;
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
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
}

.card {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-sm);
  background: var(--fluent-color-fill-control);
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
