<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
  cancelConfirmDialog,
  confirmDialogBusy,
  confirmDialogDontAskAgain,
  confirmDialogOpen,
  confirmDialogRequest,
  resolveConfirmDialog,
} from "../../state/confirmDialog";

const { t } = useI18n();

const title = computed(
  () => confirmDialogRequest.value?.title ?? t("dialog.confirmTitle"),
);
const message = computed(() => confirmDialogRequest.value?.message ?? "");
const confirmLabel = computed(
  () => confirmDialogRequest.value?.confirmLabel ?? t("common.confirm"),
);
const cancelLabel = computed(
  () => confirmDialogRequest.value?.cancelLabel ?? t("common.cancel"),
);
const showDontAskAgain = computed(
  () => Boolean(confirmDialogRequest.value?.showDontAskAgain),
);
const danger = computed(() => Boolean(confirmDialogRequest.value?.danger));

function onKeydown(event: KeyboardEvent) {
  if (!confirmDialogOpen.value) return;
  if (event.key === "Escape") {
    cancelConfirmDialog();
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

watch(confirmDialogOpen, (open) => {
  if (!open) return;
  // Focus confirm on open for keyboard users (Enter → confirm).
  requestAnimationFrame(() => {
    document
      .getElementById("afm-confirm-dialog-ok")
      ?.focus({ preventScroll: true });
  });
});
</script>

<template>
  <div
    v-if="confirmDialogOpen"
    id="afm-confirm-dialog"
    class="confirm-dialog-backdrop"
    role="presentation"
    @click.self="cancelConfirmDialog"
  >
    <div
      class="confirm-dialog fluent-surface"
      role="dialog"
      aria-modal="true"
      :aria-label="title"
    >
      <h2 class="confirm-dialog-title">{{ title }}</h2>
      <p class="confirm-dialog-message">{{ message }}</p>
      <label v-if="showDontAskAgain" class="confirm-dialog-dont-ask">
        <input
          v-model="confirmDialogDontAskAgain"
          type="checkbox"
          class="fluent-field-check"
        />
        {{ t("dialog.dontAskAgain") }}
      </label>
      <div class="confirm-dialog-actions">
        <button
          type="button"
          class="fluent-control"
          :disabled="confirmDialogBusy"
          @click="cancelConfirmDialog"
        >
          {{ cancelLabel }}
        </button>
        <button
          id="afm-confirm-dialog-ok"
          type="button"
          class="fluent-control"
          :class="danger ? 'fluent-control--danger' : 'fluent-control--accent'"
          :disabled="confirmDialogBusy"
          @click="resolveConfirmDialog(true)"
        >
          {{ confirmLabel }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.confirm-dialog-backdrop {
  position: fixed;
  inset: 0;
  z-index: 85;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--fluent-space-lg);
  background: color-mix(in srgb, var(--fluent-color-text-primary) 28%, transparent);
}

.confirm-dialog {
  width: min(26rem, 100%);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  padding: var(--fluent-space-lg);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  box-shadow: var(--fluent-shadow-flyout, 0 12px 32px rgba(0, 0, 0, 0.22));
}

.confirm-dialog-title {
  margin: 0;
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.confirm-dialog-message {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  line-height: 1.45;
  overflow-wrap: anywhere;
  -webkit-user-select: text;
  user-select: text;
}

.confirm-dialog-dont-ask {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  font-size: var(--fluent-font-size-body);
}

.confirm-dialog-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--fluent-space-sm);
}

.fluent-control--danger {
  border-color: transparent;
  background: var(--fluent-color-status-danger);
  color: var(--fluent-color-text-on-accent, #fff);
  font-weight: var(--fluent-font-weight-semibold);
}

.fluent-control--danger:hover:not(:disabled) {
  filter: brightness(1.08);
}

.fluent-control--danger:active:not(:disabled) {
  filter: brightness(0.96);
}
</style>
