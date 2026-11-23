<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import {
  activeActionTip,
  hideActionTip,
  markActionTipInteraction,
} from "../../shortcuts/actionTip.ts";
import { shortcutOverridesVersion } from "../../shortcuts/config.ts";
import { effectiveChordLabelsForId } from "../../shortcuts/registry.ts";

/**
 * Re-resolve on every override write: the same hovered button must never keep
 * showing a stale chord after the user remaps that function in Settings.
 */
const tip = computed(() => {
  void shortcutOverridesVersion.value;
  const state = activeActionTip.value;
  if (!state) return null;
  return {
    ...state,
    chords: effectiveChordLabelsForId(state.actionId),
  };
});

function dismiss() {
  hideActionTip();
}

/**
 * Only genuine key/pointer input unlocks focus-driven tips. A WebView can fire
 * `focus` on its first control when the macOS window is revealed, and startup
 * scroll restoration is not user input either — so neither may mark here.
 */
function onUserInput() {
  markActionTipInteraction();
  dismiss();
}

onMounted(() => {
  window.addEventListener("pointerdown", onUserInput, true);
  window.addEventListener("keydown", onUserInput, true);
  window.addEventListener("wheel", dismiss, true);
  window.addEventListener("scroll", dismiss, true);
});

onUnmounted(() => {
  window.removeEventListener("pointerdown", onUserInput, true);
  window.removeEventListener("keydown", onUserInput, true);
  window.removeEventListener("wheel", dismiss, true);
  window.removeEventListener("scroll", dismiss, true);
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="tip"
      class="action-tip"
      :class="{ 'action-tip--below': tip.below }"
      role="tooltip"
      :style="{ left: `${tip.left}px`, top: `${tip.top}px` }"
    >
      <span class="action-tip-label">{{ tip.label }}</span>
      <span v-if="tip.chords.length" class="action-tip-chords">
        <kbd v-for="chord in tip.chords" :key="chord" class="action-tip-kbd">{{
          chord
        }}</kbd>
      </span>
    </div>
  </Teleport>
</template>

<style scoped>
.action-tip {
  position: fixed;
  z-index: 1400;
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  max-width: 28rem;
  padding: 6px 10px;
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  box-shadow: var(--fluent-shadow-flyout);
  color: var(--fluent-color-text-primary);
  font-size: var(--fluent-font-size-caption);
  white-space: nowrap;
  pointer-events: none;
  transform: translate(-50%, -100%);
}

.action-tip--below {
  transform: translate(-50%, 0);
}

.action-tip-label {
  font-weight: var(--fluent-font-weight-semibold);
}

.action-tip-chords {
  display: inline-flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
  min-width: 0;
}

.action-tip-kbd {
  padding: 1px 5px;
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-sm);
  background: var(--fluent-color-fill-subtle);
  font-family: var(--fluent-font-family-mono);
  font-size: var(--fluent-font-size-caption);
  line-height: 1.4;
}
</style>
