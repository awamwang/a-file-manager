<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { AppCommand, CommandGroup } from "../../commands/types";
import {
  filterCommands,
  groupRankedCommands,
} from "../../commands/fuzzyFilter";
import {
  closeCommandPanel,
  commandPanelOpen,
} from "../../state/commandPanel";
import FluentIcon from "../icons/FluentIcon.vue";
import { commandGroupIcon } from "../icons/glyphs";

const { t } = useI18n();

const GROUP_LABEL: Record<CommandGroup, string> = {
  Navigate: "commands.groups.navigate",
  View: "commands.groups.view",
  Actions: "commands.groups.actions",
};

function groupLabel(group: string): string {
  const key = GROUP_LABEL[group as CommandGroup];
  return key ? t(key) : group;
}

const props = defineProps<{
  commands: AppCommand[];
}>();

const query = ref("");
const activeIndex = ref(0);
const inputEl = ref<HTMLInputElement | null>(null);
const listEl = ref<HTMLElement | null>(null);

const ranked = computed(() => filterCommands(props.commands, query.value));
const sections = computed(() => groupRankedCommands(ranked.value));
const flat = computed(() => ranked.value.map((r) => r.command));

watch(commandPanelOpen, async (open) => {
  if (open) {
    query.value = "";
    activeIndex.value = 0;
    await nextTick();
    inputEl.value?.focus();
  }
});

watch(query, () => {
  activeIndex.value = 0;
});

watch(flat, (list) => {
  if (activeIndex.value >= list.length) {
    activeIndex.value = Math.max(0, list.length - 1);
  }
});

function onBackdropClick(event: MouseEvent) {
  if (event.target === event.currentTarget) {
    closeCommandPanel();
  }
}

async function runCommand(command: AppCommand) {
  closeCommandPanel();
  await command.run();
}

async function onEnter() {
  const command = flat.value[activeIndex.value];
  if (command) await runCommand(command);
}

function moveActive(delta: number) {
  const len = flat.value.length;
  if (!len) return;
  activeIndex.value = (activeIndex.value + delta + len) % len;
  void nextTick(() => {
    const selected = listEl.value?.querySelector<HTMLElement>(
      `[data-cmd-index="${activeIndex.value}"]`,
    );
    selected?.scrollIntoView({ block: "nearest" });
  });
}

function onPanelKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    closeCommandPanel();
    return;
  }
  if (event.key === "ArrowDown") {
    event.preventDefault();
    moveActive(1);
    return;
  }
  if (event.key === "ArrowUp") {
    event.preventDefault();
    moveActive(-1);
    return;
  }
  if (event.key === "Enter") {
    event.preventDefault();
    void onEnter();
  }
}

function flatIndexFor(commandId: string): number {
  return flat.value.findIndex((c) => c.id === commandId);
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="commandPanelOpen"
      class="cmd-backdrop fluent-chrome"
      role="presentation"
      @mousedown="onBackdropClick"
      @keydown="onPanelKeydown"
    >
      <div
        class="cmd-panel"
        role="dialog"
        aria-modal="true"
        :aria-label="t('commands.panelLabel')"
        @mousedown.stop
      >
        <label class="cmd-label" for="command-panel-input">{{
          t("chrome.commands")
        }}</label>
        <input
          id="command-panel-input"
          ref="inputEl"
          v-model="query"
          class="cmd-input"
          type="text"
          spellcheck="false"
          autocomplete="off"
          :placeholder="t('commands.placeholder')"
          aria-controls="command-panel-list"
        />
        <div
          id="command-panel-list"
          ref="listEl"
          class="cmd-list"
          role="listbox"
          :aria-label="t('commands.results')"
        >
          <template v-if="flat.length">
            <section
              v-for="section in sections"
              :key="section.group"
              class="cmd-section"
            >
              <h2 class="cmd-group">
                <FluentIcon
                  :name="commandGroupIcon(section.group)"
                  size="sm"
                  class="cmd-group-icon"
                />
                {{ groupLabel(section.group) }}
              </h2>
              <button
                v-for="item in section.items"
                :key="item.command.id"
                type="button"
                role="option"
                class="cmd-item fluent-list-row"
                :class="{
                  'cmd-item--active':
                    flatIndexFor(item.command.id) === activeIndex,
                }"
                :data-cmd-index="flatIndexFor(item.command.id)"
                :aria-selected="flatIndexFor(item.command.id) === activeIndex"
                @mouseenter="activeIndex = flatIndexFor(item.command.id)"
                @click="runCommand(item.command)"
              >
                <FluentIcon
                  :name="commandGroupIcon(item.command.group)"
                  size="sm"
                  class="cmd-item-icon"
                />
                <span class="cmd-item-title">{{ item.command.title }}</span>
              </button>
            </section>
          </template>
          <p v-else class="cmd-empty">{{ t("commands.empty") }}</p>
        </div>
        <p class="cmd-hint">{{ t("commands.hint") }}</p>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.cmd-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 12vh;
  background: var(--fluent-overlay-scrim);
}

.cmd-panel {
  width: min(560px, calc(100vw - 32px));
  max-height: min(420px, 70vh);
  display: flex;
  flex-direction: column;
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-lg);
  background: var(--fluent-color-fill-control);
  box-shadow: var(--fluent-shadow-flyout);
  overflow: hidden;
}

.cmd-label {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

.cmd-input {
  flex-shrink: 0;
  height: var(--fluent-control-height-comfortable);
  min-height: var(--fluent-control-height-comfortable);
  margin: var(--fluent-space-md);
  margin-bottom: var(--fluent-space-sm);
  padding-inline: var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
  color: var(--fluent-color-text-primary);
  font: inherit;
}

.cmd-input:focus {
  outline: 2px solid var(--fluent-color-stroke-focus);
  outline-offset: 1px;
}

.cmd-list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 0 var(--fluent-space-sm) var(--fluent-space-sm);
}

.cmd-section + .cmd-section {
  margin-top: var(--fluent-space-sm);
}

.cmd-group {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  margin: var(--fluent-space-xs) var(--fluent-space-sm);
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.cmd-group-icon {
  color: var(--fluent-color-fill-accent);
}

.cmd-item {
  width: 100%;
  justify-content: flex-start;
  gap: var(--fluent-space-sm);
  border: 0;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.cmd-item-icon {
  color: var(--fluent-color-text-secondary);
  flex-shrink: 0;
}

.cmd-item--active .cmd-item-icon {
  color: var(--fluent-color-fill-accent);
}

.cmd-item:hover {
  background: var(--fluent-color-fill-control-hover);
}

.cmd-item--active {
  background: var(--fluent-color-fill-selected);
  outline: 1px solid var(--fluent-color-fill-accent);
  outline-offset: -1px;
}

.cmd-item-title {
  font-size: var(--fluent-font-size-body);
}

.cmd-empty {
  margin: var(--fluent-space-lg);
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-body);
}

.cmd-hint {
  flex-shrink: 0;
  margin: 0;
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-top: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
</style>
