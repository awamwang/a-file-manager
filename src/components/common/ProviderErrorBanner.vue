<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import {
  classifyProviderError,
  providerErrorI18nKey,
  shouldShowProviderErrorRaw,
} from "../../state/provider/errorMessage.ts";

const props = defineProps<{
  error: string | null | undefined;
}>();

const { t } = useI18n();

const kind = computed(() => classifyProviderError(props.error));
const summaryKey = computed(() => providerErrorI18nKey(kind.value));
const showRaw = computed(
  () => shouldShowProviderErrorRaw(kind.value) && Boolean(props.error?.trim()),
);
const rawText = computed(() => props.error?.trim() ?? "");

const detailsLabel = computed(() =>
  kind.value === "generic"
    ? t("providers.error.rawGeneric")
    : t("providers.error.rawDetails"),
);
</script>

<template>
  <div class="fluent-banner fluent-banner--error provider-error">
    <span class="provider-error__summary">{{ t(summaryKey) }}</span>
    <details v-if="showRaw" class="provider-error__raw">
      <summary>{{ detailsLabel }}</summary>
      <code class="fluent-text-selectable">{{ rawText }}</code>
    </details>
  </div>
</template>

<style scoped>
.provider-error {
  -webkit-user-select: text;
  user-select: text;
}

.provider-error__summary {
  display: block;
}

.provider-error__raw {
  margin-top: var(--fluent-space-sm);
  font-size: 0.92em;
}

.provider-error__raw summary {
  cursor: pointer;
  opacity: 0.85;
}

.provider-error__raw code {
  display: block;
  margin-top: var(--fluent-space-xs);
  padding: var(--fluent-space-xs) var(--fluent-space-sm);
  border-radius: var(--fluent-radius-sm);
  background: var(--fluent-color-fill-control);
  white-space: pre-wrap;
  word-break: break-word;
  font-family: "Cascadia Code", "Consolas", "SFMono-Regular", monospace;
  line-height: 1.5;
}
</style>
