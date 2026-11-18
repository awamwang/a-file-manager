<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/** Shared "?" deep link used by every feature surface. */
import { computed } from "vue";
import { useI18n } from "vue-i18n";

const props = withDefaults(
  defineProps<{
    entryId: string;
    /** Ask the Help Center to replay the tour right after opening the entry. */
    tour?: boolean;
  }>(),
  { tour: false },
);

const { t } = useI18n();

const target = computed(() => ({
  path: "/help",
  query: { entry: props.entryId, ...(props.tour ? { tour: "1" } : {}) },
}));
</script>

<template>
  <RouterLink
    class="help-entry-button"
    :to="target"
    :title="t('help.openEntry')"
    :aria-label="t('help.openEntry')"
  >?</RouterLink>
</template>

<style scoped>
.help-entry-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  width: 22px;
  height: 22px;
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: 50%;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  line-height: 1;
  text-decoration: none;
}

.help-entry-button:hover,
.help-entry-button:focus-visible {
  border-color: var(--fluent-color-fill-accent);
  color: var(--fluent-color-fill-accent);
}
</style>
