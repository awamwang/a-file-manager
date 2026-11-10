<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/** AIA-219 / A6 — Explicit Harness / SDD init (ADR 0008). Not part of Core create. */
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { toastSuccess } from "../state/toast";
import { aiHarnessInit } from "./client";

const { t } = useI18n();

const workspacePath = ref("");
const includeSdd = ref(false);
const reinforceContext = ref(true);
const error = ref<string | null>(null);
const busy = ref(false);
const lastNote = ref<string | null>(null);

const ERROR_KEYS: Record<string, string> = {
  AI_HARNESS_WORKSPACE_INVALID: "ai.harness.errorWorkspace",
};

function mapError(err: unknown): string {
  const raw = err instanceof Error ? err.message : String(err);
  for (const [code, key] of Object.entries(ERROR_KEYS)) {
    if (raw.includes(code)) return t(key);
  }
  return raw;
}

async function runInit() {
  error.value = null;
  lastNote.value = null;
  busy.value = true;
  try {
    const report = await aiHarnessInit({
      workspacePath: workspacePath.value.trim(),
      includeSdd: includeSdd.value,
      reinforceContext: reinforceContext.value,
    });
    lastNote.value = t("ai.harness.noticeDone", {
      written: report.written.length,
      skipped: report.skipped.length,
    });
    toastSuccess(lastNote.value);
  } catch (err) {
    error.value = mapError(err);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="harness-panel" data-ai-panel="harness">
    <h3 class="harness-title">{{ t("ai.harness.title") }}</h3>
    <p class="harness-hint">{{ t("ai.harness.hint") }}</p>

    <label class="field">
      <span>{{ t("ai.harness.fieldWorkspace") }}</span>
      <input
        v-model="workspacePath"
        type="text"
        class="fluent-input"
        :placeholder="t('ai.harness.fieldWorkspaceHint')"
      />
    </label>

    <label class="check">
      <input v-model="includeSdd" type="checkbox" />
      <span>{{ t("ai.harness.includeSdd") }}</span>
    </label>
    <p class="harness-hint subtle">{{ t("ai.harness.includeSddHint") }}</p>

    <label class="check">
      <input v-model="reinforceContext" type="checkbox" />
      <span>{{ t("ai.harness.reinforceContext") }}</span>
    </label>

    <div class="actions">
      <button
        type="button"
        class="fluent-btn fluent-btn--primary"
        :disabled="busy || !workspacePath.trim()"
        @click="runInit"
      >
        {{ t("ai.harness.init") }}
      </button>
    </div>

    <p v-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>
    <p v-else-if="lastNote" class="fluent-banner">{{ lastNote }}</p>
  </div>
</template>

<style scoped>
.harness-panel {
  margin-top: var(--fluent-space-md);
  padding-top: var(--fluent-space-md);
  border-top: 1px solid var(--fluent-stroke-divider, rgba(0, 0, 0, 0.08));
}
.harness-title {
  margin: 0 0 var(--fluent-space-xs);
  font-size: 1rem;
}
.harness-hint {
  margin: 0 0 var(--fluent-space-sm);
  opacity: 0.85;
  font-size: 0.9rem;
}
.harness-hint.subtle {
  margin-top: calc(var(--fluent-space-xs) * -1);
}
.field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin-bottom: var(--fluent-space-sm);
}
.check {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: var(--fluent-space-xs);
}
.actions {
  margin: var(--fluent-space-sm) 0;
}
</style>
