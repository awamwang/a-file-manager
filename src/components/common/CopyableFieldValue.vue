<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/** Click-to-copy for long machine-generated field values (e.g. magnet links). */
import { onBeforeUnmount, ref } from "vue";
import { useI18n } from "vue-i18n";
import FluentIcon from "../icons/FluentIcon.vue";
import { toastError, toastSuccess } from "../../state/toast";

const props = withDefaults(
  defineProps<{
    value: string;
    label?: string;
  }>(),
  { label: "" },
);

const { t } = useI18n();
const copied = ref(false);
let resetTimer: ReturnType<typeof setTimeout> | null = null;

/**
 * WebView2 rejects `navigator.clipboard` when the document is not focused or
 * the permission policy disallows it. Fall back to the legacy selection copy so
 * a click never silently does nothing.
 */
function copyViaSelection(text: string): boolean {
  const area = document.createElement("textarea");
  area.value = text;
  area.setAttribute("readonly", "");
  area.style.position = "fixed";
  area.style.top = "-1000px";
  area.style.opacity = "0";
  document.body.appendChild(area);
  let ok = false;
  try {
    area.select();
    ok = document.execCommand("copy");
  } catch {
    ok = false;
  } finally {
    document.body.removeChild(area);
  }
  return ok;
}

async function copyValue() {
  if (!props.value) return;
  let ok = false;
  try {
    await navigator.clipboard.writeText(props.value);
    ok = true;
  } catch {
    ok = copyViaSelection(props.value);
  }
  if (!ok) {
    toastError(t("virtualInfo.copyFailed"));
    return;
  }
  copied.value = true;
  toastSuccess(t("virtualInfo.copied"));
  if (resetTimer !== null) clearTimeout(resetTimer);
  resetTimer = setTimeout(() => {
    copied.value = false;
    resetTimer = null;
  }, 1400);
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
