<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Isolates iconByPath reactivity so one thumb arrival does not repaint the whole list (AIA-278). -->
<script setup lang="ts">
import { computed } from "vue";
import { iconUrlFor } from "../../state/shellIcons";

const props = defineProps<{
  path: string;
  size: number;
  isDir: boolean;
  /** Symlink / junction / `.lnk` — draws a Windows-style shortcut arrow overlay. */
  isLink?: boolean;
}>();

const src = computed(() => iconUrlFor(props.path, props.size));

/** 显式宽高，避免依赖父级 scoped 穿透导致缩略图被压成细条。 */
const boxStyle = computed(() => {
  const px = `${Math.max(1, props.size)}px`;
  return {
    width: px,
    height: px,
    minWidth: px,
    minHeight: px,
  };
});

/** 角标随图标尺寸缩放：宽高为图标的 22%（原 44% 的一半），小图标保底 4px。 */
const badgeStyle = computed(() => {
  const edge = Math.max(1, props.size);
  const px = `${Math.max(4, Math.round(edge * 0.22))}px`;
  return { width: px, height: px };
});
</script>

<template>
  <span class="shell-icon-frame" :style="boxStyle">
    <img
      v-if="src"
      class="shell-icon"
      :src="src"
      alt=""
      decoding="async"
      :width="size"
      :height="size"
      :style="boxStyle"
    />
    <span
      v-else
      class="shell-icon-slot"
      aria-hidden="true"
      :style="boxStyle"
    />
    <span
      v-if="isLink"
      class="shell-icon-link-badge"
      aria-hidden="true"
      :style="badgeStyle"
    >
      <svg viewBox="0 0 16 16" width="100%" height="100%">
        <!-- 白色底板 + 描边，保证在深色/彩色应用图标上都清晰 -->
        <path
          d="M1.6 1.6h8.2l4.6 4.6v8.2H1.6z"
          fill="#ffffff"
          stroke="rgba(0,0,0,0.35)"
          stroke-width="0.9"
          stroke-linejoin="round"
        />
        <path d="M9.6 1.8v4.4H14" fill="none" stroke="rgba(0,0,0,0.28)" stroke-width="0.9" />
        <!-- 经典快捷方式箭头 -->
        <path
          d="M4.4 11.6 10.6 5.4M6.9 5.4h3.7v3.7"
          fill="none"
          stroke="#1a6fc4"
          stroke-width="1.7"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </span>
  </span>
</template>

<style scoped>
.shell-icon-frame {
  position: relative;
  display: block;
  flex-shrink: 0;
}

.shell-icon {
  display: block;
  object-fit: contain;
  flex-shrink: 0;
  animation: shell-icon-fade-in 120ms ease-out;
}

.shell-icon-slot {
  display: block;
  flex-shrink: 0;
  border-radius: var(--fluent-radius-sm);
  background: color-mix(in srgb, var(--fluent-color-text-secondary) 12%, transparent);
}

.shell-icon-link-badge {
  position: absolute;
  left: -12%;
  /* 上移到图标框内，避免向下突出盖住条目名称 */
  bottom: 1%;
  display: block;
  pointer-events: none;
  line-height: 0;
  filter: drop-shadow(0 1px 1px rgba(0, 0, 0, 0.25));
}

@keyframes shell-icon-fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}
</style>
