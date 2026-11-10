<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Isolates iconByPath reactivity so one thumb arrival does not repaint the whole list (AIA-278). -->
<script setup lang="ts">
import { computed } from "vue";
import { iconUrlFor } from "../../state/shellIcons";

const props = defineProps<{
  path: string;
  size: number;
  isDir: boolean;
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
</script>

<template>
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
</template>

<style scoped>
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

@keyframes shell-icon-fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}
</style>
