<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 单个 Feature Pack 的配置表单（发现机制共享 UI）。
 *
 * 三处复用：扩展 → 服务配置（每 pack 一张卡）、各功能页的「本服务配置」区块、
 * 提供程序卡片里的内联单项编辑。字段来自 pack 的 `contributions.config[]` 声明，
 * 值为空时提示「未设置（使用服务默认）」，并按已知服务探测**当前生效值**，
 * 避免"页面空白但服务其实在按默认值工作"的认知落差。
 */
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import {
  configRowsFor,
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
  /** 仅渲染这些声明 id（内联单项编辑用）；缺省渲染本 pack 全部配置项。 */
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

function toggleReveal(row: ConfigRow): void {
  revealed.value[row.settingKey] = !revealed.value[row.settingKey];
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

async function save() {
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
    <div v-for="row in rows" :key="row.settingKey" class="field">
      <label class="field-label" :for="`cfg-${row.settingKey}`">{{ fieldLabel(row) }}</label>
      <p v-if="fieldDescription(row)" class="field-desc">{{ fieldDescription(row) }}</p>

      <!-- 文本 / 密钥 / 数字 -->
      <div
        v-if="row.kind === 'string' || row.kind === 'secret' || row.kind === 'number'"
        class="input-row"
      >
        <input
          :id="`cfg-${row.settingKey}`"
          v-model="values[row.settingKey]"
          class="fluent-field"
          :type="inputType(row)"
          :readonly="isReadonly(row)"
          :aria-label="fieldLabel(row)"
          spellcheck="false"
        />
        <button
          v-if="row.kind === 'secret'"
          class="reveal-btn"
          type="button"
          :aria-label="fieldLabel(row)"
          @click="toggleReveal(row)"
        >
          {{ revealed[row.settingKey] ? "•" : "•••" }}
        </button>
      </div>

      <!-- 布尔开关 -->
      <label v-else-if="row.kind === 'boolean'" class="switch-row">
        <input
          v-model="values[row.settingKey]"
          type="checkbox"
          :disabled="isReadonly(row)"
          :aria-label="fieldLabel(row)"
        />
      </label>

      <!-- 枚举下拉 -->
      <select
        v-else-if="row.kind === 'enum'"
        v-model="values[row.settingKey]"
        class="fluent-field"
        :disabled="isReadonly(row)"
        :aria-label="fieldLabel(row)"
      >
        <option v-for="opt in row.enumValues ?? []" :key="opt" :value="opt">{{ opt }}</option>
      </select>

      <p v-if="isUnset(row) && !isReadonly(row)" class="field-hint">
        {{ t("settings.extensions.notSet") }}
        <template v-if="effective[row.settingKey]">
          · {{ t("settings.extensions.effective", { value: effective[row.settingKey] }) }}
        </template>
      </p>
      <p v-if="isReadonly(row)" class="field-hint">{{ t("settings.extensions.managedReadonly") }}</p>
    </div>

    <button
      class="fluent-control fluent-control--accent"
      type="button"
      :disabled="saving"
      @click="save"
    >
      {{ saving ? t("settings.extensions.saving") : t("settings.extensions.save") }}
    </button>
  </div>
</template>

<style scoped>
.pack-config {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  min-width: 0;
}

.field {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-2xs, 0.25rem);
}

.field-label {
  font-weight: var(--fluent-font-weight-semibold);
}

.field-desc,
.field-hint {
  margin: 0;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.input-row {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-2xs, 0.25rem);
}

input.fluent-field,
select.fluent-field {
  width: 100%;
  max-width: 40rem;
}

.reveal-btn {
  flex: 0 0 auto;
  min-width: 2.5rem;
}

.switch-row {
  display: flex;
  align-items: center;
}

input[readonly] {
  opacity: 0.7;
  cursor: not-allowed;
}

button {
  align-self: flex-start;
}
</style>