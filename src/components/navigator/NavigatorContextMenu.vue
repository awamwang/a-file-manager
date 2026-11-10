<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { NavigatorEntry, UserWorkGroupsDoc } from "../../state/navigatorBrowse";
import {
  activeWorkGroupId,
  currentPath,
  hydrateWorkGroups,
  listing,
  navigateTo,
  newNavigatorTab,
  openEntry,
  rememberHydratedWorkGroups,
  restoreWorkGroupOpenState,
  snapshotLiveOpenState,
} from "../../state/navigatorBrowse";
import { workGroupDisplayName } from "../../state/workGroup";
import type { WorkGroupOpenState } from "../../state/openStateModel";
import {
  clipboardMode,
  clipboardPaths,
  createFile,
  createFolder,
  deleteEntries,
  hasClipboard,
  pathsWithoutRecycle,
  pasteClipboard,
  refreshClipboardFromSystem,
  setClipboard,
} from "../../state/fileClipboard";
import {
  recordCopyUndo,
  recordCreateUndo,
  recordDeleteUndo,
  recordMoveUndo,
} from "../../state/fileUndo";
import { isRecyclePath } from "../../state/recycle";
import { restoreRecyclePaths } from "../../state/recycle";
import { confirmDeleteIfNeeded, confirmPermanentDelete } from "../../state/confirmBeforeDelete";
import { openPromptDialog } from "../../state/promptDialog";
import {
  resolveNewFileDefaultName,
  resolveNewFolderDefaultName,
} from "../../state/newItemNames";
import {
  loadNewFileTemplates,
  newFileTemplates,
} from "../../state/newFileTemplates";
import { selectedEntries, selectedPaths } from "../../state/navigatorSelection";
import {
  isFavoritePath,
  toggleFavorite,
} from "../../state/favorites";
import FluentIcon from "../icons/FluentIcon.vue";
import {
  buildContextMenuItems,
  buildContextToolbarItems,
  canOpenInNewTab,
  type ShellEmbedMenuChild,
} from "./contextMenuModel";
import {
  invokeShellEmbedChild,
  loadShellEmbedChildren,
  shellEmbedLoadingPlaceholder,
} from "./shellMenuEmbed";
import { BUILTIN_SHORTCUTS, formatChordLabel } from "../../shortcuts/index.ts";

const props = defineProps<{
  entry: NavigatorEntry | null;
  x: number;
  y: number;
  open: boolean;
  emptyArea?: boolean;
}>();

const emit = defineEmits<{
  close: [];
  notice: [message: string];
  favoritesChanged: [];
  listingChanged: [];
  openProperties: [path: string];
  /** Parent starts Windows-style inline rename (no modal). */
  requestRename: [path: string];
}>();

const { t } = useI18n();
const root = ref<HTMLElement | null>(null);
const submenuRoot = ref<HTMLElement | null>(null);
const pos = ref({ left: 0, top: 0 });
const submenuPos = ref({ left: 0, top: 0 });
const openSubmenuId = ref<string | null>(null);
const submenuAnchor = ref<HTMLElement | null>(null);
let submenuCloseTimer: ReturnType<typeof setTimeout> | null = null;
/** AIA-239: shell verbs loaded into AFM submenu (no native TrackPopupMenu). */
const shellEmbedChildren = ref<ShellEmbedMenuChild[]>([]);
const shellEmbedPath = ref<string | null>(null);
const shellEmbedLoading = ref(false);
const shellEmbedError = ref<string | null>(null);
let shellEmbedLoadSeq = 0;
const ideApps = ref<Array<{ id: string; name: string; available: boolean }>>([]);
const terminals = ref<
  Array<{ id: string; name: string; available: boolean; supportsElevation: boolean }>
>([]);
let launchersWarm = false;
/** Cached WG list for “Add to Work Group” submenu (refreshed when menu opens). */
const workGroupsForMenu = ref<Array<{ id: string; name: string; isDefault: boolean }>>([]);

type SoftNotice = {
  kind: string;
  workGroupId: string;
  openedMemberPaths: string[];
};

type IdeAppInfo = {
  id: string;
  name: string;
  available: boolean;
  enabled?: boolean;
  resolvedPath?: string | null;
  resolution?: string;
  configuredPath?: string;
};
type TerminalAppInfo = {
  id: string;
  name: string;
  available: boolean;
  supportsElevation: boolean;
};

async function loadLaunchers() {
  try {
    ideApps.value = await invoke<IdeAppInfo[]>("ide_list_apps");
  } catch {
    ideApps.value = [];
  }
  try {
    terminals.value = await invoke<TerminalAppInfo[]>("terminal_list_apps");
  } catch {
    terminals.value = [];
  }
  launchersWarm = true;
}

/** Warm IDE/terminal lists so Open-with / Terminal submenus open instantly. */
function preloadLaunchersIfNeeded() {
  if (launchersWarm && (ideApps.value.length || terminals.value.length)) return;
  void loadLaunchers();
}

function childLabel(child: {
  labelKey: string;
  ideId?: string;
  terminalId?: string;
  elevated?: boolean;
  rawLabel?: string;
}): string {
  if (child.rawLabel) return child.rawLabel;
  if (child.ideId != null) {
    return ideApps.value.find((app) => app.id === child.ideId)?.name ?? child.ideId;
  }
  if (child.terminalId != null) {
    const name =
      terminals.value.find((app) => app.id === child.terminalId)?.name ?? child.terminalId;
    return child.elevated
      ? t("contextMenu.openInTerminalAdminNamed", { name })
      : name;
  }
  return t(child.labelKey);
}

const TOOLBAR_SHORTCUT_IDS: Record<string, string> = {
  cut: "navigator.cut",
  copy: "navigator.copy",
  paste: "navigator.paste",
  rename: "navigator.rename",
  copyPath: "navigator.copyPath",
};

const tip = ref<{ text: string; left: number; top: number } | null>(null);

const toolbarItems = computed(() =>
  buildContextToolbarItems({
    canPaste: hasClipboard.value,
    emptyArea: Boolean(props.emptyArea),
  }).map((item) => ({
    ...item,
    label: t(item.labelKey),
  })),
);

function toolbarTipText(item: { id: string; label: string }): string {
  const shortcutId = TOOLBAR_SHORTCUT_IDS[item.id];
  const def = shortcutId
    ? BUILTIN_SHORTCUTS.find((row) => row.id === shortcutId)
    : undefined;
  if (!def) return item.label;
  return `${item.label} (${formatChordLabel(def.chord)})`;
}

function showToolbarTip(event: Event, item: { id: string; label: string }) {
  const btn = event.currentTarget;
  if (!(btn instanceof HTMLElement)) return;
  const rect = btn.getBoundingClientRect();
  tip.value = {
    text: toolbarTipText(item),
    left: rect.left + rect.width / 2,
    top: rect.top - 8,
  };
}

function hideToolbarTip() {
  tip.value = null;
}

function resetShellEmbed() {
  shellEmbedChildren.value = [];
  shellEmbedPath.value = null;
  shellEmbedLoading.value = false;
  shellEmbedError.value = null;
  shellEmbedLoadSeq += 1;
}

async function ensureShellEmbedLoaded(path: string) {
  if (
    shellEmbedPath.value === path &&
    (shellEmbedChildren.value.length > 0 || shellEmbedLoading.value || shellEmbedError.value)
  ) {
    return;
  }
  const seq = ++shellEmbedLoadSeq;
  shellEmbedPath.value = path;
  shellEmbedLoading.value = true;
  shellEmbedError.value = null;
  shellEmbedChildren.value = shellEmbedLoadingPlaceholder(
    t("contextMenu.systemMenuLoading"),
  );
  const { children, error } = await loadShellEmbedChildren(path, {
    loading: t("contextMenu.systemMenuLoading"),
    error: t("contextMenu.systemMenuError"),
    empty: t("contextMenu.systemMenuEmpty"),
  });
  if (seq !== shellEmbedLoadSeq) return;
  shellEmbedChildren.value = children;
  shellEmbedError.value = error;
  shellEmbedLoading.value = false;
}

const workGroupMenuOptions = computed(() =>
  workGroupsForMenu.value.map((group) => ({
    id: group.id,
    name: workGroupDisplayName(group, t("workGroups.defaultName")),
  })),
);

const items = computed(() =>
  buildContextMenuItems(props.entry, {
    isFavorite: isFavoritePath(props.entry?.path),
    canPaste: hasClipboard.value,
    emptyArea: Boolean(props.emptyArea),
    recycleMode: isRecyclePath(currentPath.value),
    ideApps: ideApps.value,
    terminals: terminals.value,
    newFileTemplates: newFileTemplates.value,
    workGroups: workGroupMenuOptions.value,
  }).map((item) => {
    const base = {
      ...item,
      label: t(item.labelKey),
      children: item.children?.map((child) => ({
        ...child,
        label: childLabel(child),
      })),
    };
    if (item.shellEmbed) {
      return {
        ...base,
        children:
          shellEmbedChildren.value.length > 0
            ? shellEmbedChildren.value
            : [
                {
                  id: "shell:loading",
                  labelKey: "contextMenu.systemMenuLoading",
                  rawLabel: t("contextMenu.systemMenuLoading"),
                  label: t("contextMenu.systemMenuLoading"),
                  enabled: false,
                  shellCommandId: 0,
                  shellVerb: null,
                },
              ],
      };
    }
    return base;
  }),
);

const openSubmenuChildren = computed(
  () => items.value.find((item) => item.id === openSubmenuId.value)?.children ?? [],
);

async function refreshWorkGroupsForMenu() {
  try {
    const doc = await hydrateWorkGroups({ force: true });
    workGroupsForMenu.value = doc.workGroups.map((group) => ({
      id: group.id,
      name: group.name,
      isDefault: group.isDefault,
    }));
  } catch {
    workGroupsForMenu.value = [];
  }
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      void refreshClipboardFromSystem();
      void loadNewFileTemplates();
      void refreshWorkGroupsForMenu();
      clearSubmenuCloseTimer();
      resetShellEmbed();
      openSubmenuId.value = null;
      submenuAnchor.value = null;
      hideToolbarTip();
      // Warm once; do not re-probe PATH helpers on every right-click.
      preloadLaunchersIfNeeded();
    } else {
      clearSubmenuCloseTimer();
      resetShellEmbed();
      openSubmenuId.value = null;
      submenuAnchor.value = null;
      hideToolbarTip();
    }
  },
);

watch(openSubmenuChildren, async () => {
  if (openSubmenuId.value && submenuAnchor.value) {
    await placeSubmenu(submenuAnchor.value);
  }
});

watch(
  () => [props.open, props.x, props.y] as const,
  async ([isOpen]) => {
    if (!isOpen) return;
    await nextTick();
    const el = root.value;
    const pad = 8;
    const w = el?.offsetWidth ?? 220;
    const h = el?.offsetHeight ?? 180;
    const left = Math.min(props.x, window.innerWidth - w - pad);
    const top = Math.min(props.y, window.innerHeight - h - pad);
    pos.value = { left: Math.max(pad, left), top: Math.max(pad, top) };
    // Right-click often opens the menu under the cursor — mouseenter never fires.
    await nextTick();
    syncSubmenuFromClientPoint(props.x, props.y);
  },
);

async function copyPath(path: string) {
  try {
    await navigator.clipboard.writeText(path);
    emit("notice", t("contextMenu.copiedPath"));
  } catch (err) {
    emit("notice", err instanceof Error ? err.message : String(err));
  }
}

async function refreshListing() {
  emit("listingChanged");
  if (currentPath.value) {
    await navigateTo(currentPath.value, { syncOpenState: false });
  }
}

async function runPaste() {
  const dest = currentPath.value;
  if (!dest) {
    emit("notice", t("contextMenu.pasteNoFolder"));
    return;
  }
  try {
    await refreshClipboardFromSystem();
    const sources = [...clipboardPaths.value];
    const wasCut = clipboardMode.value === "cut";
    const result = await pasteClipboard(dest);
    if (!result.ok) {
      emit("notice", result.error ?? t("contextMenu.opFailed"));
      return;
    }
    if (wasCut) recordMoveUndo(sources, result.paths);
    else recordCopyUndo(result.paths);
    emit("notice", t("contextMenu.pasted", { count: result.paths.length }));
    await refreshListing();
  } catch (err) {
    emit("notice", err instanceof Error ? err.message : String(err));
  }
}

async function runNewFolder() {
  const parent = currentPath.value;
  if (!parent) {
    emit("notice", t("contextMenu.pasteNoFolder"));
    return;
  }
  const suggested = await resolveNewFolderDefaultName(t("contextMenu.newFolderDefault"));
  const prompted = await openPromptDialog({
    title: t("contextMenu.newFolder"),
    label: t("contextMenu.newFolderPrompt"),
    defaultValue: suggested,
    confirmLabel: t("common.create"),
  });
  if (!prompted.confirmed) return;
  const trimmed = prompted.value.trim();
  if (!trimmed) return;
  try {
    const result = await createFolder(parent, trimmed);
    if (!result.ok) {
      emit("notice", result.error ?? t("contextMenu.opFailed"));
      return;
    }
    if (result.paths[0]) recordCreateUndo(result.paths[0]);
    emit("notice", t("contextMenu.folderCreated"));
    await refreshListing();
  } catch (err) {
    emit("notice", err instanceof Error ? err.message : String(err));
  }
}

async function runNewFile(defaultName?: string) {
  const parent = currentPath.value;
  if (!parent) {
    emit("notice", t("contextMenu.pasteNoFolder"));
    return;
  }
  const suggested =
    defaultName?.trim() ||
    (await resolveNewFileDefaultName(t("contextMenu.newFileDefault")));
  const prompted = await openPromptDialog({
    title: t("contextMenu.newFile"),
    label: t("contextMenu.newFilePrompt"),
    defaultValue: suggested,
    confirmLabel: t("common.create"),
  });
  if (!prompted.confirmed) return;
  const trimmed = prompted.value.trim();
  if (!trimmed) return;
  try {
    const result = await createFile(parent, trimmed);
    if (!result.ok) {
      emit("notice", result.error ?? t("contextMenu.opFailed"));
      return;
    }
    if (result.paths[0]) recordCreateUndo(result.paths[0]);
    emit("notice", t("contextMenu.fileCreated"));
    await refreshListing();
  } catch (err) {
    emit("notice", err instanceof Error ? err.message : String(err));
  }
}

function pathsForClipboard(entry: NavigatorEntry): string[] {
  if (selectedPaths.value.includes(entry.path) && selectedPaths.value.length > 1) {
    return [...selectedPaths.value];
  }
  return [entry.path];
}

/** Selected folders for “Add to Work Group” (files in a multi-select are skipped). */
function folderPathsForWorkGroup(entry: NavigatorEntry): string[] {
  if (entry.kind !== "dir") return [];
  if (selectedPaths.value.includes(entry.path) && selectedPaths.value.length > 1) {
    const dirs = selectedEntries(listing.value?.entries)
      .filter((row) => row.kind === "dir")
      .map((row) => row.path);
    return dirs.length ? dirs : [entry.path];
  }
  return [entry.path];
}

const SKILL_ERROR_KEYS: Record<string, string> = {
  AI_SKILL_SOURCE_INVALID: "ai.skills.errorSourceInvalid",
  AI_SKILL_MANIFEST_MISSING: "ai.skills.errorManifestMissing",
  AI_SKILL_NAME_INVALID: "ai.skills.errorNameInvalid",
  AI_SKILL_EXISTS: "ai.skills.errorExists",
  AI_SKILL_UNKNOWN: "ai.skills.errorUnknown",
  AI_SKILL_ROOT_INVALID: "ai.skills.errorRootInvalid",
};

function mapSkillError(err: unknown): string {
  const raw = err instanceof Error ? err.message : String(err);
  for (const [code, key] of Object.entries(SKILL_ERROR_KEYS)) {
    if (raw.includes(code)) return t(key);
  }
  return raw;
}

/** 将文件夹导入 AI 板块公共 Skill 库（全局 skill）。 */
async function runAddAsSkill(entry: NavigatorEntry) {
  if (entry.kind !== "dir") return;
  try {
    const [, skill] = await invoke<
      [{ root: string; skills: unknown[] }, { name: string; path: string }]
    >("ai_skills_import", {
      sourcePath: entry.path,
      name: null,
    });
    emit("notice", t("contextMenu.addedAsSkill", { name: skill.name }));
  } catch (err) {
    const raw = err instanceof Error ? err.message : String(err);
    // 已入库：友好提示，不当作失败噪音
    if (raw.includes("AI_SKILL_EXISTS")) {
      emit("notice", t("contextMenu.addAsSkillExists", { name: entry.name }));
      return;
    }
    emit("notice", mapSkillError(err));
  }
}

async function runAddToWorkGroup(workGroupId: string, entry: NavigatorEntry) {
  const paths = folderPathsForWorkGroup(entry);
  if (!paths.length) return;
  try {
    let doc = await hydrateWorkGroups({ force: true });
    let target = doc.workGroups.find((group) => group.id === workGroupId);
    if (!target) {
      emit("notice", t("contextMenu.opFailed"));
      return;
    }
    const displayName = workGroupDisplayName(target, t("workGroups.defaultName"));
    let liveForTarget: WorkGroupOpenState | null =
      workGroupId === activeWorkGroupId.value
        ? snapshotLiveOpenState()
        : target.openState;
    let lastOpen: WorkGroupOpenState | null = null;
    for (const memberPath of paths) {
      const [nextDoc, nextOpen] = await invoke<
        [UserWorkGroupsDoc, WorkGroupOpenState, SoftNotice | null]
      >("work_groups_add_member", {
        workGroupId,
        memberPath,
        liveOpenState: liveForTarget,
      });
      doc = nextDoc;
      rememberHydratedWorkGroups(nextDoc);
      liveForTarget = nextOpen;
      lastOpen = nextOpen;
      target = nextDoc.workGroups.find((group) => group.id === workGroupId) ?? target;
    }
    if (workGroupId === activeWorkGroupId.value && lastOpen) {
      await restoreWorkGroupOpenState(workGroupId, lastOpen);
    }
    emit(
      "notice",
      t("contextMenu.addedToWorkGroup", { count: paths.length, name: displayName }),
    );
  } catch (err) {
    emit("notice", err instanceof Error ? err.message : String(err));
  }
}

async function runDelete(entry: NavigatorEntry) {
  const paths = pathsForClipboard(entry);
  try {
    const blocked = await pathsWithoutRecycle(paths);
    const label =
      paths.length === 1
        ? entry.name
        : t("navigator.nItems", { count: paths.length });
    if (blocked.length) {
      const ok = await confirmPermanentDelete({
        title: t("dialog.deleteTitle"),
        message: t("contextMenu.deletePermanentConfirm", { name: label }),
        confirmLabel: t("common.delete"),
        cancelLabel: t("common.cancel"),
      });
      if (!ok) return;
      const recyclable = paths.filter((path) => !blocked.includes(path));
      if (recyclable.length) {
        const recycled = await deleteEntries(recyclable, false);
        if (!recycled.ok) {
          emit("notice", recycled.error ?? t("contextMenu.opFailed"));
          return;
        }
        recordDeleteUndo(recyclable);
      }
      const removed = await deleteEntries(blocked, true);
      if (!removed.ok) {
        emit("notice", removed.error ?? t("contextMenu.opFailed"));
        return;
      }
      emit("notice", t("contextMenu.deletedPermanent"));
      await refreshListing();
      return;
    }
    const ok = await confirmDeleteIfNeeded({
      title: t("dialog.deleteTitle"),
      message: t("contextMenu.deleteConfirm", { name: label }),
      confirmLabel: t("common.delete"),
      cancelLabel: t("common.cancel"),
    });
    if (!ok) return;
    const result = await deleteEntries(paths);
    if (!result.ok) {
      emit("notice", result.error ?? t("contextMenu.opFailed"));
      return;
    }
    recordDeleteUndo(paths);
    emit("notice", t("contextMenu.deleted"));
    await refreshListing();
  } catch (err) {
    emit("notice", err instanceof Error ? err.message : String(err));
  }
}

type ArchiveResult = {
  ok: boolean;
  message: string;
  outputPath: string | null;
};

async function runCompressZip(entry: NavigatorEntry) {
  const paths = pathsForClipboard(entry);
  try {
    const result = await invoke<ArchiveResult>("archive_compress_zip", {
      paths,
      destZip: null,
    });
    if (!result.ok) {
      emit("notice", result.message || t("contextMenu.opFailed"));
      return;
    }
    emit(
      "notice",
      t("contextMenu.compressed", {
        path: result.outputPath ?? "",
      }),
    );
    await refreshListing();
  } catch (err) {
    emit("notice", err instanceof Error ? err.message : String(err));
  }
}

async function runExtractZip(entry: NavigatorEntry) {
  try {
    const result = await invoke<ArchiveResult>("archive_extract_zip", {
      zipPath: entry.path,
      destDir: null,
    });
    if (!result.ok) {
      emit("notice", result.message || t("contextMenu.opFailed"));
      return;
    }
    emit(
      "notice",
      t("contextMenu.extracted", {
        path: result.outputPath ?? "",
      }),
    );
    await refreshListing();
  } catch (err) {
    emit("notice", err instanceof Error ? err.message : String(err));
  }
}

async function runShellVerb(child: ShellEmbedMenuChild, path: string) {
  const error = await invokeShellEmbedChild(path, child);
  if (error) emit("notice", error);
}

async function run(id: string) {
  const entry = props.entry;
  if (id.startsWith("shell:")) {
    if (id === "shell:loading" || id === "shell:error" || id === "shell:empty") return;
    const child = shellEmbedChildren.value.find((row) => row.id === id);
    if (!entry || !child?.enabled) return;
    emit("close");
    await runShellVerb(child, entry.path);
    return;
  }
  emit("close");
  switch (id) {
    case "open":
      // Folders → current tab; files follow openEntry policy (same as dblclick).
      if (!entry) break;
      try {
        if (entry.kind === "dir") {
          await navigateTo(entry.path);
        } else {
          await openEntry(entry);
        }
      } catch (err) {
        emit("notice", err instanceof Error ? err.message : String(err));
      }
      break;
    case "openInNewTab":
      // Always append a new page group (even if path already has a tab).
      if (entry && canOpenInNewTab(entry)) {
        await newNavigatorTab(entry.path);
      }
      break;
    case "openWithSystemDefault":
      if (!entry) break;
      try {
        const result = await invoke<{ success: boolean; error: string | null }>(
          "open_path_default",
          { path: entry.path },
        );
        if (!result.success && result.error) {
          emit("notice", result.error);
        }
      } catch (err) {
        emit("notice", err instanceof Error ? err.message : String(err));
      }
      break;
    case "openWithNative":
      if (!entry) break;
      try {
        const result = await invoke<{
          success: boolean;
          error: string | null;
        }>("open_native_open_with_dialog", { path: entry.path });
        if (!result.success && result.error) {
          emit("notice", result.error);
        }
      } catch (err) {
        emit("notice", err instanceof Error ? err.message : String(err));
      }
      break;
    case "openInTerminal":
      if (!entry) break;
      try {
        const result = await invoke<{
          success: boolean;
          error: string | null;
          launcher: string | null;
        }>("open_in_terminal", {
          path: entry.path,
          terminalId: null,
          elevated: false,
        });
        if (!result.success && result.error) {
          emit("notice", result.error);
        }
      } catch (err) {
        emit("notice", err instanceof Error ? err.message : String(err));
      }
      break;
    case "toggleFavorite":
      if (!entry) break;
      try {
        const result = await toggleFavorite(entry.path, entry.kind === "dir");
        emit(
          "notice",
          result.added
            ? t("contextMenu.favoriteAdded")
            : t("contextMenu.favoriteRemoved"),
        );
        emit("favoritesChanged");
      } catch (err) {
        emit("notice", err instanceof Error ? err.message : String(err));
      }
      break;
    case "addAsSkill":
      if (entry) await runAddAsSkill(entry);
      break;
    case "systemMenu":
      // AIA-239: systemMenu is a submenu parent; leaf shell verbs use shell:* ids.
      break;
    case "reveal":
      if (!entry) break;
      try {
        await revealItemInDir(entry.path);
      } catch (err) {
        emit("notice", err instanceof Error ? err.message : String(err));
      }
      break;
    case "copy":
      if (!entry) break;
      setClipboard(pathsForClipboard(entry), "copy");
      emit("notice", t("contextMenu.copied"));
      break;
    case "cut":
      if (!entry) break;
      setClipboard(pathsForClipboard(entry), "cut");
      emit("notice", t("contextMenu.cutDone"));
      break;
    case "paste":
      await runPaste();
      break;
    case "rename":
      if (entry) emit("requestRename", entry.path);
      break;
    case "newFolder":
      await runNewFolder();
      break;
    case "newFile":
    case "newFileCustom":
      await runNewFile();
      break;
    case "copyPath":
      if (entry) await copyPath(entry.path);
      break;
    case "compressZip":
      if (entry) await runCompressZip(entry);
      break;
    case "extractZip":
      if (entry) await runExtractZip(entry);
      break;
    case "properties":
      if (entry) emit("openProperties", entry.path);
      break;
    case "restore":
      if (!entry) break;
      try {
        const paths = pathsForClipboard(entry);
        const result = await restoreRecyclePaths(paths);
        if (!result.ok) {
          emit("notice", result.error ?? t("contextMenu.opFailed"));
          return;
        }
        emit("notice", t("recycleBin.restored", { count: result.paths.length }));
        await refreshListing();
      } catch (err) {
        emit("notice", err instanceof Error ? err.message : String(err));
      }
      break;
    case "delete":
      if (entry) await runDelete(entry);
      break;
    default:
      if (id.startsWith("addToWorkGroup:") && entry) {
        const workGroupId = id.slice("addToWorkGroup:".length);
        if (workGroupId && workGroupId !== "empty") {
          await runAddToWorkGroup(workGroupId, entry);
        }
        break;
      }
      if (id.startsWith("newFileTemplate:")) {
        const ext = id.slice("newFileTemplate:".length);
        const tpl = newFileTemplates.value.find((item) => item.extension === ext);
        await runNewFile(tpl?.defaultName ?? `New ${ext} file.${ext}`);
        break;
      }
      if (id.startsWith("openWithIde:") && entry) {
        const ideId = id.slice("openWithIde:".length);
        try {
          const result = await invoke<{ success: boolean; error: string | null }>(
            "ide_open_path",
            { ideId, path: entry.path },
          );
          if (!result.success && result.error) {
            emit("notice", result.error);
          }
        } catch (err) {
          emit("notice", err instanceof Error ? err.message : String(err));
        }
      } else if (id.startsWith("terminal:") && entry) {
        const rest = id.slice("terminal:".length);
        const elevated = rest.endsWith(":admin");
        const terminalId = elevated ? rest.slice(0, -":admin".length) : rest;
        try {
          const result = await invoke<{
            success: boolean;
            error: string | null;
            launcher: string | null;
          }>("open_in_terminal", {
            path: entry.path,
            terminalId,
            elevated,
          });
          if (!result.success && result.error) {
            emit("notice", result.error);
          }
        } catch (err) {
          emit("notice", err instanceof Error ? err.message : String(err));
        }
      }
      break;
  }
}

function positionSubmenuFromAnchor(anchor: HTMLElement, width = 200, height = 160) {
  const rect = anchor.getBoundingClientRect();
  const pad = 8;
  // Overlap parent so the pointer can travel into the flyout without a dead gap.
  let left = rect.right - 8;
  if (left + width > window.innerWidth - pad) {
    left = Math.max(pad, rect.left - width + 8);
  }
  let top = rect.top;
  if (top + height > window.innerHeight - pad) {
    top = Math.max(pad, window.innerHeight - height - pad);
  }
  submenuPos.value = { left, top };
}

async function placeSubmenu(anchor: HTMLElement) {
  // Position immediately (avoid flash at 0,0), then refine after Teleport mount.
  positionSubmenuFromAnchor(anchor);
  await nextTick();
  await nextTick();
  const sub = submenuRoot.value;
  if (!sub) return;
  positionSubmenuFromAnchor(anchor, sub.offsetWidth || 200, sub.offsetHeight || 160);
}

function clearSubmenuCloseTimer() {
  if (submenuCloseTimer != null) {
    clearTimeout(submenuCloseTimer);
    submenuCloseTimer = null;
  }
}

function scheduleSubmenuClose() {
  clearSubmenuCloseTimer();
  submenuCloseTimer = setTimeout(() => {
    openSubmenuId.value = null;
    submenuAnchor.value = null;
    submenuCloseTimer = null;
  }, 220);
}

function openSubmenuFor(itemId: string, anchor: HTMLElement) {
  clearSubmenuCloseTimer();
  if (itemId === "openWith" || itemId === "openInTerminal") {
    preloadLaunchersIfNeeded();
  }
  if (itemId === "systemMenu" && props.entry) {
    void ensureShellEmbedLoaded(props.entry.path);
  }
  // Position before mount so the flyout never appears at (0,0).
  positionSubmenuFromAnchor(anchor);
  openSubmenuId.value = itemId;
  submenuAnchor.value = anchor;
  void placeSubmenu(anchor);
}

function findItemById(itemId: string) {
  return items.value.find((item) => item.id === itemId) ?? null;
}

function itemHasSubmenu(item: {
  children?: unknown[];
  shellEmbed?: boolean;
}): boolean {
  return Boolean(item.shellEmbed || (item.children && item.children.length));
}

/** Sigma-style: which parent row is under the pointer → expand its submenu. */
function syncSubmenuFromClientPoint(clientX: number, clientY: number) {
  if (!props.open || !root.value) return;
  const hit = document.elementFromPoint(clientX, clientY);
  if (!(hit instanceof Element)) return;
  // Still inside the open flyout — keep it.
  if (submenuRoot.value?.contains(hit)) {
    clearSubmenuCloseTimer();
    return;
  }
  const btn = hit.closest<HTMLElement>("[data-ctx-id]");
  if (!btn || !root.value.contains(btn)) {
    // Pointer left the parent menu (and not over flyout) — delay-close.
    if (openSubmenuId.value) scheduleSubmenuClose();
    return;
  }
  const itemId = btn.dataset.ctxId;
  if (!itemId) return;
  const item = findItemById(itemId);
  if (!item) return;
  if (itemHasSubmenu(item)) {
    openSubmenuFor(item.id, btn);
    return;
  }
  scheduleSubmenuClose();
}

function onItemClick(
  item: { id: string; children?: unknown[]; shellEmbed?: boolean },
  event: MouseEvent,
) {
  if (itemHasSubmenu(item)) {
    // Hover owns expand; click keeps the flyout open (do not toggle shut).
    if (event.currentTarget instanceof HTMLElement) {
      openSubmenuFor(item.id, event.currentTarget);
    }
    return;
  }
  void run(item.id);
}

function onItemEnter(
  item: { id: string; children?: unknown[]; shellEmbed?: boolean },
  event: MouseEvent,
) {
  if (itemHasSubmenu(item)) {
    if (event.currentTarget instanceof HTMLElement) {
      openSubmenuFor(item.id, event.currentTarget);
    }
    return;
  }
  scheduleSubmenuClose();
}

/** Keyboard focus expands nested / shell-embed submenu like hover. */
function onItemFocus(
  item: { id: string; children?: unknown[]; shellEmbed?: boolean },
  event: FocusEvent,
) {
  if (!(event.currentTarget instanceof HTMLElement)) return;
  if (itemHasSubmenu(item)) {
    openSubmenuFor(item.id, event.currentTarget);
  }
}

function onItemKeydown(
  item: { id: string; children?: unknown[]; shellEmbed?: boolean },
  event: KeyboardEvent,
) {
  if (event.key !== "ArrowRight") return;
  if (!(event.currentTarget instanceof HTMLElement)) return;
  if (itemHasSubmenu(item)) {
    event.preventDefault();
    openSubmenuFor(item.id, event.currentTarget);
  }
}

function onMenuMouseMove(event: MouseEvent) {
  syncSubmenuFromClientPoint(event.clientX, event.clientY);
}

function onSubmenuEnter() {
  clearSubmenuCloseTimer();
}

function onSubmenuLeave(event: MouseEvent) {
  const related = event.relatedTarget;
  if (related instanceof Node && root.value?.contains(related)) {
    // Moving back to a parent row — let that row's enter/mousemove decide.
    return;
  }
  scheduleSubmenuClose();
}

function onDocPointer(event: MouseEvent) {
  if (!props.open) return;
  const target = event.target;
  if (!(target instanceof Node)) {
    emit("close");
    return;
  }
  if (root.value?.contains(target) || submenuRoot.value?.contains(target)) return;
  emit("close");
}

function onKey(event: KeyboardEvent) {
  if (!props.open) return;
  if (event.key === "Escape") {
    event.preventDefault();
    emit("close");
  }
}

onMounted(() => {
  window.addEventListener("mousedown", onDocPointer, true);
  window.addEventListener("keydown", onKey, true);
  // AIA-181 P0: do not warm IDE/terminal on Navigator mount — first right-click
  // (watch on `open`) or Open-with/Terminal submenu path calls preloadLaunchersIfNeeded.
});

onUnmounted(() => {
  clearSubmenuCloseTimer();
  resetShellEmbed();
  window.removeEventListener("mousedown", onDocPointer, true);
  window.removeEventListener("keydown", onKey, true);
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open && (entry || emptyArea)"
      ref="root"
      class="ctx fluent-surface"
      role="menu"
      :aria-label="t('contextMenu.label')"
      :style="{ left: `${pos.left}px`, top: `${pos.top}px` }"
      @contextmenu.prevent
      @mousemove="onMenuMouseMove"
    >
      <div class="ctx-toolbar" role="toolbar" :aria-label="t('contextMenu.toolbar')">
        <button
          v-for="tool in toolbarItems"
          :key="tool.id"
          type="button"
          class="ctx-tool"
          :disabled="!tool.enabled"
          :aria-label="toolbarTipText(tool)"
          @click="run(tool.id)"
          @mouseenter="showToolbarTip($event, tool)"
          @mouseleave="hideToolbarTip"
          @focus="showToolbarTip($event, tool)"
          @blur="hideToolbarTip"
        >
          <FluentIcon v-if="tool.icon" :name="tool.icon" size="sm" />
        </button>
      </div>
      <div
        v-for="item in items"
        :key="item.id"
        class="ctx-row"
        :class="{ 'ctx-row--sep': item.separatorBefore }"
      >
        <button
          type="button"
          class="ctx-item"
          role="menuitem"
          :data-ctx-id="item.id"
          :aria-haspopup="itemHasSubmenu(item) ? 'menu' : undefined"
          :aria-expanded="itemHasSubmenu(item) ? openSubmenuId === item.id : undefined"
          :disabled="!item.enabled"
          :class="{
            'ctx-item--danger': item.danger,
            'ctx-item--submenu-open': openSubmenuId === item.id,
          }"
          @click="onItemClick(item, $event)"
          @mouseenter="onItemEnter(item, $event)"
          @focus="onItemFocus(item, $event)"
          @keydown="onItemKeydown(item, $event)"
        >
          <span class="ctx-leading">
            <FluentIcon v-if="item.icon" :name="item.icon" size="sm" class="ctx-icon" />
            <span>{{ item.label }}</span>
          </span>
          <span
            v-if="itemHasSubmenu(item)"
            class="ctx-chevron"
            aria-hidden="true"
          >›</span>
          <span v-else-if="item.stub" class="later">{{ t("chrome.later") }}</span>
        </button>
      </div>
    </div>
  </Teleport>
  <Teleport to="body">
    <div
      v-if="open && tip"
      class="ctx-tip"
      role="tooltip"
      :style="{ left: `${tip.left}px`, top: `${tip.top}px` }"
    >
      {{ tip.text }}
    </div>
  </Teleport>
  <Teleport to="body">
    <div
      v-if="open && openSubmenuId && openSubmenuChildren.length"
      ref="submenuRoot"
      class="ctx-sub fluent-surface"
      role="menu"
      :style="{ left: `${submenuPos.left}px`, top: `${submenuPos.top}px` }"
      @contextmenu.prevent
      @mouseenter="onSubmenuEnter"
      @mouseleave="onSubmenuLeave"
    >
      <button
        v-for="child in openSubmenuChildren"
        :key="child.id"
        type="button"
        class="ctx-item"
        role="menuitem"
        :disabled="!child.enabled"
        :class="{
          'ctx-item--danger': child.danger,
          'ctx-item--sep': child.separatorBefore,
        }"
        @click="run(child.id)"
      >
        <span class="ctx-leading">
          <FluentIcon v-if="child.icon" :name="child.icon" size="sm" class="ctx-icon" />
          <span>{{ child.label }}</span>
        </span>
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
.ctx,
.ctx-sub {
  position: fixed;
  z-index: 1200;
  min-width: 13.5rem;
  padding: var(--fluent-space-xs);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  box-shadow: var(--fluent-shadow-flyout, 0 8px 24px rgb(0 0 0 / 18%));
  background: var(--fluent-color-fill-control);
}

.ctx-sub {
  min-width: 12.5rem;
  z-index: 1201;
}

.ctx-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 2px;
  padding: var(--fluent-space-xs) var(--fluent-space-xs) var(--fluent-space-sm);
  margin-bottom: var(--fluent-space-xs);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.ctx-tool {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--fluent-control-height-standard);
  height: var(--fluent-control-height-standard);
  border: 0;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: inherit;
  cursor: pointer;
}

.ctx-tool:hover:not(:disabled),
.ctx-tool:focus-visible {
  background: var(--fluent-color-fill-selected);
  outline: none;
}

.ctx-tool:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.ctx-tip {
  position: fixed;
  z-index: 1300;
  transform: translate(-50%, -100%);
  padding: 4px 8px;
  border-radius: var(--fluent-radius-sm);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
  box-shadow: var(--fluent-shadow-flyout);
  color: var(--fluent-color-text-primary);
  font-size: var(--fluent-font-size-caption);
  white-space: nowrap;
  pointer-events: none;
}

.ctx-row--sep {
  margin-top: var(--fluent-space-xs);
  border-top: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  padding-top: var(--fluent-space-xs);
}

.ctx-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  width: 100%;
  min-height: var(--fluent-control-height-standard);
  padding: 0 var(--fluent-space-md);
  border: 0;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.ctx-leading {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  min-width: 0;
}

.ctx-icon {
  flex-shrink: 0;
  opacity: 0.9;
}

.ctx-chevron {
  color: var(--fluent-color-text-secondary);
  font-size: 1rem;
  line-height: 1;
}

.ctx-item:hover:not(:disabled),
.ctx-item:focus-visible,
.ctx-item--submenu-open:not(:disabled) {
  background: var(--fluent-color-fill-selected);
  outline: none;
}

.ctx-item:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.ctx-item--danger {
  color: var(--fluent-color-status-danger);
}

.ctx-item--sep {
  margin-top: var(--fluent-space-xs);
  border-top: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: 0 0 var(--fluent-radius-sm) var(--fluent-radius-sm);
  padding-top: var(--fluent-space-xs);
}

.later {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
</style>
