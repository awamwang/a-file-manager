<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * AIA-213 / C7 — Projects UI talks only to Workspace disk APIs (C1–C6).
 * Legacy projects.json is no longer the authority.
 */
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import {
  applyProjectSilentSwitch,
  openMembersIntoOpenState,
  openOrFocusNavigatorTab,
  openPathsAsNavigatorTabs,
  snapshotLeavingOpenState,
  snapshotLiveOpenState,
  type SilentSwitchResult,
} from "../state/navigatorBrowse";
import { openConfirmDialog } from "../state/confirmDialog";
import { toastInfo, toastSuccess } from "../state/toast";
import FluentIcon from "../components/icons/FluentIcon.vue";
import type { FluentGlyphId } from "../components/icons/glyphs";
import { AI_ADDON_ENABLED } from "../config/flavor";

const { t } = useI18n();
const aiAddonEnabled = AI_ADDON_ENABLED;

const STATUS_ICONS: Record<string, FluentGlyphId> = {
  backlog: "board",
  doing: "flash",
  review: "eye",
  done: "check",
};

/** Member junction surfaced as a path card (openPath = real target). */
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
  createdAt: number;
  updatedAt: number;
  completedAt: number | null;
};

/** View-model for one disk Workspace (id === slug). */
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
  path: string;
};

type UserProjects = {
  projects: UserProject[];
  activeProjectId: string | null;
};

type WorkGroupOption = {
  id: string;
  name: string;
  isDefault: boolean;
  memberPaths?: string[];
};

type WorkspaceListResult = {
  projectsRoot: string;
  workspaces: Array<{
    path: string;
    project: {
      slug: string;
      displayName: string;
      description: string;
      boundWorkGroupId?: string | null;
      createdAt: number;
      updatedAt: number;
    };
  }>;
  activeSlug: string | null;
};

type WorkspaceMember = {
  name: string;
  junctionPath: string;
  targetPath: string;
  openPath: string;
};

type WorkspaceBoard = {
  schemaVersion: number;
  columns: string[];
  tasks: ProjectTask[];
};

type ActivateWorkspaceResult = {
  workspace: { path: string; project: WorkspaceListResult["workspaces"][0]["project"] };
  index: { activeProject?: { slug: string } | null };
  silentSwitch: SilentSwitchResult | null;
};

const PROJECT_ERROR_KEYS: Record<string, string> = {
  PROJECT_DEFAULT_WORK_GROUP_FORBIDDEN: "projects.errorDefaultWorkGroup",
  PROJECT_UNKNOWN_WORK_GROUP: "projects.errorUnknownWorkGroup",
  PROJECT_UNBOUND: "projects.errorUnbound",
  PROJECT_WORKSPACE_NOT_FOUND: "projects.errorUnknownWorkGroup",
  PROJECT_SLUG_CONFLICT: "projects.errorUnknownWorkGroup",
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

function mapListItem(
  item: WorkspaceListResult["workspaces"][0],
  extras?: Partial<Pick<UserProject, "workspaceLinks" | "tasks" | "contextNotes">>,
): UserProject {
  const p = item.project;
  return {
    id: p.slug,
    name: p.displayName,
    description: p.description,
    workGroupId: p.boundWorkGroupId ?? null,
    workspaceLinks: extras?.workspaceLinks ?? [],
    tasks: extras?.tasks ?? [],
    contextNotes: extras?.contextNotes ?? "",
    createdAt: p.createdAt,
    updatedAt: p.updatedAt,
    path: item.path,
  };
}

function membersToLinks(members: WorkspaceMember[]): WorkspaceLink[] {
  return members.map((m) => ({
    id: m.name,
    path: m.openPath,
    label: m.name,
    kind: "folder",
    addedAt: 0,
  }));
}

async function loadProjectDetails(slug: string): Promise<{
  workspaceLinks: WorkspaceLink[];
  tasks: ProjectTask[];
  contextNotes: string;
}> {
  const [members, board, contextNotes] = await Promise.all([
    invoke<WorkspaceMember[]>("projects_workspace_member_list", { slug }),
    invoke<WorkspaceBoard>("projects_workspace_board_get", { slug }),
    invoke<string>("projects_workspace_context_get", { slug }),
  ]);
  return {
    workspaceLinks: membersToLinks(members),
    tasks: board.tasks ?? [],
    contextNotes,
  };
}

async function patchSelectedDetails(slug: string) {
  if (!document.value) return;
  const details = await loadProjectDetails(slug);
  const idx = document.value.projects.findIndex((p) => p.id === slug);
  if (idx < 0) return;
  document.value.projects[idx] = {
    ...document.value.projects[idx],
    ...details,
  };
}

async function refresh(preferSlug?: string | null) {
  loading.value = true;
  error.value = null;
  try {
    const [list, wgDoc] = await Promise.all([
      invoke<WorkspaceListResult>("projects_workspace_list"),
      invoke<{ workGroups: WorkGroupOption[] }>("work_groups_get"),
    ]);
    workGroups.value = wgDoc.workGroups;
    const projects = list.workspaces.map((w) => mapListItem(w));
    document.value = {
      projects,
      activeProjectId: list.activeSlug,
    };
    const prefer = preferSlug ?? selectedId.value;
    if (prefer && projects.some((p) => p.id === prefer)) {
      selectedId.value = prefer;
    } else {
      selectedId.value = list.activeSlug ?? projects[0]?.id ?? null;
    }
    if (selectedId.value) {
      await patchSelectedDetails(selectedId.value);
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
  try {
    const result = await invoke<ActivateWorkspaceResult>("projects_workspace_activate", {
      slug: id,
      liveOpenState: snapshotLeavingOpenState(),
    });
    if (document.value) {
      document.value.activeProjectId = result.index.activeProject?.slug ?? id;
    }
    await patchSelectedDetails(id);
    syncContextDraft();
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
    const created = await invoke<{ project: { slug: string } }>("projects_workspace_create", {
      displayName: newName.value.trim() || "Project",
      description: null,
      slug: null,
      workGroupId: null,
    });
    const slug = created.project.slug;
    newName.value = "";
    // Refresh list then select/activate so rename & detail edit target the new project (AIA-247).
    await refresh(slug);
    await selectProject(slug);
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

const renamingProject = ref(false);
const projectRenameDraft = ref("");
const projectMoreOpen = ref(false);
const projectRenameInputEl = ref<HTMLInputElement | null>(null);
const projectMoreMenuEl = ref<HTMLElement | null>(null);
const projectMoreTriggerEl = ref<HTMLElement | null>(null);
const projectTitleRowEl = ref<HTMLElement | null>(null);
const listMoreOpenId = ref<string | null>(null);
const listMoreMenuEl = ref<HTMLElement | null>(null);
const listMoreMenuStyle = ref<Record<string, string>>({});
const projectMoreMenuStyle = ref<Record<string, string>>({});
const listRenamingId = ref<string | null>(null);
const listRenameDraft = ref("");
const listRenameInputEl = ref<HTMLInputElement | null>(null);
const masterListEl = ref<HTMLElement | null>(null);

const listMoreProject = computed(
  () => document.value?.projects.find((p) => p.id === listMoreOpenId.value) ?? null,
);

function positionMorePopover(
  anchor: HTMLElement,
  styleRef: typeof listMoreMenuStyle,
) {
  const rect = anchor.getBoundingClientRect();
  const menuWidth = Math.min(160, window.innerWidth * 0.7);
  let left = rect.right - menuWidth;
  if (left < 8) left = 8;
  if (left + menuWidth > window.innerWidth - 8) {
    left = Math.max(8, window.innerWidth - menuWidth - 8);
  }
  styleRef.value = {
    position: "fixed",
    top: `${Math.round(rect.bottom + 4)}px`,
    left: `${Math.round(left)}px`,
    zIndex: "1200",
  };
}

function setListRenameInputEl(el: unknown) {
  listRenameInputEl.value = el instanceof HTMLInputElement ? el : null;
}

function startProjectInlineRename() {
  if (!selected.value) return;
  projectRenameDraft.value = selected.value.name;
  renamingProject.value = true;
  projectMoreOpen.value = false;
  listMoreOpenId.value = null;
  listRenamingId.value = null;
  void nextTick(() => {
    projectRenameInputEl.value?.focus();
    projectRenameInputEl.value?.select();
  });
}

function cancelProjectInlineRename() {
  renamingProject.value = false;
  projectRenameDraft.value = "";
}

function onProjectRenameBlur() {
  // Defer so Enter-commit can clear `renamingProject` first in the same turn.
  queueMicrotask(() => {
    if (renamingProject.value) cancelProjectInlineRename();
  });
}

async function commitProjectInlineRename() {
  if (!selected.value || !renamingProject.value) return;
  const next = projectRenameDraft.value.trim();
  renamingProject.value = false;
  projectRenameDraft.value = "";
  if (!next || next === selected.value.name) return;
  error.value = null;
  try {
    await invoke("projects_workspace_rename", {
      slug: selected.value.id,
      displayName: next,
      newSlug: null,
    });
    toastSuccess(t("projects.renamed"));
    await refresh(selected.value.id);
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

function toggleListMore(projectId: string, event: Event) {
  event.stopPropagation();
  projectMoreOpen.value = false;
  if (listMoreOpenId.value === projectId) {
    listMoreOpenId.value = null;
    return;
  }
  const anchor =
    event.currentTarget instanceof HTMLElement ? event.currentTarget : null;
  if (anchor) positionMorePopover(anchor, listMoreMenuStyle);
  listMoreOpenId.value = projectId;
}

function toggleProjectDetailMore() {
  listMoreOpenId.value = null;
  if (projectMoreOpen.value) {
    projectMoreOpen.value = false;
    return;
  }
  if (projectMoreTriggerEl.value) {
    positionMorePopover(projectMoreTriggerEl.value, projectMoreMenuStyle);
  }
  projectMoreOpen.value = true;
}

function startListRename(project: UserProject, event?: Event) {
  event?.stopPropagation();
  listMoreOpenId.value = null;
  projectMoreOpen.value = false;
  renamingProject.value = false;
  listRenamingId.value = project.id;
  listRenameDraft.value = project.name;
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

async function commitListRename(projectId: string) {
  if (listRenamingId.value !== projectId) return;
  const project = document.value?.projects.find((p) => p.id === projectId);
  const next = listRenameDraft.value.trim();
  listRenamingId.value = null;
  listRenameDraft.value = "";
  if (!project || !next || next === project.name) return;
  error.value = null;
  try {
    await invoke("projects_workspace_rename", {
      slug: projectId,
      displayName: next,
      newSlug: null,
    });
    toastSuccess(t("projects.renamed"));
    await refresh(projectId);
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

function onProjectDetailPointerDown(event: PointerEvent) {
  const target = event.target as Node | null;
  if (!target) return;
  if (projectMoreOpen.value) {
    const inMenu = projectMoreMenuEl.value?.contains(target);
    const inTrigger = projectMoreTriggerEl.value?.contains(target);
    if (!inMenu && !inTrigger) projectMoreOpen.value = false;
  }
  if (
    renamingProject.value &&
    projectTitleRowEl.value &&
    !projectTitleRowEl.value.contains(target)
  ) {
    cancelProjectInlineRename();
  }
  if (listMoreOpenId.value) {
    const trigger = masterListEl.value?.querySelector(
      `[data-list-more-id="${listMoreOpenId.value}"]`,
    );
    const inMenu = listMoreMenuEl.value?.contains(target);
    if (!inMenu && !trigger?.contains(target)) {
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

async function bindSelectedWorkGroup() {
  if (!selected.value) return;
  error.value = null;
  try {
    await invoke("projects_workspace_bind_work_group", {
      slug: selected.value.id,
      workGroupId: bindWorkGroupId.value.trim() || null,
    });
    await refresh(selected.value.id);
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
    // Session members still live on the bound Work Group (not projects.json).
    await invoke("work_groups_save_members_from_open_state", {
      workGroupId: selected.value.workGroupId,
      liveOpenState: snapshotLiveOpenState(),
    });
    toastSuccess(t("projects.noticeMembersSaved"));
    await refresh(selected.value.id);
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

async function copyProject(project: UserProject) {
  listMoreOpenId.value = null;
  projectMoreOpen.value = false;
  error.value = null;
  try {
    const result = await invoke<{
      workspace: { project: { slug: string; displayName: string } };
      copiedWorkGroup?: { id: string; name: string } | null;
    }>("projects_workspace_copy", { slug: project.id });
    const slug = result.workspace.project.slug;
    toastSuccess(
      result.copiedWorkGroup
        ? t("projects.copiedWithWorkGroup", {
            name: result.workspace.project.displayName,
            workGroup: result.copiedWorkGroup.name,
          })
        : t("projects.copied", { name: result.workspace.project.displayName }),
    );
    await refresh(slug);
    await selectProject(slug);
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

async function deleteProject(project: UserProject) {
  const confirmed = await openConfirmDialog({
    title: t("dialog.deleteTitle"),
    message: t("projects.confirmDelete", { name: project.name }),
    confirmLabel: t("common.delete"),
    cancelLabel: t("common.cancel"),
    danger: true,
  });
  if (!confirmed.confirmed) return;
  listMoreOpenId.value = null;
  projectMoreOpen.value = false;
  error.value = null;
  try {
    const result = await invoke<{ deletedSlug: string; index: { activeProject?: { slug: string } | null } }>(
      "projects_workspace_delete",
      { slug: project.id },
    );
    selectedId.value = result.index.activeProject?.slug ?? null;
    await refresh(selectedId.value);
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

async function deleteSelected() {
  if (!selected.value) return;
  await deleteProject(selected.value);
}

async function addLink() {
  if (!selected.value || !linkPath.value.trim()) return;
  error.value = null;
  try {
    await invoke("projects_workspace_member_add", {
      slug: selected.value.id,
      targetPath: linkPath.value.trim(),
      linkName: null,
    });
    linkPath.value = "";
    await patchSelectedDetails(selected.value.id);
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

async function removeLink(linkId: string) {
  if (!selected.value) return;
  error.value = null;
  try {
    await invoke("projects_workspace_member_remove", {
      slug: selected.value.id,
      name: linkId,
    });
    await patchSelectedDetails(selected.value.id);
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

async function saveContext() {
  if (!selected.value) return;
  error.value = null;
  try {
    await invoke("projects_workspace_context_set", {
      slug: selected.value.id,
      content: contextDraft.value,
    });
    selected.value.contextNotes = contextDraft.value;
    toastSuccess(t("projects.noticeContextSaved"));
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

async function addTask() {
  if (!selected.value || !taskTitle.value.trim()) return;
  error.value = null;
  try {
    const [board] = await invoke<[WorkspaceBoard, ProjectTask]>(
      "projects_workspace_board_add_task",
      {
        slug: selected.value.id,
        title: taskTitle.value.trim(),
        statusId: "backlog",
      },
    );
    selected.value.tasks = board.tasks;
    taskTitle.value = "";
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

async function moveTask(task: ProjectTask, statusId: string) {
  if (!selected.value) return;
  const peers = tasksForStatus(statusId);
  const sortKey = peers.length ? peers[peers.length - 1].sortKey + 1 : 0;
  error.value = null;
  try {
    const board = await invoke<WorkspaceBoard>("projects_workspace_board_move_task", {
      slug: selected.value.id,
      taskId: task.id,
      statusId,
      sortKey,
    });
    selected.value.tasks = board.tasks;
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

async function deleteTask(taskId: string) {
  if (!selected.value) return;
  error.value = null;
  try {
    const board = await invoke<WorkspaceBoard>("projects_workspace_board_delete_task", {
      slug: selected.value.id,
      taskId,
    });
    selected.value.tasks = board.tasks;
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

async function openAllLinks() {
  if (!selected.value) return;
  error.value = null;
  try {
    const members = await invoke<WorkspaceMember[]>("projects_workspace_member_list", {
      slug: selected.value.id,
    });
    const paths = members.map((m) => m.openPath);
    if (!paths.length) {
      toastInfo(t("projects.noticeNoLinks"));
      return;
    }
    await openPathsAsNavigatorTabs(paths);
    toastSuccess(t("projects.noticeOpened", { count: paths.length }));
    await router.push("/navigator");
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

/**
 * AIA-253 — open the Project Workspace folder on disk (OS Explorer).
 * Prefer `open_path_default` over plugin-opener for arbitrary FS paths (ACL).
 */
async function openWorkspaceFolder() {
  if (!selected.value?.path) return;
  error.value = null;
  try {
    const result = await invoke<{ success: boolean; error: string | null }>(
      "open_path_default",
      { path: selected.value.path },
    );
    if (!result.success) {
      error.value = result.error || t("projects.openWorkspaceFailed");
      return;
    }
    toastSuccess(t("projects.noticeOpenedWorkspace"));
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

/** AIA-221 / A8 — Cursor opens Workspace dir; Qoder opens CONTEXT.md. */
async function openInEditor(ideId: "cursor" | "qoder") {
  if (!selected.value?.path) return;
  error.value = null;
  try {
    const report = await invoke<{
      ideId: string;
      openedPath: string;
      mode: string;
      success: boolean;
      error: string | null;
    }>("ai_project_open_editor", {
      workspacePath: selected.value.path,
      ideId,
    });
    if (!report.success) {
      error.value = report.error || t("projects.openEditorFailed");
      return;
    }
    toastSuccess(
      t("projects.noticeOpenedEditor", {
        ide: ideId === "qoder" ? "Qoder" : "Cursor",
        mode: report.mode,
      }),
    );
  } catch (err) {
    error.value = mapProjectError(err);
  }
}

async function openLink(path: string) {
  // Sigma openOrFocusTabGroup: select existing page group or append one.
  await openOrFocusNavigatorTab(path);
  await router.push("/navigator");
}

onMounted(() => {
  void refresh();
  window.addEventListener("pointerdown", onProjectDetailPointerDown, true);
});

onUnmounted(() => {
  window.removeEventListener("pointerdown", onProjectDetailPointerDown, true);
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
      <aside ref="masterListEl" class="master-list">
        <div class="master-list-scroll">
          <div
            v-for="project in document.projects"
            :key="project.id"
            class="fluent-surface group"
            :class="{ 'group--active': project.id === selectedId }"
            role="button"
            tabindex="0"
            :data-list-item-id="project.id"
            @click="selectProject(project.id)"
            @keydown.enter.prevent="selectProject(project.id)"
          >
            <div class="group-top">
              <span class="group-head">
                <span class="fluent-icon-wash">
                  <FluentIcon name="board" size="md" />
                </span>
                <input
                  v-if="listRenamingId === project.id"
                  :ref="setListRenameInputEl"
                  v-model="listRenameDraft"
                  class="fluent-control list-inline-rename"
                  @click.stop
                  @keydown.enter.prevent="commitListRename(project.id)"
                  @keydown.escape.prevent="cancelListRename"
                  @blur="onListRenameBlur"
                />
                <strong v-else>{{ project.name }}</strong>
              </span>
              <div
                class="list-more-menu"
                :data-list-more-id="project.id"
                @click.stop
              >
                <button
                  type="button"
                  class="fluent-control more-trigger list-more-trigger"
                  :aria-expanded="listMoreOpenId === project.id"
                  aria-haspopup="menu"
                  :title="t('common.more')"
                  @click="toggleListMore(project.id, $event)"
                >
                  ···
                </button>
              </div>
            </div>
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
          </div>
        </div>

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
        <div ref="projectTitleRowEl" class="detail-title-row">
          <input
            v-if="renamingProject"
            ref="projectRenameInputEl"
            v-model="projectRenameDraft"
            class="fluent-control inline-rename"
            @keydown.enter.prevent="commitProjectInlineRename"
            @keydown.escape.prevent="cancelProjectInlineRename"
            @blur="onProjectRenameBlur"
          />
          <h2
            v-else
            class="inline-title"
            :title="t('projects.renameHint')"
            @dblclick="startProjectInlineRename"
          >
            {{ selected.name }}
          </h2>
          <div class="more-menu">
            <button
              ref="projectMoreTriggerEl"
              type="button"
              class="fluent-control more-trigger"
              :aria-expanded="projectMoreOpen"
              aria-haspopup="menu"
              :title="t('common.more')"
              @click="toggleProjectDetailMore"
            >
              ···
            </button>
          </div>
        </div>
        <p class="fluent-page-meta">
          {{ selected.description || t("projects.noDescription") }} ·
          {{ t("projects.openTasks", { count: openTaskCount }) }}
        </p>
        <div class="actions">
          <button class="fluent-control fluent-control--accent" type="button" @click="openAllLinks">
            {{ t("projects.openAllLinks") }}
          </button>
          <button
            class="fluent-control"
            type="button"
            :title="selected.path || t('projects.openWorkspaceFolderHint')"
            @click="openWorkspaceFolder"
          >
            {{ t("projects.openWorkspaceFolder") }}
          </button>
          <button
            v-if="aiAddonEnabled"
            class="fluent-control"
            type="button"
            :title="t('projects.openInCursorHint')"
            @click="openInEditor('cursor')"
          >
            {{ t("projects.openInCursor") }}
          </button>
          <button
            v-if="aiAddonEnabled"
            class="fluent-control"
            type="button"
            :title="t('projects.openInQoderHint')"
            @click="openInEditor('qoder')"
          >
            {{ t("projects.openInQoder") }}
          </button>
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
              @click.stop="removeLink(link.id)"
            >
              <FluentIcon name="close" size="sm" />
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

    <!-- Portaled out of .fluent-surface { overflow: hidden } cards / scrollers. -->
    <Teleport to="body">
      <div
        v-if="listMoreProject"
        ref="listMoreMenuEl"
        class="more-popover more-popover--portal fluent-surface"
        role="menu"
        :style="listMoreMenuStyle"
        @click.stop
      >
        <button
          type="button"
          class="more-item"
          role="menuitem"
          @click="copyProject(listMoreProject)"
        >
          {{ t("projects.copy") }}
        </button>
        <button
          type="button"
          class="more-item"
          role="menuitem"
          @click="startListRename(listMoreProject, $event)"
        >
          {{ t("projects.rename") }}
        </button>
        <button
          type="button"
          class="more-item more-item--danger"
          role="menuitem"
          @click="deleteProject(listMoreProject)"
        >
          {{ t("common.delete") }}
        </button>
      </div>
    </Teleport>
    <Teleport to="body">
      <div
        v-if="projectMoreOpen && selected"
        ref="projectMoreMenuEl"
        class="more-popover more-popover--portal fluent-surface"
        role="menu"
        :style="projectMoreMenuStyle"
        @click.stop
      >
        <button
          type="button"
          class="more-item"
          role="menuitem"
          @click="copyProject(selected)"
        >
          {{ t("projects.copy") }}
        </button>
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
    </Teleport>
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

.fluent-page {
  height: 100%;
  min-height: 0;
  overflow: hidden;
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
.actions,
.toggle,
.card-actions {
  display: flex;
  gap: var(--fluent-space-sm);
  flex-wrap: wrap;
  align-items: center;
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

.detail--empty {
  align-items: center;
  justify-content: center;
  min-height: 12rem;
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
  margin-inline-start: auto;
  flex-shrink: 0;
}

.more-trigger {
  min-width: 2rem;
  padding-inline: var(--fluent-space-sm);
  letter-spacing: 0.08em;
}

.more-popover {
  min-width: 9rem;
  padding: var(--fluent-space-xs);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.more-popover--portal {
  /* position/top/left/z-index set via Teleport fixed inline style */
  box-shadow: var(--fluent-shadow-flyout, 0 8px 24px rgba(0, 0, 0, 0.18));
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
    overflow: auto;
  }

  .master-list {
    max-height: min(40vh, 20rem);
  }

  .board {
    grid-template-columns: 1fr 1fr;
  }
}
</style>
