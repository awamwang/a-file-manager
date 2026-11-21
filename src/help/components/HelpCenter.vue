<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * Help & Guidance Center (ADR 0021): the single hub for L1 articles, L2 hints
 * and L3 guided tours. All copy is i18n; replay never writes progress.
 */
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import DOMPurify from "dompurify";
import { marked } from "marked";
import {
  HELP_CATEGORIES,
  applicableHelpEntries,
  articleMarkdown,
  categoryKey,
  getHelpEntry,
  tourForEntry,
} from "../registry.ts";
import { resetAllProgress } from "../progress.ts";
import { startTour, startTourWhenAnchorReady } from "../tourState.ts";
import type { HelpCategory, HelpEntry } from "../types.ts";

const props = withDefaults(
  defineProps<{
    initialEntry?: string;
    /** Deep-link request to start the entry's tour after opening it. */
    autoTour?: boolean;
  }>(),
  { initialEntry: "", autoTour: false },
);

const { t } = useI18n();
const router = useRouter();

const entries = computed<readonly HelpEntry[]>(() => applicableHelpEntries("/help"));
const expandedId = ref<string>(props.initialEntry && getHelpEntry(props.initialEntry) ? props.initialEntry : "");
const resetConfirming = ref(false);
const resetNotice = ref(false);

const groups = computed<{ category: HelpCategory; label: string; entries: readonly HelpEntry[] }[]>(() =>
  HELP_CATEGORIES.map((category) => ({
    category,
    label: t(categoryKey(category)),
    entries: entries.value.filter((entry) => entry.category === category),
  })).filter((group) => group.entries.length > 0),
);

const activeEntry = computed<HelpEntry | undefined>(() =>
  expandedId.value ? getHelpEntry(expandedId.value) : undefined,
);

/** `tourEnabled` defaults to true; L1 articles are never gated by it. */
function tourAllowed(entry: HelpEntry): boolean {
  return entry.tourEnabled !== false;
}

const articleHtml = computed<string>(() => {
  if (!activeEntry.value) return "";
  const parsed = marked.parse(articleMarkdown(activeEntry.value.articlePath), {
    async: false,
    gfm: true,
    breaks: false,
  });
  return DOMPurify.sanitize(parsed, {
    FORBID_TAGS: ["style", "form", "input", "iframe"],
    FORBID_ATTR: ["style", "onerror", "onload", "onclick"],
  });
});

function toggleEntry(id: string): void {
  expandedId.value = expandedId.value === id ? "" : id;
  resetConfirming.value = false;
  resetNotice.value = false;
}

/** Replay is intentionally progress-neutral (R2-2). */
function replay(entry: HelpEntry): void {
  if (!tourAllowed(entry)) return;
  const tour = tourForEntry(entry);
  if (!tour) return;
  // Lazy views are not in the DOM yet at /help; wait for the first anchor
  // before starting so GuidedTour cannot skip the whole tour.
  startTourWhenAnchorReady(tour.steps[0]?.anchor ?? "", entry.route, () => {
    startTour(tour.id, tour.steps, false);
  });
  void router.push(entry.route);
}

function confirmReset(): void {
  resetAllProgress();
  resetConfirming.value = false;
  resetNotice.value = true;
}

watch(
  () => props.initialEntry,
  (id) => {
    if (id && getHelpEntry(id)) expandedId.value = id;
  },
);

function maybeAutoTour(): void {
  if (!props.autoTour || !activeEntry.value) return;
  if (!tourAllowed(activeEntry.value)) return;
  replay(activeEntry.value);
}

onMounted(maybeAutoTour);
</script>

<template>
  <div class="help-center">
    <div class="help-intro">
      <h2 class="help-section-title">{{ t("help.browseTitle") }}</h2>
      <p class="help-section-meta">{{ t("help.browseMeta") }}</p>
    </div>

    <div v-for="group in groups" :key="group.category" class="help-group">
      <h3 class="help-group-title">{{ group.label }}</h3>
      <div class="help-group-list">
        <article
          v-for="entry in group.entries"
          :key="entry.id"
          class="fluent-surface help-item"
          :class="{ 'help-item--open': expandedId === entry.id }"
        >
          <button
            type="button"
            class="help-item-head"
            :aria-expanded="expandedId === entry.id"
            @click="toggleEntry(entry.id)"
          >
            <span class="help-item-title">{{ t(entry.titleKey) }}</span>
            <span class="help-item-chevron" aria-hidden="true">
              {{ expandedId === entry.id ? "−" : "+" }}
            </span>
          </button>
          <div v-if="expandedId === entry.id" class="help-item-body">
            <section class="help-block">
              <h4 class="help-block-title">{{ t("help.hintLabel") }}</h4>
              <p class="help-block-text">{{ entry.hintKey ? t(entry.hintKey) : "" }}</p>
            </section>
            <section class="help-block">
              <h4 class="help-block-title">{{ t("help.articleLabel") }}</h4>
              <!-- eslint-disable-next-line vue/no-v-html -- sanitized with DOMPurify above -->
              <div class="help-article" v-html="articleHtml"></div>
            </section>
            <section class="help-block">
              <h4 class="help-block-title">{{ t("help.tourLabel") }}</h4>
              <template v-if="tourAllowed(entry)">
                <p class="help-block-text">{{ t("help.replayHint") }}</p>
                <button
                  v-if="tourForEntry(entry)"
                  type="button"
                  class="fluent-control fluent-control--accent"
                  @click="replay(entry)"
                >
                  {{ t("help.replay") }}
                </button>
                <p v-else class="help-block-text">{{ t("help.noTour") }}</p>
              </template>
              <p v-else class="help-block-text">{{ t("help.tourDisabled") }}</p>
            </section>
          </div>
        </article>
      </div>
    </div>

    <footer class="help-footer">
      <template v-if="!resetConfirming">
        <button type="button" class="fluent-control" @click="resetConfirming = true">
          {{ t("help.reset") }}
        </button>
      </template>
      <template v-else>
        <div class="help-confirm" role="group" :aria-label="t('help.resetTitle')">
          <div>
            <strong>{{ t("help.resetConfirm") }}</strong>
            <p class="help-confirm-meta">{{ t("help.resetHint") }}</p>
          </div>
          <button type="button" class="fluent-control" @click="resetConfirming = false">
            {{ t("help.resetCancel") }}
          </button>
          <button type="button" class="fluent-control fluent-control--accent" @click="confirmReset">
            {{ t("help.resetConfirmYes") }}
          </button>
        </div>
      </template>
      <p v-if="resetNotice" class="help-reset-notice" role="status">{{ t("help.resetDone") }}</p>
    </footer>
  </div>
</template>

<style scoped>
.help-center {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-lg);
  max-width: 60rem;
}

.help-section-title {
  margin: 0;
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.help-section-meta {
  margin: var(--fluent-space-xs) 0 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-body);
}

.help-group {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.help-group-title {
  margin: 0;
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-secondary);
}

.help-group-list {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.help-item {
  overflow: hidden;
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
}

.help-item--open {
  border-color: var(--fluent-color-fill-accent);
}

.help-item-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.help-item-title {
  font-weight: var(--fluent-font-weight-semibold);
}

.help-item-chevron {
  color: var(--fluent-color-fill-accent);
  font-size: 1.1em;
}

.help-item-body {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  padding: 0 var(--fluent-space-md) var(--fluent-space-md);
  border-top: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.help-block {
  padding-top: var(--fluent-space-md);
}

.help-block-title {
  margin: 0 0 var(--fluent-space-xs);
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-secondary);
}

.help-block-text {
  margin: 0 0 var(--fluent-space-sm);
  line-height: 1.6;
}

.help-article {
  max-height: 30rem;
  overflow-y: auto;
  padding-right: var(--fluent-space-sm);
  font-size: var(--fluent-font-size-body);
  line-height: 1.7;
}

.help-article :deep(h1),
.help-article :deep(h2),
.help-article :deep(h3) {
  margin: 1em 0 0.5em;
  line-height: 1.3;
}

.help-article :deep(h1) { font-size: 1.4em; }
.help-article :deep(h2) { font-size: 1.2em; }
.help-article :deep(h3) { font-size: 1.1em; }

.help-article :deep(p),
.help-article :deep(ul),
.help-article :deep(ol),
.help-article :deep(blockquote) {
  margin: 0.5em 0;
}

.help-article :deep(ul),
.help-article :deep(ol) {
  padding-left: 1.5em;
}

.help-article :deep(blockquote) {
  padding-left: 0.8em;
  border-left: 3px solid var(--fluent-color-stroke-divider);
  color: var(--fluent-color-text-secondary);
}

.help-footer {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--fluent-space-sm);
  padding-top: var(--fluent-space-md);
  border-top: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.help-confirm {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  flex-wrap: wrap;
}

.help-confirm-meta {
  margin: 2px 0 0;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.help-reset-notice {
  margin: 0;
  color: var(--fluent-color-fill-accent);
  font-size: var(--fluent-font-size-caption);
}
</style>
