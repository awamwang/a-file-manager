<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 单个 Feature Pack 的配置表单（发现机制共享 UI）。
 *
 * 三处复用：扩展 → 服务配置（每 pack 一张卡）、各功能页的「本服务配置」区块、
 * 提供程序卡片内的单项编辑弹层。字段来自 pack 的 `contributions.config[]` 声明，
 * 值为空时提示「未设置（使用服务默认）」，并按已知服务探测**当前生效值**，
 * 避免"页面空白但服务其实在按默认值工作"的认知落差。
 *
 * 版式约定：
 * - 一律「标签在上、控件在下」，控件独占整行宽度（密钥类需要足够长度）；
 *   不再按容器宽度切列，相邻字段高度只差一行说明文字，不会互相撑高。
 * - 密钥显隐用输入框内嵌的**文字**按钮（显示 / 隐藏），图标不单独承担语义。
 * - 保存/重置只在有未保存改动时才出现，避免常亮主按钮打断阅读。
 */
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import {
  configRowsFor,
  diffChangedKeys,
  isEmptyValue,
  isManagedMode,
  loadConfigValues,
  loadManagedModes,
  saveConfig,
  type ConfigRow,
} from "../../state/extensionConfig";
import { toastError, toastSuccess } from "../../state/toast";

const props = defineProps<{
  packId: string;
  /** 仅渲染这些声明 id（单项编辑用）；缺省渲染本 pack 全部配置项。 */
  onlyIds?: string[];
}>();

const emit = defineEmits<{ saved: [] }>();

const { t, te } = useI18n();

const rows = computed<ConfigRow[]>(() => {
  const all = configRowsFor(props.packId);
  if (!props.onlyIds?.length) return all;
  return all.filter((row) => props.onlyIds!.includes(row.id));
});

const values = ref<Record<string, unknown>>({});
const original = ref<Record<string, unknown>>({});
const managedModes = ref<Record<string, string>>({});
const revealed = ref<Record<string, boolean>>({});
/** settingKey → 服务当前实际生效值（探测得到；拿不到则缺省）。 */
const effective = ref<Record<string, string>>({});
const saving = ref(false);

/** 是否还有未保存的改动（决定保存/重置是否出现）。 */
const dirty = computed(
  () => diffChangedKeys(rows.value, values.value, original.value).length > 0,
);

function fieldLabel(row: ConfigRow): string {
  if (row.titleKey && te(row.titleKey)) return t(row.titleKey);
  return row.title || row.id;
}

function fieldDescription(row: ConfigRow): string {
  if (row.descriptionKey && te(row.descriptionKey)) return t(row.descriptionKey);
  return row.description || "";
}

function isReadonly(row: ConfigRow): boolean {
  return row.managedReadonly && isManagedMode(managedModes.value[row.packId ?? ""]);
}

function isUnset(row: ConfigRow): boolean {
  return isEmptyValue(row, values.value[row.settingKey]);
}

function inputType(row: ConfigRow): string {
  if (row.kind === "secret") return revealed.value[row.settingKey] ? "text" : "password";
  if (row.kind === "number") return "number";
  return "text";
}

/** 显隐按钮文字：当前隐藏 → 动作是「显示内容」，反之「隐藏内容」。 */
function revealText(row: ConfigRow): string {
  return revealed.value[row.settingKey]
    ? t("settings.extensions.hide")
    : t("settings.extensions.reveal");
}

function toggleReveal(row: ConfigRow): void {
  revealed.value[row.settingKey] = !revealed.value[row.settingKey];
}

function toggleBoolean(row: ConfigRow): void {
  if (isReadonly(row)) return;
  values.value[row.settingKey] = values.value[row.settingKey] !== true;
}

/** 已知服务的生效值探测：只对「空值 + 服务自己有默认」的字段做，避免重复展示已填内容。 */
async function probeEffective(all: ConfigRow[]) {
  const next: Record<string, string> = {};
  if (all.some((row) => row.settingKey === "power.awamKnowledgeDataRoot")) {
    try {
      const status = await invoke<{
        indexConfigured: boolean;
        indexRoot?: string | null;
      }>("knowledge_service_status");
      if (status.indexConfigured && status.indexRoot) {
        next["power.awamKnowledgeDataRoot"] = status.indexRoot;
      }
    } catch {
      /* 服务离线时不展示生效值 */
    }
  }
  effective.value = next;
}

async function load() {
  const current = rows.value;
  const loaded = await loadConfigValues(current);
  values.value = loaded;
  original.value = { ...loaded };
  managedModes.value = await loadManagedModes();
  await probeEffective(current);
}

function reset() {
  values.value = { ...original.value };
}

async function save() {
  if (!dirty.value || saving.value) return;
  saving.value = true;
  try {
    const restarted = await saveConfig(
      rows.value,
      values.value,
      original.value,
      managedModes.value,
    );
    original.value = { ...values.value };
    toastSuccess(
      restarted.length
        ? t("settings.extensions.savedRestarted", { packs: restarted.join(", ") })
        : t("settings.extensions.saved"),
    );
    emit("saved");
    await probeEffective(rows.value);
  } catch (err) {
    toastError(err instanceof Error ? err.message : String(err));
  } finally {
    saving.value = false;
  }
}

onMounted(() => {
  void load();
});

watch(
  () => props.packId,
  () => {
    void load();
  },
);
</script>

<template>
  <div v-if="rows.length" class="pack-config">
    <div class="pack-config__fields">
      <div
        v-for="row in rows"
        :key="row.settingKey"
        class="field"
        :class="{ 'field--readonly': isReadonly(row) }"
      >
        <label class="field__label" :for="`cfg-${row.settingKey}`">{{ fieldLabel(row) }}</label>

        <!-- 文本 / 密钥 / 数字：密钥的显隐是输入框内嵌的文字按钮 -->
        <div
          v-if="row.kind === 'string' || row.kind === 'secret' || row.kind === 'number'"
          class="field__control"
        >
          <input
            :id="`cfg-${row.settingKey}`"
            v-model="values[row.settingKey]"
            class="fluent-control field__input"
            :class="{ 'field__input--secret': row.kind === 'secret' }"
            :type="inputType(row)"
            :readonly="isReadonly(row)"
            spellcheck="false"
            autocomplete="off"
            @keydown.enter.prevent="save"
          />
          <button
            v-if="row.kind === 'secret'"
            class="field__reveal"
            type="button"
            :aria-pressed="revealed[row.settingKey] ? 'true' : 'false'"
            @click="toggleReveal(row)"
          >
            {{ revealText(row) }}
          </button>
        </div>

        <!-- 布尔开关 -->
        <button
          v-else-if="row.kind === 'boolean'"
          class="fluent-switch field__switch"
          :class="{ 'fluent-switch--on': values[row.settingKey] === true }"
          type="button"
          role="switch"
          :aria-checked="values[row.settingKey] === true"
          :disabled="isReadonly(row)"
          :aria-label="fieldLabel(row)"
          @click="toggleBoolean(row)"
        >
          <span class="fluent-switch-thumb" aria-hidden="true"></span>
        </button>

        <!-- 枚举下拉 -->
        <select
          v-else-if="row.kind === 'enum'"
          :id="`cfg-${row.settingKey}`"
          v-model="values[row.settingKey]"
          class="fluent-control field__input"
          :disabled="isReadonly(row)"
        >
          <option v-for="opt in row.enumValues ?? []" :key="opt" :value="opt">{{ opt }}</option>
        </select>

        <p v-if="isReadonly(row)" class="field__meta">
          {{ t("settings.extensions.managedReadonly") }}
        </p>
        <p v-else-if="isUnset(row)" class="field__meta">
          {{ t("settings.extensions.notSet") }}
          <template v-if="effective[row.settingKey]">
            · {{ t("settings.extensions.effective", { value: effective[row.settingKey] }) }}
          </template>
        </p>
        <p v-if="fieldDescription(row)" class="field__meta">{{ fieldDescription(row) }}</p>
      </div>
    </div>

    <!-- 只有存在未保存改动时才出现，避免页面上常驻一枚主按钮 -->
    <div v-if="dirty || saving" class="pack-config__actions">
      <span class="pack-config__hint">{{ t("settings.extensions.unsavedChanges") }}</span>
      <button
        class="pack-config__reset"
        type="button"
        :disabled="saving"
        @click="reset"
      >
        {{ t("settings.extensions.reset") }}
      </button>
      <button
        class="fluent-control fluent-control--accent pack-config__save"
        type="button"
        :disabled="saving"
        @click="save"
      >
        {{ saving ? t("settings.extensions.saving") : t("settings.extensions.save") }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.pack-config {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  min-width: 0;
}

.pack-config__fields {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  min-width: 0;
}

/* 标签在上、控件在下：一行一项，高度只由自身内容决定，相邻项不会互相撑高。 */
.field {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-2xs, 0.25rem);
  min-width: 0;
}

.field__label {
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-secondary);
  overflow-wrap: anywhere;
}

.field__control {
  position: relative;
  display: flex;
  align-items: center;
  min-width: 0;
}

.field__input {
  inline-size: 100%;
  min-width: 0;
  box-sizing: border-box;
}

/* 给内嵌的文字显隐按钮留位，长密钥不会被按钮盖住。 */
.field__input--secret {
  padding-inline-end: 4.5rem;
}

.field__reveal {
  position: absolute;
  inset-block: 3px;
  inset-inline-end: 3px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding-inline: var(--fluent-space-sm);
  min-inline-size: 4rem;
  border: 0;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: var(--fluent-color-text-secondary);
  font: inherit;
  font-size: var(--fluent-font-size-caption);
  cursor: pointer;
  transition: background-color 80ms ease, color 80ms ease;
}

.field__reveal:hover {
  background: var(--fluent-color-fill-control-hover);
  color: var(--fluent-color-text-primary);
}

.field__reveal:focus-visible {
  outline: 2px solid var(--fluent-color-stroke-focus);
  outline-offset: -2px;
}

/* 已切换为明文：按钮进入「可隐藏」的可用态，语义与视觉同时变化。 */
.field__reveal[aria-pressed="true"] {
  color: var(--fluent-color-fill-accent);
  background: var(--fluent-color-fill-control-hover);
}

.field__switch {
  align-self: flex-start;
}

.field__meta {
  margin: 0;
  font-size: var(--fluent-font-size-caption);
  line-height: 1.45;
  color: var(--fluent-color-text-secondary);
  overflow-wrap: anywhere;
}

input[readonly],
select:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.pack-config__actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--fluent-space-sm);
  min-width: 0;
}

.pack-config__hint {
  margin-inline-end: auto;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.pack-config__reset {
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--fluent-color-text-secondary);
  font: inherit;
  font-size: var(--fluent-font-size-caption);
  cursor: pointer;
}

.pack-config__reset:hover:not(:disabled) {
  color: var(--fluent-color-text-primary);
  text-decoration: underline;
}

.pack-config__reset:disabled {
  opacity: 0.6;
  cursor: default;
}

.pack-config__save {
  flex: 0 0 auto;
}
</style>