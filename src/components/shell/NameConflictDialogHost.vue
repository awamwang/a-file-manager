<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import {
  nameConflictName,
  nameConflictOpen,
  nameConflictRememberReplace,
  nameConflictRememberSkip,
  resolveNameConflict,
  setNameConflictRemember,
} from "../../state/nameConflict";

const { t } = useI18n();

function onKeydown(event: KeyboardEvent) {
  if (!nameConflictOpen.value) return;
  if (event.key === "Escape") {
    resolveNameConflict("cancel");
    event.preventDefault();
    event.stopPropagation();
  }
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown, true);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
});
</script>

<template>
  <div
    v-if="nameConflictOpen"
    class="conflict-backdrop"
    role="presentation"
    @click.self="resolveNameConflict('cancel')"
  >
    <div
      class="conflict-dialog fluent-surface"
      role="dialog"
      aria-modal="true"
      :aria-label="t('dialog.conflictTitle')"
    >
      <h2 class="conflict-title">{{ t("dialog.conflictTitle") }}</h2>
      <p class="conflict-message">{{ t("dialog.conflictMessage", { name: nameConflictName }) }}</p>
      <label class="conflict-remember">
        <input
          :checked="nameConflictRememberReplace"
          type="checkbox"
          class="fluent-field-check"
          @change="setNameConflictRemember('replace', ($event.target as HTMLInputElement).checked)"
        />
        {{ t("dialog.conflictDefaultReplace") }}
      </label>
      <label class="conflict-remember">
        <input
          :checked="nameConflictRememberSkip"
          type="checkbox"
          class="fluent-field-check"
          @change="setNameConflictRemember('skip', ($event.target as HTMLInputElement).checked)"
        />
        {{ t("dialog.conflictDefaultSkip") }}
      </label>
      <div class="conflict-actions">
        <button type="button" class="fluent-control" @click="resolveNameConflict('cancel')">
          {{ t("common.cancel") }}
        </button>
        <button type="button" class="fluent-control" @click="resolveNameConflict('keep')">
          {{ t("dialog.conflictKeep") }}
        </button>
        <button type="button" class="fluent-control" @click="resolveNameConflict('skip')">
          {{ t("dialog.conflictSkip") }}
        </button>
        <button
          type="button"
          class="fluent-control fluent-control--accent"
          @click="resolveNameConflict('replace')"
        >
          {{ t("dialog.conflictReplace") }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.conflict-backdrop {
  position: fixed;
  inset: 0;
  z-index: 86;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--fluent-space-lg);
  background: color-mix(in srgb, var(--fluent-color-text-primary) 28%, transparent);
}

.conflict-dialog {
  width: min(28rem, 100%);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-lg);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  box-shadow: var(--fluent-shadow-flyout, 0 12px 32px rgba(0, 0, 0, 0.22));
}

.conflict-title {
  margin: 0;
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.conflict-message {
  margin: 0 0 var(--fluent-space-xs);
  color: var(--fluent-color-text-secondary);
  line-height: 1.45;
  -webkit-user-select: text;
  user-select: text;
}

.conflict-remember {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-xs);
}

.conflict-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--fluent-space-xs);
  margin-top: var(--fluent-space-sm);
}
</style>
