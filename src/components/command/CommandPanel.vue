<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import router from "../../router/index.ts";
import type { AppCommand } from "../../commands/types.ts";
import {
  closeCommandPanel,
  commandPanelOpen,
} from "../../state/commandPanel.ts";
import { toastError, toastInfo } from "../../state/toast.ts";
import { buildSearchSources } from "../../search/registry.ts";
import {
  useSearchPalette,
  type PaletteGroup,
  type PaletteRow,
} from "../../search/useSearchPalette.ts";
import { iconUrlFor } from "../../state/shellIcons.ts";
import type {
  SearchActivateNotice,
  SearchItemType,
  SearchSource,
  SearchSourceId,
  SearchResultItem,
} from "../../search/types.ts";
import FluentIcon from "../icons/FluentIcon.vue";
import {
  commandGroupIcon,
  type FluentGlyphId,
} from "../icons/glyphs.ts";

const props = defineProps<{
  commands: AppCommand[];
}>();

const { t } = useI18n();

const sources = buildSearchSources({
  router,
  commands: () => props.commands,
  t: (key) => t(key),
});

const {
  query,
  activeIndex,
  fuzzy,
  scopeChips,
  groups,
  flat,
  toggleScope,
  toggleScopeAt,
  reset,
  rerun,
} = useSearchPalette(() => sources);

const inputEl = ref<HTMLInputElement | null>(null);
const listEl = ref<HTMLElement | null>(null);

/** 行首图标缺省映射（文件源接入后按条目覆盖）。 */
const ITEM_TYPE_ICON: Record<SearchItemType, FluentGlyphId> = {
  command: "commands",
  file: "document",
  folder: "folder",
  project: "board",
  workgroup: "viewAll",
  tool: "settings",
  capability: "flash",
  bookmark: "bookmark",
  extension: "puzzle",
};

/** 分组图标缺省映射。 */
const SOURCE_ICON: Record<SearchSourceId, FluentGlyphId> = {
  commands: "commands",
  files: "search",
  projects: "board",
  workgroups: "viewAll",
  tools: "settings",
  bookmarks: "bookmark",
  extensions: "puzzle",
};

function itemIcon(item: SearchResultItem): FluentGlyphId {
  if (item.icon) return item.icon;
  if (item.type === "command" && item.group) {
    return commandGroupIcon(item.group);
  }
  return ITEM_TYPE_ICON[item.type] ?? "commands";
}

/** 文件/文件夹行的系统壳图标 URL（无则回落 Fluent 图标）。 */
function rowShellIcon(item: SearchResultItem): string | null {
  return item.iconPath ? iconUrlFor(item.iconPath) : null;
}

function rowTitle(group: PaletteGroup, row: PaletteRow): string {
  if (row.viewAll) return t(group.source.viewAll?.labelKey ?? "");
  return row.item.title;
}

function itemSubtitle(item: SearchResultItem): string | null {
  if (item.subtitleKey) return t(item.subtitleKey, item.subtitleParams ?? {});
  return item.subtitle ?? null;
}

function itemTypeLabel(item: SearchResultItem): string {
  return t(`searchPalette.types.${item.type}`);
}

function statusActionLabel(source: SearchSource): string | null {
  if (!source.unavailableAction || !source.unavailableActionLabelKey) {
    return null;
  }
  return t(source.unavailableActionLabelKey);
}

watch(commandPanelOpen, async (open) => {
  if (open) {
    reset();
    await nextTick();
    inputEl.value?.focus();
  }
});

watch(
  () => props.commands,
  () => rerun(),
);

function onBackdropClick(event: MouseEvent) {
  if (event.target === event.currentTarget) {
    closeCommandPanel();
  }
}

async function runItem(
  item: SearchResultItem,
  ctrl = false,
): Promise<void> {
  closeCommandPanel();
  const notice = await item.activate({ ctrl });
  if (notice) {
    showNotice(notice);
  }
}

function showNotice(notice: SearchActivateNotice): void {
  if (notice.kind === "error") {
    toastError(t(notice.toastKey));
  } else {
    toastInfo(t(notice.toastKey));
  }
}

async function onEnter(ctrl: boolean): Promise<void> {
  const row = flat.value[activeIndex.value];
  if (row) await runItem(row.item, ctrl);
}

async function runStatusAction(source: SearchSource): Promise<void> {
  if (!source.unavailableAction) return;
  // 刷新类动作保留面板并重跑；导航类动作关闭面板。
  if (!source.retainPanelOnStatusAction) {
    closeCommandPanel();
  }
  await source.unavailableAction();
  if (source.retainPanelOnStatusAction) {
    rerun();
  }
}

function moveActive(delta: number) {
  const len = flat.value.length;
  if (!len) return;
  activeIndex.value = (activeIndex.value + delta + len) % len;
  void nextTick(() => {
    const selected = listEl.value?.querySelector<HTMLElement>(
      `[data-palette-index="${activeIndex.value}"]`,
    );
    selected?.scrollIntoView({ block: "nearest" });
  });
}

function onPanelKeydown(event: KeyboardEvent) {
  // 面板打开时优先于全局 Alt+数字（工作组切换）。
  if (
    event.altKey &&
    !event.ctrlKey &&
    !event.metaKey &&
    event.key >= "1" &&
    event.key <= "9"
  ) {
    event.preventDefault();
    event.stopPropagation();
    toggleScopeAt(Number(event.key));
    return;
  }
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
    void onEnter(event.ctrlKey);
  }
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
        :aria-label="t('searchPalette.title')"
        @mousedown.stop
      >
        <label class="cmd-label" for="command-panel-input">{{
          t("searchPalette.title")
        }}</label>
        <input
          id="command-panel-input"
          ref="inputEl"
          v-model="query"
          class="cmd-input"
          type="text"
          spellcheck="false"
          autocomplete="off"
          :placeholder="t('searchPalette.placeholder')"
          aria-controls="command-panel-list"
        />
        <div
          v-if="scopeChips.length"
          class="cmd-scopes"
          role="group"
          :aria-label="t('searchPalette.scopesLabel')"
        >
          <button
            v-for="chip in scopeChips"
            :key="chip.source.id"
            type="button"
            class="cmd-scope-chip"
            :class="{ 'cmd-scope-chip--active': chip.active }"
            :aria-pressed="chip.active"
            @click="toggleScope(chip.source.id)"
          >
            {{ t(chip.source.labelKey) }}
            <kbd class="cmd-scope-chip-key">Alt+{{ chip.index }}</kbd>
          </button>
        </div>
        <div
          id="command-panel-list"
          ref="listEl"
          class="cmd-list"
          role="listbox"
          :aria-label="t('searchPalette.results')"
        >
          <template v-if="groups.length">
            <section
              v-for="group in groups"
              :key="group.source.id"
              class="cmd-section"
            >
              <h2 class="cmd-group">
                <FluentIcon
                  :name="SOURCE_ICON[group.source.id] ?? 'commands'"
                  size="sm"
                  class="cmd-group-icon"
                />
                {{ t(group.source.labelKey) }}
              </h2>
              <button
                v-for="row in group.rows"
                :key="row.item.key"
                type="button"
                role="option"
                class="cmd-item fluent-list-row"
                :class="{
                  'cmd-item--active': row.index === activeIndex,
                  'cmd-view-all': row.viewAll,
                }"
                :data-palette-index="row.index"
                :aria-selected="row.index === activeIndex"
                @mouseenter="activeIndex = row.index"
                @click="runItem(row.item)"
              >
                <img
                  v-if="!row.viewAll && rowShellIcon(row.item)"
                  class="cmd-item-shell-icon"
                  :src="rowShellIcon(row.item)!"
                  alt=""
                  width="16"
                  height="16"
                />
                <FluentIcon
                  v-else
                  :name="itemIcon(row.item)"
                  size="sm"
                  class="cmd-item-icon"
                />
                <span class="cmd-item-main">
                  <span class="cmd-item-title">{{ rowTitle(group, row) }}</span>
                  <span
                    v-if="!row.viewAll && itemSubtitle(row.item)"
                    class="cmd-item-subtitle"
                  >{{ itemSubtitle(row.item) }}</span>
                </span>
                <span v-if="!row.viewAll" class="cmd-item-type">{{
                  itemTypeLabel(row.item)
                }}</span>
              </button>
              <p
                v-if="group.state.status === 'loading' && !group.rows.length"
                class="cmd-source-status"
              >
                {{ t("searchPalette.status.loading") }}
              </p>
              <p
                v-else-if="group.state.status === 'unavailable'"
                class="cmd-source-status"
              >
                {{ t(group.source.unavailableKey) }}
                <button
                  v-if="statusActionLabel(group.source)"
                  type="button"
                  class="cmd-status-action"
                  @click="runStatusAction(group.source)"
                >
                  {{ statusActionLabel(group.source) }}
                </button>
              </p>
              <p
                v-else-if="group.state.hintKey"
                class="cmd-source-status"
              >
                {{ t(group.state.hintKey) }}
                <button
                  v-if="statusActionLabel(group.source)"
                  type="button"
                  class="cmd-status-action"
                  @click="runStatusAction(group.source)"
                >
                  {{ statusActionLabel(group.source) }}
                </button>
              </p>
            </section>
          </template>
          <p v-else class="cmd-empty">{{ t("searchPalette.empty") }}</p>
        </div>
        <div class="cmd-footer">
          <p class="cmd-hint">{{ t("searchPalette.hint") }}</p>
          <label class="cmd-fuzzy">
            <input
              v-model="fuzzy"
              type="checkbox"
              class="cmd-fuzzy-box"
            />{{ t("searchPalette.fuzzy") }}
          </label>
        </div>
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
  max-height: min(460px, 70vh);
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
  margin-bottom: var(--fluent-space-xs);
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

.cmd-scopes {
  flex-shrink: 0;
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-xs);
  padding: 0 var(--fluent-space-md);
  margin-bottom: var(--fluent-space-sm);
}

.cmd-scope-chip {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  padding: 2px var(--fluent-space-sm);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: 999px;
  background: transparent;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  cursor: pointer;
}

.cmd-scope-chip:hover {
  background: var(--fluent-color-fill-control-hover);
}

.cmd-scope-chip--active {
  border-color: var(--fluent-color-fill-accent);
  color: var(--fluent-color-text-primary);
  background: var(--fluent-color-fill-selected);
}

.cmd-scope-chip-key {
  font: inherit;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
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

.cmd-item-shell-icon {
  width: 16px;
  height: 16px;
  object-fit: contain;
  flex-shrink: 0;
}

.cmd-view-all .cmd-item-title {
  color: var(--fluent-color-fill-accent);
  font-size: var(--fluent-font-size-caption);
}

.cmd-view-all .cmd-item-icon {
  color: var(--fluent-color-fill-accent);
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

.cmd-item-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.cmd-item-title {
  font-size: var(--fluent-font-size-body);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cmd-item-subtitle {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cmd-item-type {
  flex-shrink: 0;
  align-self: center;
  padding: 1px var(--fluent-space-xs);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: 999px;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.cmd-item--active .cmd-item-type {
  border-color: var(--fluent-color-fill-accent);
  color: var(--fluent-color-text-primary);
}

.cmd-source-status {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  margin: var(--fluent-space-xs) var(--fluent-space-sm);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.cmd-status-action {
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--fluent-color-fill-accent);
  font: inherit;
  cursor: pointer;
  text-decoration: underline;
}

.cmd-empty {
  margin: var(--fluent-space-lg);
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-body);
}

.cmd-footer {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  border-top: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  padding: 0 var(--fluent-space-md);
}

.cmd-hint {
  flex: 1;
  margin: 0;
  padding: var(--fluent-space-sm) 0;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.cmd-fuzzy {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  padding: var(--fluent-space-xxs) 0;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  white-space: nowrap;
  cursor: pointer;
  user-select: none;
}

.cmd-fuzzy:hover {
  color: var(--fluent-color-text-primary);
}

.cmd-fuzzy-box {
  margin: 0;
  accent-color: var(--fluent-color-fill-accent);
}
</style>
