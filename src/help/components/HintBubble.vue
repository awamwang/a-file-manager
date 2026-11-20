<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/** Getting-started hint (L2): shown once per feature, dismissible. */
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { getHelpEntry } from "../registry.ts";
import { isSeen, markSeen } from "../progress.ts";

const props = defineProps<{
  entryId: string;
}>();

const { t } = useI18n();
const visible = ref(false);

const entry = computed(() => getHelpEntry(props.entryId));
const hintKey = computed<string>(() => entry.value?.hintKey ?? "");
/** `hintEnabled` defaults to true; a missing entry is a safe no-op. */
const hintAllowed = computed<boolean>(() => entry.value?.hintEnabled !== false);

onMounted(() => {
  if (!hintAllowed.value) return;
  if (hintKey.value && !isSeen(props.entryId)) visible.value = true;
});

function dismiss(): void {
  markSeen(props.entryId);
  visible.value = false;
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
  box-shadow: var(--fluent-shadow-flyout, 0 12px 32px rgba(0, 0, 0, 0.24));
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
}
</style>
