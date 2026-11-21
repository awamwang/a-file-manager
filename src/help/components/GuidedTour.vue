<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * Spotlight guided tour (ADR 0021, L3).
 *
 * The overlay never blocks the app: only the bubble receives pointer events,
 * so a `requireClick` step can really be completed by clicking its anchor.
 * Missing anchors advance immediately — a tour can never get stuck.
 */
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { markDone } from "../progress.ts";
import type { TourStep } from "../types.ts";

type Rect = { left: number; top: number; width: number; height: number; bottom: number };

const props = withDefaults(
  defineProps<{
    steps: readonly TourStep[];
    tourId?: string;
    /** Replay passes false; first-run playback passes true. */
    recordProgress?: boolean;
  }>(),
  { tourId: "", recordProgress: true },
);

const emit = defineEmits<{ finish: [] }>();
const { t } = useI18n();

const index = ref(0);
const rect = ref<Rect | null>(null);
const bubbleEl = ref<HTMLElement | null>(null);
const current = computed<TourStep | null>(() => props.steps[index.value] ?? null);

let pollTimer: number | undefined;
let missingPending = false;
let finished = false;

const HIGHLIGHT_PAD = 6;

const highlightStyle = computed<Record<string, string>>(() => {
  const box = rect.value;
  if (!box) {
    return { display: "none", left: "0px", top: "0px", width: "0px", height: "0px" };
  }
  return {
    display: "block",
    left: `${Math.max(0, box.left - HIGHLIGHT_PAD)}px`,
    top: `${Math.max(0, box.top - HIGHLIGHT_PAD)}px`,
    width: `${box.width + HIGHLIGHT_PAD * 2}px`,
    height: `${box.height + HIGHLIGHT_PAD * 2}px`,
  };
});

const bubbleStyle = computed<Record<string, string>>(() => {
  const box = rect.value;
  const width = 360;
  const estimatedHeight = 200;
  const viewportWidth = typeof window === "undefined" ? 1280 : window.innerWidth;
  const viewportHeight = typeof window === "undefined" ? 800 : window.innerHeight;
  const left = box
    ? Math.min(Math.max(12, box.left), Math.max(12, viewportWidth - width - 12))
    : Math.max(12, (viewportWidth - width) / 2);
  const below = box ? box.bottom + 14 : 80;
  const above = box ? box.top - estimatedHeight - 14 : 80;
  const top = below + estimatedHeight < viewportHeight ? below : Math.max(12, above);
  return { left: `${left}px`, top: `${top}px`, width: `${width}px` };
});

function anchorElement(anchor: string): HTMLElement | null {
  try {
    return document.querySelector<HTMLElement>(`[data-tour-id="${CSS.escape(anchor)}"]`);
  } catch {
    // A malformed anchor is treated the same as a missing one.
    return null;
  }
}

/** Advance one step; the watcher re-measures after the DOM settles. */
function advance(): void {
  if (finished) return;
  missingPending = false;
  if (index.value + 1 >= props.steps.length) {
    finish();
    return;
  }
  index.value += 1;
}

function goBack(): void {
  if (finished || index.value === 0) return;
  missingPending = false;
  index.value -= 1;
}

function finish(): void {
  if (finished) return;
  finished = true;
  if (props.recordProgress && props.tourId) markDone(props.tourId);
  emit("finish");
}

function measure(): void {
  if (finished || missingPending) return;
  const step = current.value;
  if (!step) {
    finish();
    return;
  }
  const element = anchorElement(step.anchor);
  if (!element) {
    missingPending = true;
    // Let the current paint finish before moving on.
    void nextTick(advance);
    return;
  }
  const box = element.getBoundingClientRect();
  if (box.width <= 0 && box.height <= 0) {
    missingPending = true;
    void nextTick(advance);
    return;
  }
  rect.value = {
    left: box.left,
    top: box.top,
    width: box.width,
    height: box.height,
    bottom: box.bottom,
  };
}

function onPointerDown(event: PointerEvent): void {
  const step = current.value;
  if (!step || !step.requireClick) return;
  const target = event.target;
  if (!(target instanceof Element)) return;
  if (target.closest(".tour-bubble")) return;
  const anchor = anchorElement(step.anchor);
  if (anchor && (target === anchor || anchor.contains(target))) {
    // Do not stop propagation: the anchor's own action must still run.
    advance();
  }
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    event.preventDefault();
    finish();
    return;
  }
  const step = current.value;
  if (!step || step.requireClick) return;
  if (event.key === "ArrowRight") {
    event.preventDefault();
    advance();
  } else if (event.key === "ArrowLeft") {
    event.preventDefault();
    goBack();
  }
}

function clearTimers(): void {
  if (pollTimer !== undefined) {
    window.clearInterval(pollTimer);
    pollTimer = undefined;
  }
}

watch(index, () => {
  rect.value = null;
  missingPending = false;
  void nextTick(measure);
});

onMounted(() => {
  void nextTick(measure);
  window.addEventListener("scroll", measure, true);
  window.addEventListener("resize", measure);
  window.addEventListener("pointerdown", onPointerDown, true);
  window.addEventListener("keydown", onKeydown, true);
  // Catches anchors that appear later (lazy panels, route transitions).
  pollTimer = window.setInterval(measure, 240);
});

onUnmounted(() => {
  clearTimers();
  window.removeEventListener("scroll", measure, true);
  window.removeEventListener("resize", measure);
  window.removeEventListener("pointerdown", onPointerDown, true);
  window.removeEventListener("keydown", onKeydown, true);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="current" class="tour-layer" data-tour-layer>
      <div v-if="rect" class="tour-highlight" :style="highlightStyle" aria-hidden="true"></div>
      <div v-else class="tour-dim" aria-hidden="true"></div>
      <div
        ref="bubbleEl"
        class="tour-bubble fluent-surface"
        :style="bubbleStyle"
        role="dialog"
        :aria-label="t(current.titleKey)"
      >
        <div class="tour-bubble-top">
          <span class="tour-counter">{{ t("tour.step", { current: index + 1, total: steps.length }) }}</span>
          <strong class="tour-title">{{ t(current.titleKey) }}</strong>
        </div>
        <p class="tour-body">{{ t(current.bodyKey) }}</p>
        <div class="tour-actions">
          <button type="button" class="fluent-control tour-skip" @click="finish">
            {{ t("tour.skip") }}
          </button>
          <span class="tour-spacer"></span>
          <button
            v-if="index > 0"
            type="button"
            class="fluent-control"
            @click="goBack"
          >
            {{ t("tour.previous") }}
          </button>
          <button
            type="button"
            class="fluent-control fluent-control--accent"
            :disabled="current.requireClick === true"
            @click="advance"
          >
            {{ current.requireClick ? t("tour.clickTarget") : index === steps.length - 1 ? t("tour.finish") : t("tour.next") }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.tour-layer {
  position: fixed;
  inset: 0;
  z-index: 1600;
  pointer-events: none;
}

.tour-highlight {
  position: fixed;
  border: 2px solid var(--fluent-color-fill-accent);
  border-radius: var(--fluent-radius-md);
  box-shadow:
    0 0 0 9999px color-mix(in srgb, var(--fluent-color-text-primary) 42%, transparent),
    0 0 18px color-mix(in srgb, var(--fluent-color-fill-accent) 38%, transparent);
  pointer-events: none;
  transition: left 140ms ease, top 140ms ease, width 140ms ease, height 140ms ease;
}

.tour-dim {
  position: fixed;
  inset: 0;
  background: color-mix(in srgb, var(--fluent-color-text-primary) 42%, transparent);
  pointer-events: none;
}

.tour-bubble {
  position: fixed;
  max-width: calc(100vw - 24px);
  padding: var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  box-shadow: var(--fluent-shadow-flyout);
  color: var(--fluent-color-text-primary);
  pointer-events: auto;
}

.tour-bubble-top {
  display: flex;
  align-items: baseline;
  gap: var(--fluent-space-sm);
}

.tour-counter {
  flex-shrink: 0;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.tour-title {
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.tour-body {
  margin: var(--fluent-space-sm) 0 var(--fluent-space-md);
  font-size: var(--fluent-font-size-body);
  line-height: 1.6;
  white-space: pre-line;
}

.tour-actions {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
}

.tour-spacer {
  flex: 1;
}

.tour-skip {
  color: var(--fluent-color-text-secondary);
}
</style>
