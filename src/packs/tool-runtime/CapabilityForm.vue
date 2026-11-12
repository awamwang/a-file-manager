<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { JsonSchema } from "./client";

const props = defineProps<{
  schema: JsonSchema | null;
  modelValue: Record<string, unknown>;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: Record<string, unknown>];
}>();

const { t } = useI18n();

const jsonText = ref("{}");
const jsonError = ref<string | null>(null);

const propertyEntries = computed(() => {
  const properties = props.schema?.properties;
  if (!properties) return [];
  return Object.entries(properties);
});

const useJsonFallback = computed(() => {
  if (!props.schema?.properties) return true;
  if (propertyEntries.value.length === 0) return false;
  return !propertyEntries.value.every(([, schema]) => isSimpleField(schema));
});

watch(
  () => [props.modelValue, useJsonFallback.value] as const,
  ([value, fallback]) => {
    if (!fallback) return;
    jsonText.value = stringify(value);
    jsonError.value = null;
  },
  { immediate: true, deep: true },
);

function isSimpleField(schema: JsonSchema | undefined): boolean {
  if (!schema) return false;
  if (Array.isArray(schema.enum) && schema.enum.length > 0) return true;
  const type = primaryType(schema);
  return type === "string" || type === "number" || type === "integer" || type === "boolean";
}

function primaryType(schema: JsonSchema): string | undefined {
  if (Array.isArray(schema.type)) return schema.type[0];
  return schema.type;
}

function fieldLabel(key: string, schema: JsonSchema): string {
  return schema.title || schema.description || key;
}

function isRequired(key: string): boolean {
  return (props.schema?.required ?? []).includes(key);
}

function fieldValue(key: string): unknown {
  return props.modelValue[key];
}

function patch(key: string, value: unknown): void {
  emit("update:modelValue", { ...props.modelValue, [key]: value });
}

function onTextInput(key: string, event: Event): void {
  patch(key, (event.target as HTMLInputElement).value);
}

function onNumberInput(key: string, event: Event, integer: boolean): void {
  const raw = (event.target as HTMLInputElement).value;
  if (raw.trim() === "") {
    const next = { ...props.modelValue };
    delete next[key];
    emit("update:modelValue", next);
    return;
  }
  const parsed = integer ? Number.parseInt(raw, 10) : Number.parseFloat(raw);
  patch(key, Number.isNaN(parsed) ? raw : parsed);
}

function onBooleanChange(key: string, event: Event): void {
  patch(key, (event.target as HTMLInputElement).checked);
}

function onEnumChange(key: string, schema: JsonSchema, event: Event): void {
  const raw = (event.target as HTMLSelectElement).value;
  const matched = (schema.enum ?? []).find((item) => String(item) === raw);
  patch(key, matched ?? raw);
}

function onJsonInput(event: Event): void {
  const text = (event.target as HTMLTextAreaElement).value;
  jsonText.value = text;
  try {
    const parsed: unknown = JSON.parse(text);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      jsonError.value = t("toolRuntime.jsonInvalid");
      return;
    }
    jsonError.value = null;
    emit("update:modelValue", parsed as Record<string, unknown>);
  } catch {
    jsonError.value = t("toolRuntime.jsonInvalid");
  }
}

function stringify(value: Record<string, unknown>): string {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return "{}";
  }
}

function enumOptions(schema: JsonSchema): unknown[] {
  return Array.isArray(schema.enum) ? schema.enum : [];
}

function fieldKind(schema: JsonSchema): "enum" | "boolean" | "number" | "integer" | "string" {
  if (enumOptions(schema).length > 0) return "enum";
  const type = primaryType(schema);
  if (type === "boolean" || type === "number" || type === "integer") return type;
  return "string";
}
</script>

<template>
  <div class="capability-form">
    <template v-if="useJsonFallback">
      <label class="field">
        <span class="field-label">{{ t("toolRuntime.jsonFallback") }}</span>
        <textarea
          class="fluent-control json-input"
          rows="10"
          spellcheck="false"
          :value="jsonText"
          @input="onJsonInput"
        />
      </label>
      <p v-if="jsonError" class="fluent-banner fluent-banner--error">{{ jsonError }}</p>
    </template>
    <p v-else-if="propertyEntries.length === 0" class="fluent-page-meta">
      {{ t("toolRuntime.emptySchema") }}
    </p>
    <template v-else>
      <label v-for="[key, schema] in propertyEntries" :key="key" class="field">
        <span class="field-label">
          {{ fieldLabel(key, schema) }}
          <span v-if="isRequired(key)" class="required">*</span>
        </span>
        <select
          v-if="fieldKind(schema) === 'enum'"
          class="fluent-control"
          :value="String(fieldValue(key) ?? '')"
          @change="onEnumChange(key, schema, $event)"
        >
          <option value="" disabled>{{ key }}</option>
          <option
            v-for="option in enumOptions(schema)"
            :key="String(option)"
            :value="String(option)"
          >
            {{ String(option) }}
          </option>
        </select>
        <span v-else-if="fieldKind(schema) === 'boolean'" class="check-row">
          <input
            class="fluent-field-check"
            type="checkbox"
            :checked="Boolean(fieldValue(key))"
            @change="onBooleanChange(key, $event)"
          />
        </span>
        <input
          v-else-if="fieldKind(schema) === 'integer' || fieldKind(schema) === 'number'"
          class="fluent-control"
          type="number"
          :step="fieldKind(schema) === 'integer' ? '1' : 'any'"
          :value="fieldValue(key) == null ? '' : String(fieldValue(key))"
          @input="onNumberInput(key, $event, fieldKind(schema) === 'integer')"
        />
        <input
          v-else
          class="fluent-control"
          type="text"
          spellcheck="false"
          :value="String(fieldValue(key) ?? '')"
          @input="onTextInput(key, $event)"
        />
      </label>
    </template>
  </div>
</template>

<style scoped>
.capability-form {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}

.field {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
}

.field-label {
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.required {
  color: var(--fluent-color-status-danger);
  margin-inline-start: var(--fluent-space-xs);
}

.check-row {
  display: inline-flex;
  align-items: center;
  min-height: var(--fluent-control-height-standard);
}

.json-input {
  min-height: 10rem;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  resize: vertical;
}
</style>
