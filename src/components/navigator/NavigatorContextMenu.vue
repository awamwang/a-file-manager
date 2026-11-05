<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import type { NavigatorEntry } from "../../state/navigatorBrowse";
import { currentPath, navigateTo, newNavigatorTab, openEntry } from "../../state/navigatorBrowse";
import {
  createFile,
  createFolder,
  deleteEntries,
  hasClipboard,
  pathsWithoutRecycle,
  pasteClipboard,
  renameEntry,
  setClipboard,
} from "../../state/fileClipboard";
import {
  resolveNewFileDefaultName,
  resolveNewFolderDefaultName,
} from "../../state/newItemNames";
import { selectedPaths } from "../../state/navigatorSelection";
import {
  isFavoritePath,
  toggleFavorite,
} from "../../state/favorites";
import FluentIcon from "../icons/FluentIcon.vue";
import {
  buildContextMenuItems,
  canOpenInNewTab,
} from "./contextMenuModel";

type ShellMenuResult = {
  success: boolean;
  cancelled: boolean;
  error: string | null;
};

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
}>();

const { t } = useI18n();
const root = ref<HTMLElement | null>(null);
const submenuRoot = ref<HTMLElement | null>(null);
const pos = ref({ left: 0, top: 0 });
const submenuPos = ref({ left: 0, top: 0 });
const openSubmenuId = ref<string | null>(null);
const ideApps = ref<Array<{ id: string; name: string; available: boolean }>>([]);
const terminals = ref<
  Array<{ id: string; name: string; available: boolean; supportsElevation: boolean }>
>([]);

type IdeAppInfo = { id: string; name: string; available: boolean; resolvedPath?: string | null };
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
}

function childLabel(child: {
  labelKey: string;
  ideId?: string;
  terminalId?: string;
  elevated?: boolean;
}): string {
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

const items = computed(() =>
  buildContextMenuItems(props.entry, {
    isFavorite: isFavoritePath(props.entry?.path),
    canPaste: hasClipboard.value,
    emptyArea: Boolean(props.emptyArea),
    ideApps: ideApps.value,
    terminals: terminals.value,
  }).map((item) => ({
    ...item,
    label: t(item.labelKey),
    children: item.children?.map((child) => ({
      ...child,
      label: childLabel(child),
    })),
  })),
);

const openSubmenuChildren = computed(
  () => items.value.find((item) => item.id === openSubmenuId.value)?.children ?? [],
);

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      openSubmenuId.value = null;
      void loadLaunchers();
    }
  },
);

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
    const result = await pasteClipboard(dest);
    if (!result.ok) {
      emit("notice", result.error ?? t("contextMenu.opFailed"));
      return;
    }
    emit("notice", t("contextMenu.pasted", { count: result.paths.length }));
    await refreshListing();
  } catch (err) {
    emit("notice", err instanceof Error ? err.message : String(err));
  }
}

async function runRename(entry: NavigatorEntry) {
  const next = window.prompt(t("contextMenu.renamePrompt"), entry.name);
  if (next == null) return;
  const trimmed = next.trim();
  if (!trimmed || trimmed === entry.name) return;
  try {
    const result = await renameEntry(entry.path, trimmed);
    if (!result.ok) {
      emit("notice", result.error ?? t("contextMenu.opFailed"));
      return;
    }
    emit("notice", t("contextMenu.renamed"));
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
  const name = window.prompt(t("contextMenu.newFolderPrompt"), suggested) ?? "";
  const trimmed = name.trim();
  if (!trimmed) return;
  try {
    const result = await createFolder(parent, trimmed);
    if (!result.ok) {
      emit("notice", result.error ?? t("contextMenu.opFailed"));
      return;
    }
    emit("notice", t("contextMenu.folderCreated"));
    await refreshListing();
  } catch (err) {
    emit("notice", err instanceof Error ? err.message : String(err));
  }
}

async function runNewFile() {
  const parent = currentPath.value;
  if (!parent) {
    emit("notice", t("contextMenu.pasteNoFolder"));
    return;
  }
  const suggested = await resolveNewFileDefaultName(t("contextMenu.newFileDefault"));
  const name = window.prompt(t("contextMenu.newFilePrompt"), suggested) ?? "";
  const trimmed = name.trim();
  if (!trimmed) return;
  try {
    const result = await createFile(parent, trimmed);
    if (!result.ok) {
      emit("notice", result.error ?? t("contextMenu.opFailed"));
      return;
    }
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

async function runDelete(entry: NavigatorEntry) {
  const paths = pathsForClipboard(entry);
  try {
    const blocked = await pathsWithoutRecycle(paths);
    if (blocked.length) {
      emit("notice", t("contextMenu.deleteNoRecycle"));
      return;
    }
    const label =
      paths.length === 1
        ? entry.name
        : t("navigator.nItems", { count: paths.length });
    const ok = window.confirm(
      t("contextMenu.deleteConfirm", { name: label }),
    );
    if (!ok) return;
    const result = await deleteEntries(paths);
    if (!result.ok) {
      emit("notice", result.error ?? t("contextMenu.opFailed"));
      return;
    }
    emit("notice", t("contextMenu.deleted"));
    await refreshListing();
  } catch (err) {
    emit("notice", err instanceof Error ? err.message : String(err));
  }
}

async function run(id: string) {
  const entry = props.entry;
  emit("close");
  switch (id) {
    case "open":
      if (!entry) break;
      if (entry.kind === "dir") {
        await openEntry(entry);
      } else {
        try {
          await openPath(entry.path);
        } catch (err) {
          emit("notice", err instanceof Error ? err.message : String(err));
        }
      }
      break;
    case "openInNewTab":
      if (entry && canOpenInNewTab(entry)) {
        await newNavigatorTab(entry.path);
      }
      break;
    case "openWithSystemDefault":
      if (!entry) break;
      try {
        await openPath(entry.path);
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
    case "systemMenu":
      if (!entry) break;
      try {
        const result = await invoke<ShellMenuResult>("shell_show_context_menu", {
          path: entry.path,
          x: Math.round(props.x),
          y: Math.round(props.y),
        });
        if (!result.success && result.error) {
          emit("notice", result.error);
        }
      } catch (err) {
        emit("notice", err instanceof Error ? err.message : String(err));
      }
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
      if (entry) await runRename(entry);
      break;
    case "newFolder":
      await runNewFolder();
      break;
    case "newFile":
      await runNewFile();
      break;
    case "copyPath":
      if (entry) await copyPath(entry.path);
      break;
    case "properties":
      if (entry) emit("openProperties", entry.path);
      break;
    case "delete":
      if (entry) await runDelete(entry);
      break;
    default:
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

async function placeSubmenu(anchor: HTMLElement) {
  await nextTick();
  const rect = anchor.getBoundingClientRect();
  const sub = submenuRoot.value;
  const pad = 8;
  const w = sub?.offsetWidth ?? 200;
  const h = sub?.offsetHeight ?? 160;
  let left = rect.right - 4;
  if (left + w > window.innerWidth - pad) {
    left = Math.max(pad, rect.left - w + 4);
  }
  let top = rect.top;
  if (top + h > window.innerHeight - pad) {
    top = Math.max(pad, window.innerHeight - h - pad);
  }
  submenuPos.value = { left, top };
}

function onItemClick(item: { id: string; children?: unknown[] }, event: MouseEvent) {
  if (item.children?.length) {
    const next = openSubmenuId.value === item.id ? null : item.id;
    openSubmenuId.value = next;
    if (next && event.currentTarget instanceof HTMLElement) {
      void placeSubmenu(event.currentTarget);
    }
    return;
  }
  void run(item.id);
}

function onItemEnter(item: { id: string; children?: unknown[] }, event: MouseEvent) {
  if (!item.children?.length) {
    openSubmenuId.value = null;
    return;
  }
  openSubmenuId.value = item.id;
  if (event.currentTarget instanceof HTMLElement) {
    void placeSubmenu(event.currentTarget);
  }
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
});

onUnmounted(() => {
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
    >
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
          :aria-haspopup="item.children?.length ? 'menu' : undefined"
          :aria-expanded="item.children?.length ? openSubmenuId === item.id : undefined"
          :disabled="!item.enabled"
          :class="{ 'ctx-item--danger': item.danger }"
          @click="onItemClick(item, $event)"
          @mouseenter="onItemEnter(item, $event)"
        >
          <span class="ctx-leading">
            <FluentIcon v-if="item.icon" :name="item.icon" size="sm" class="ctx-icon" />
            <span>{{ item.label }}</span>
          </span>
          <span v-if="item.children?.length" class="ctx-chevron" aria-hidden="true">›</span>
          <span v-else-if="item.stub" class="later">{{ t("chrome.later") }}</span>
        </button>
      </div>
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
.ctx-item:focus-visible {
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
