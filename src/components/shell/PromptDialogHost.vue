<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
  cancelPromptDialog,
  promptDialogBusy,
  promptDialogOpen,
  promptDialogRequest,
  promptDialogValue,
  resolvePromptDialog,
} from "../../state/promptDialog";

const { t } = useI18n();

const title = computed(
  () => promptDialogRequest.value?.title ?? t("dialog.promptTitle"),
);
const message = computed(() => promptDialogRequest.value?.message ?? "");
const label = computed(
  () => promptDialogRequest.value?.label ?? t("dialog.promptLabel"),
);
const placeholder = computed(
  () => promptDialogRequest.value?.placeholder ?? "",
);
const confirmLabel = computed(
  () => promptDialogRequest.value?.confirmLabel ?? t("common.create"),
);
const cancelLabel = computed(
  () => promptDialogRequest.value?.cancelLabel ?? t("common.cancel"),
);

function submit() {
  if (promptDialogBusy.value) return;
  resolvePromptDialog(true);
}

function onKeydown(event: KeyboardEvent) {
  if (!promptDialogOpen.value) return;
  if (event.key === "Escape") {
    cancelPromptDialog();
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

watch(promptDialogOpen, async (open) => {
  if (!open) return;
  await nextTick();
  const input = document.getElementById(
    "afm-prompt-dialog-input",
  ) as HTMLInputElement | null;
  input?.focus({ preventScroll: true });
  input?.select();
});
</script>

<template>
  <div
    v-if="promptDialogOpen"
    id="afm-prompt-dialog"
    class="prompt-dialog-backdrop"
    role="presentation"
    @click.self="cancelPromptDialog"
  >
    <div
      class="prompt-dialog fluent-surface"
      role="dialog"
      aria-modal="true"
      :aria-label="title"
    >
      <h2 class="prompt-dialog-title">{{ title }}</h2>
      <p v-if="message" class="prompt-dialog-message">{{ message }}</p>
      <label class="prompt-dialog-field">
        <span class="prompt-dialog-label">{{ label }}</span>
        <input
          id="afm-prompt-dialog-input"
          v-model="promptDialogValue"
          type="text"
          class="fluent-control prompt-dialog-input"
          :placeholder="placeholder"
          :disabled="promptDialogBusy"
          autocomplete="off"
          spellcheck="false"
          @keydown.enter.prevent="submit"
        />
      </label>
      <div class="prompt-dialog-actions">
        <button
          type="button"
          class="fluent-control"
          :disabled="promptDialogBusy"
          @click="cancelPromptDialog"
        >
          {{ cancelLabel }}
        </button>
        <button
          id="afm-prompt-dialog-ok"
          type="button"
          class="fluent-control fluent-control--accent"
          :disabled="promptDialogBusy"
          @click="submit"
        >
          {{ confirmLabel }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.prompt-dialog-backdrop {
  position: fixed;
  inset: 0;
  z-index: 86;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--fluent-space-lg);
  background: color-mix(in srgb, var(--fluent-color-text-primary) 28%, transparent);
}

.prompt-dialog {
  width: min(26rem, 100%);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  padding: var(--fluent-space-lg);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  box-shadow: var(--fluent-shadow-flyout, 0 12px 32px rgba(0, 0, 0, 0.22));
}

.prompt-dialog-title {
  margin: 0;
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.prompt-dialog-message {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  line-height: 1.45;
  overflow-wrap: anywhere;
}

.prompt-dialog-field {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
}

.prompt-dialog-label {
  font-size: var(--fluent-font-size-body);
  color: var(--fluent-color-text-secondary);
}

.prompt-dialog-input {
  width: 100%;
}

.prompt-dialog-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--fluent-space-sm);
}
</style>
