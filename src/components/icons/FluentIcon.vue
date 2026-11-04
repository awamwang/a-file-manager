<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed } from "vue";
import { FLUENT_GLYPHS, type FluentGlyphId } from "./glyphs";

const props = withDefaults(
  defineProps<{
    name: FluentGlyphId;
    size?: "sm" | "md" | "lg";
    label?: string;
    /** Force SVG even on Windows (tests / debugging). */
    forceSvg?: boolean;
  }>(),
  {
    size: "md",
    forceSvg: false,
  },
);

const preferSegoe = computed(() => {
  if (props.forceSvg) return false;
  if (typeof navigator === "undefined") return false;
  return /Windows/i.test(navigator.userAgent || "");
});

const glyph = computed(() => FLUENT_GLYPHS[props.name]);

const decorative = computed(() => !props.label);
</script>

<template>
  <span
    class="fluent-icon"
    :class="[`fluent-icon--${size}`, { 'fluent-icon--segoe': preferSegoe }]"
    :role="decorative ? undefined : 'img'"
    :aria-hidden="decorative ? 'true' : undefined"
    :aria-label="label"
  >
    <span v-if="preferSegoe" class="fluent-icon__glyph">{{ glyph.codepoint }}</span>
    <svg
      v-else
      class="fluent-icon__svg"
      viewBox="0 0 16 16"
      width="1em"
      height="1em"
      focusable="false"
      aria-hidden="true"
    >
      <path v-for="(d, index) in glyph.paths" :key="index" :d="d" fill="currentColor" />
    </svg>
  </span>
</template>

<style scoped>
.fluent-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  width: 1em;
  height: 1em;
  line-height: 1;
  color: inherit;
  vertical-align: -0.125em;
}

.fluent-icon--sm {
  font-size: 14px;
}

.fluent-icon--md {
  font-size: 16px;
}

.fluent-icon--lg {
  font-size: 20px;
}

.fluent-icon__glyph {
  font-family: "Segoe Fluent Icons", "Segoe MDL2 Assets", sans-serif;
  font-style: normal;
  font-weight: 400;
  font-size: 1em;
  line-height: 1;
  speak: never;
  -webkit-font-smoothing: antialiased;
}

.fluent-icon__svg {
  display: block;
  width: 1em;
  height: 1em;
}
</style>
