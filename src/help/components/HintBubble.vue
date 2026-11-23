<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * Getting-started hint (L2): one sentence saying what this place is, plus an
 * opt-in button that starts the walkthrough only when the user clicks it.
 * The hint never starts a tour by itself and never checks progress to do so.
 */
import { computed, onActivated, onDeactivated, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute } from "vue-router";
import { getHelpEntry, tourRegistry } from "../registry.ts";
import { isSeen, markSeen } from "../progress.ts";
import { startTour } from "../tourState.ts";
import { isHintPageActive } from "../hintRouteScope.ts";

const props = defineProps<{
  entryId: string;
  /** Tour played by the opt-in button; falls back to the entry's tourId. */
  tourId?: string;
}>();

const { t } = useI18n();
const route = useRoute();
const visible = ref(false);

const entry = computed(() => getHelpEntry(props.entryId));
const hintKey = computed<string>(() => entry.value?.hintKey ?? "");
/** `hintEnabled` defaults to true; a missing entry is a safe no-op. */
const hintAllowed = computed<boolean>(() => entry.value?.hintEnabled !== false);
/** `tourEnabled` defaults to true and gates the opt-in button too. */
const tourAllowed = computed<boolean>(() => entry.value?.tourEnabled !== false);
const resolvedTourId = computed<string>(() => props.tourId || entry.value?.tourId || "");
/** An unknown tour id has no steps, so the button is not offered at all. */
const canTryTour = computed<boolean>(
  () => tourAllowed.value && Boolean(tourRegistry[resolvedTourId.value]?.steps.length),
);

/**
 * A hint belongs to the functional page it describes, not to the shell.
 * Teleported to `body`, the bubble would otherwise outlive the view that owns
 * it (NavigatorView is keep-alive cached, so this instance is never unmounted).
 */
const pageActive = computed<boolean>(() => isHintPageActive(entry.value, route.path));

/**
 * Visibility is derived afresh on mount, on every route change and on
 * keep-alive reactivation — never latched on first mount. Still unseen-only:
 * dismiss marks the entry seen, so this stays false afterwards.
 */
function refreshVisibility(): void {
  if (!hintAllowed.value) return;
  visible.value = pageActive.value && Boolean(hintKey.value) && !isSeen(props.entryId);
}

onMounted(() => {
  refreshVisibility();
});

/** Watchers stay live on deactivated keep-alive views, so leaving the page hides at once. */
watch(pageActive, () => {
  refreshVisibility();
});

/** KeepAlive deactivation: the exact moment the source page stops being shown. */
onDeactivated(() => {
  visible.value = false;
});

/** An unseen hint returns when its cached page is activated again. */
onActivated(() => {
  refreshVisibility();
});

function dismiss(): void {
  markSeen(props.entryId);
  visible.value = false;
}

/**
 * Explicit user request only: remember the hint as seen, close it, then play
 * the resolved tour with progress recording (a real guided walkthrough).
 */
function tryTour(): void {
  if (!canTryTour.value) return;
  dismiss();
  const definition = tourRegistry[resolvedTourId.value];
  if (!definition) return;
  // The caller is already on the target view; no navigation, no delayed launch.
  startTour(resolvedTourId.value, definition.steps, true);
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="hint-bubble fluent-surface"
      role="status"
      :aria-label="t('help.hintLabel')"
    >
      <div class="hint-title">{{ t("help.hintLabel") }}</div>
      <p class="hint-body">{{ t(hintKey) }}</p>
      <div class="hint-actions">
        <button
          v-if="canTryTour"
          type="button"
          class="fluent-control hint-try"
          @click="tryTour"
        >
          {{ t("help.hintTryTour") }}
        </button>
        <button type="button" class="fluent-control" @click="dismiss">
          {{ t("help.hintDismiss") }}
        </button>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.hint-bubble {
  position: fixed;
  right: var(--fluent-space-lg);
  bottom: var(--fluent-space-lg);
  z-index: 1300;
  width: min(24rem, calc(100vw - 32px));
  padding: var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  box-shadow: var(--fluent-shadow-flyout);
  color: var(--fluent-color-text-primary);
}

.hint-title {
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-fill-accent);
}

.hint-body {
  margin: var(--fluent-space-sm) 0 var(--fluent-space-md);
  font-size: var(--fluent-font-size-body);
  line-height: 1.6;
}

.hint-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--fluent-space-sm);
}

.hint-try {
  background: var(--fluent-color-fill-accent);
  border-color: var(--fluent-color-fill-accent);
  color: var(--fluent-color-text-on-accent, #fff);
}
</style>
