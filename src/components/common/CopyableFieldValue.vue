<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/** Click-to-copy for long machine-generated field values (e.g. magnet links). */
import { onBeforeUnmount, ref } from "vue";
import FluentIcon from "../icons/FluentIcon.vue";

const props = withDefaults(
  defineProps<{
    value: string;
    label?: string;
  }>(),
  { label: "" },
);

const copied = ref(false);
let resetTimer: ReturnType<typeof setTimeout> | null = null;

async function copyValue() {
  if (!props.value) return;
  try {
    await navigator.clipboard.writeText(props.value);
    copied.value = true;
    if (resetTimer !== null) clearTimeout(resetTimer);
    resetTimer = setTimeout(() => {
      copied.value = false;
      resetTimer = null;
    }, 1400);
  } catch {
    /* Clipboard may be blocked; the original value stays selectable in the field. */
  }
}

onBeforeUnmount(() => {
  if (resetTimer !== null) clearTimeout(resetTimer);
});
</script>

<template>
  <button
    type="button"
    class="copyable-field-value"
    :title="label"
    :aria-label="label"
    @click.stop="copyValue"
  >
    <span class="copyable-field-value-text">{{ value }}</span>
    <FluentIcon :name="copied ? 'check' : 'copy'" size="sm" />
  </button>
</template>

<style scoped>
.copyable-field-value {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  max-width: 100%;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.copyable-field-value:hover,
.copyable-field-value:focus-visible {
  color: var(--fluent-color-fill-accent);
}

.copyable-field-value-text {
  min-width: 0;
  overflow-wrap: anywhere;
}

.copyable-field-value :deep(.fluent-icon) {
  flex: 0 0 auto;
}
</style>
