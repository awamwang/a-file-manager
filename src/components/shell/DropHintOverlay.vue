<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 文件拖动时的松开提示：跟随光标显示「松开剪切到此处 / 松开复制到此处」。
 * 由 state/dnd 的拖动会话驱动，标签条、列表、目录树、收藏夹共用同一浮层。
 */
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { dropHint } from "../../state/dnd";
import type { FluentGlyphId } from "../icons/glyphs";
import FluentIcon from "../icons/FluentIcon.vue";

const { t } = useI18n();
const hint = computed(() => dropHint.value);

const iconName = computed<FluentGlyphId>(() => {
  if (hint.value?.mode === "move") return "cut";
  if (hint.value?.mode === "link") return "link";
  return "copy";
});

const label = computed(() => {
  if (!hint.value) return "";
  if (hint.value.mode === "move") return t("navigator.dropHintMove");
  if (hint.value.mode === "link") return t("navigator.dropHintLink");
  return t("navigator.dropHintCopy");
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="hint"
      class="drop-hint"
      :class="`drop-hint--${hint.mode}`"
      :style="{ left: `${hint.x}px`, top: `${hint.y}px` }"
      role="status"
      aria-live="polite"
    >
      <span class="drop-hint__icon" aria-hidden="true">
        <FluentIcon :name="iconName" size="sm" />
      </span>
      <span class="drop-hint__text">
        <span class="drop-hint__label">{{ label }}</span>
        <span class="drop-hint__target" :title="hint.target">{{ hint.target }}</span>
      </span>
    </div>
  </Teleport>
</template>

<style scoped>
.drop-hint {
  position: fixed;
  z-index: 10050;
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  width: 220px;
  max-width: 220px;
  padding: 6px 10px;
  border: 1px solid var(--fluent-color-stroke-accent-soft);
  border-left-width: 3px;
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  box-shadow: var(--fluent-shadow-flyout);
  color: var(--fluent-color-text-primary);
  font-family: var(--fluent-font-family);
  font-size: var(--fluent-font-size-caption);
  line-height: 1.35;
  pointer-events: none;
  user-select: none;
}

.drop-hint--copy {
  border-left-color: var(--fluent-color-status-success);
}

.drop-hint--move {
  border-left-color: var(--fluent-color-status-info);
}

.drop-hint--link {
  border-left-color: var(--fluent-color-status-warning);
}

.drop-hint__icon {
  display: inline-flex;
  flex: 0 0 auto;
  color: var(--fluent-color-text-secondary);
}

.drop-hint__text {
  display: flex;
  min-width: 0;
  flex-direction: column;
}

.drop-hint__label {
  font-weight: var(--fluent-font-weight-semibold);
  white-space: nowrap;
}

.drop-hint__target {
  overflow: hidden;
  color: var(--fluent-color-text-secondary);
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
