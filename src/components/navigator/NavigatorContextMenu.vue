<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import type { NavigatorEntry } from "../../state/navigatorBrowse";
import { newNavigatorTab, openEntry } from "../../state/navigatorBrowse";
import {
  buildContextMenuItems,
  canOpenInNewTab,
} from "./contextMenuModel";

const props = defineProps<{
  entry: NavigatorEntry | null;
  x: number;
  y: number;
  open: boolean;
}>();

const emit = defineEmits<{
  close: [];
  notice: [message: string];
}>();

const { t } = useI18n();
const root = ref<HTMLElement | null>(null);
const pos = ref({ left: 0, top: 0 });

const items = computed(() =>
  buildContextMenuItems(props.entry).map((item) => ({
    ...item,
    label: t(item.labelKey),
  })),
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

async function run(id: string) {
  const entry = props.entry;
  if (!entry) return;
  emit("close");
  switch (id) {
    case "open":
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
      if (canOpenInNewTab(entry)) {
        await newNavigatorTab(entry.path);
      }
      break;
    case "reveal":
      try {
        await revealItemInDir(entry.path);
      } catch (err) {
        emit("notice", err instanceof Error ? err.message : String(err));
      }
      break;
    case "copyPath":
      await copyPath(entry.path);
      break;
    case "delete":
      emit("notice", t("contextMenu.deleteStub"));
      break;
    default:
      break;
  }
}

function onDocPointer(event: MouseEvent) {
  if (!props.open) return;
  const el = root.value;
  if (el && event.target instanceof Node && el.contains(event.target)) return;
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
      v-if="open && entry"
      ref="root"
      class="ctx fluent-surface"
      role="menu"
      :aria-label="t('contextMenu.label')"
      :style="{ left: `${pos.left}px`, top: `${pos.top}px` }"
      @contextmenu.prevent
    >
      <button
        v-for="item in items"
        :key="item.id"
        type="button"
        class="ctx-item"
        role="menuitem"
        :disabled="!item.enabled"
        :class="{ 'ctx-item--danger': item.danger }"
        @click="run(item.id)"
      >
        <span>{{ item.label }}</span>
        <span v-if="item.stub" class="later">{{ t("chrome.later") }}</span>
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
.ctx {
  position: fixed;
  z-index: 80;
  min-width: 12.5rem;
  padding: var(--fluent-space-xs);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  box-shadow: var(--fluent-shadow-flyout, 0 8px 24px rgb(0 0 0 / 18%));
  background: var(--fluent-color-fill-control);
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

.later {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
</style>
