<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { dismissToast, toasts } from "../../state/toast";

const { t } = useI18n();
</script>

<template>
  <div id="afm-toast-host" class="toast-host" aria-live="polite" aria-relevant="additions">
    <div
      v-for="item in toasts"
      :key="item.id"
      class="toast"
      :class="`toast--${item.kind}`"
      role="status"
    >
      <span class="toast-message">{{ item.message }}</span>
      <button
        type="button"
        class="toast-dismiss"
        :aria-label="t('common.dismiss')"
        @click="dismissToast(item.id)"
      >
        ×
      </button>
    </div>
  </div>
</template>

<style scoped>
.toast-host {
  position: fixed;
  right: var(--fluent-space-lg);
  bottom: calc(var(--fluent-statusbar-height, 2rem) + var(--fluent-space-md));
  z-index: 90;
  display: flex;
  flex-direction: column-reverse;
  align-items: flex-end;
  gap: var(--fluent-space-sm);
  max-width: min(24rem, calc(100vw - 2rem));
  pointer-events: none;
}

.toast {
  pointer-events: auto;
  display: flex;
  align-items: flex-start;
  gap: var(--fluent-space-sm);
  min-width: 12rem;
  max-width: 100%;
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-radius: var(--fluent-radius-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
  color: var(--fluent-color-text-primary);
  box-shadow: var(--fluent-shadow-flyout, 0 8px 24px rgba(0, 0, 0, 0.18));
  font-size: var(--fluent-font-size-body);
  line-height: 1.4;
}

.toast--success {
  border-color: var(--fluent-color-status-success);
  background: var(--fluent-color-status-success-bg, var(--fluent-color-fill-control));
}

.toast--error {
  border-color: var(--fluent-color-status-danger);
  background: var(--fluent-color-status-danger-bg, var(--fluent-color-fill-control));
}

.toast--info {
  border-color: var(--fluent-color-stroke-divider);
}

.toast-message {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
}

.toast-dismiss {
  flex-shrink: 0;
  border: 0;
  background: transparent;
  color: var(--fluent-color-text-secondary);
  cursor: pointer;
  font: inherit;
  line-height: 1;
  padding: 0 0.15rem;
}
</style>
