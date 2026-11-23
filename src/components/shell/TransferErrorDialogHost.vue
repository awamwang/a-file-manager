<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted, onUnmounted, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
  resolveTransferError,
  transferErrorName,
  transferErrorOpen,
} from "../../state/transferError";

const { t } = useI18n();

function onKeydown(event: KeyboardEvent) {
  if (!transferErrorOpen.value) return;
  if (event.key === "Escape") {
    resolveTransferError("abort");
    event.preventDefault();
    event.stopPropagation();
  }
}

// Keyboard default: focus the accent Retry button on open (mirrors the other
// dialog hosts' autofocus so Enter retries the failed item).
watch(transferErrorOpen, (open) => {
  if (!open) return;
  requestAnimationFrame(() =>
    document.getElementById("afm-transfer-error-retry")?.focus({ preventScroll: true }),
  );
});

onMounted(() => {
  window.addEventListener("keydown", onKeydown, true);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
});
</script>

<template>
  <div
    v-if="transferErrorOpen"
    class="transfer-error-backdrop"
    role="presentation"
    @click.self="resolveTransferError('abort')"
  >
    <div
      class="transfer-error-dialog fluent-surface"
      role="dialog"
      aria-modal="true"
      :aria-label="t('dialog.transferErrorTitle')"
    >
      <h2 class="transfer-error-title">{{ t("dialog.transferErrorTitle") }}</h2>
      <p class="transfer-error-message">
        {{ t("dialog.transferErrorMessage", { name: transferErrorName }) }}
      </p>
      <div class="transfer-error-actions">
        <button type="button" class="fluent-control" @click="resolveTransferError('abort')">
          {{ t("dialog.transferErrorAbort") }}
        </button>
        <button type="button" class="fluent-control" @click="resolveTransferError('skip')">
          {{ t("dialog.transferErrorSkip") }}
        </button>
        <button
          id="afm-transfer-error-retry"
          type="button"
          class="fluent-control fluent-control--accent"
          @click="resolveTransferError('retry')"
        >
          {{ t("dialog.transferErrorRetry") }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.transfer-error-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--fluent-space-lg);
  background: color-mix(in srgb, var(--fluent-color-text-primary) 28%, transparent);
}

.transfer-error-dialog {
  width: min(28rem, 100%);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-lg);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  box-shadow: var(--fluent-shadow-flyout, 0 12px 32px rgba(0, 0, 0, 0.22));
}

.transfer-error-title {
  margin: 0;
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.transfer-error-message {
  margin: 0 0 var(--fluent-space-xs);
  color: var(--fluent-color-text-secondary);
  line-height: 1.45;
  -webkit-user-select: text;
  user-select: text;
}

.transfer-error-hint {
  margin: 0 0 var(--fluent-space-xs);
  color: var(--fluent-color-text-primary);
  line-height: 1.45;
}

.transfer-error-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--fluent-space-xs);
  margin-top: var(--fluent-space-sm);
}
</style>
