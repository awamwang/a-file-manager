<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Special-entry glyph with an optional cloud badge for ordinary cloud files. -->
<script setup lang="ts">
import { computed } from "vue";
import FluentIcon from "../icons/FluentIcon.vue";
import type { FluentGlyphId } from "../icons/glyphs.ts";

const props = defineProps<{
  name: FluentGlyphId;
  size: number;
  /** Ordinary cloud file/folder: draw the small cloud origin badge. */
  cloudBadge?: boolean;
}>();

/** Badge scales with the row icon; ~44% edge, 6px floor for legibility. */
const badgeStyle = computed(() => {
  const edge = Math.max(1, props.size);
  const px = `${Math.max(6, Math.round(edge * 0.44))}px`;
  return { width: px, height: px };
});
</script>

<template>
  <span class="cloud-file-frame">
    <FluentIcon :name="name" class="entry-special-icon" />
    <span v-if="cloudBadge" class="cloud-file-badge" aria-hidden="true" :style="badgeStyle">
      <svg viewBox="0 0 16 16" width="100%" height="100%">
        <!-- White disc + outline keeps the badge readable on any type glyph. -->
        <circle cx="8" cy="8" r="7.2" fill="#ffffff" stroke="rgba(0,0,0,0.32)" stroke-width="1" />
        <path
          d="M4.3 11h7.5a2.35 2.35 0 0 0 .1-4.7 3.3 3.3 0 0 0-6.2-.7A2.75 2.75 0 0 0 4.3 11z"
          fill="#1a6fc4"
        />
      </svg>
    </span>
  </span>
</template>

<style scoped>
.cloud-file-frame {
  position: relative;
  display: inline-flex;
  flex-shrink: 0;
  line-height: 0;
}

.cloud-file-badge {
  position: absolute;
  right: -10%;
  bottom: 1%;
  display: block;
  pointer-events: none;
  line-height: 0;
  filter: drop-shadow(0 1px 1px rgba(0, 0, 0, 0.25));
}
</style>
