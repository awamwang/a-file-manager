<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 扩展 → 服务配置：发现驱动的后端服务配置页。
 *
 * 字段不再硬编码，而是来自各已启用 Feature Pack 的 `contributions.config[]` 声明
 * （经 `extension_host_list_contributions` 聚合、`configExtras` 暴露）。每个 pack 一张卡片，
 * 卡片内按其声明渲染控件：string/secret/number → 输入框（secret 用密码框 + 显示切换）、
 * boolean → 开关、enum → 下拉。值统一存进 settings 文档（pack 声明的 settingKey 经后端 schema
 * overlay 落盘），保存后对「值发生变化且处于托管模式」的 pack 自动 stop→ensure 重启，使新配置生效。
 *
 * `managedReadonly` 项在对应 pack 为 managed 时只读（地址由宿主派生），config 模式仍可编辑。
 */
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import {
  hydrateSettings,
  rememberSettingsDocument,
  type SettingsDocument,
} from "../../state/settingsHydrate";
import {
  configExtras,
  hydrateFeaturePackContributions,
  type FeaturePackConfigContribution,
} from "../../state/featurePack/contributions";
import { toastError, toastSuccess } from "../../state/toast";

const { t, te } = useI18n();

type ManagedRecord = { packId: string; mode: string };
/** getConfigExtras 已过滤 settingKey 非空，渲染时收窄为必有生效 key 的行。 */
type ConfigRow = FeaturePackConfigContribution & { settingKey: string };

const values = ref<Record<string, unknown>>({});
const original = ref<Record<string, unknown>>({});
const managedModes = ref<Record<string, string>>({});
const revealed = ref<Record<string, boolean>>({});
const saving = ref(false);

const groups = computed((): Array<{ packId: string; packName: string; items: ConfigRow[] }> => {
  const map = new Map<string, { packId: string; packName: string; items: ConfigRow[] }>();
  for (const item of configExtras.value) {
    if (!item.settingKey) continue;
    const row = item as ConfigRow;
    const packId = row.packId ?? "";
    if (!map.has(packId)) {
      map.set(packId, { packId, packName: row.packName ?? packId, items: [] });
    }
    map.get(packId)!.items.push(row);
  }
  return [...map.values()];
});

function fieldLabel(row: ConfigRow): string {
  if (row.titleKey && te(row.titleKey)) return t(row.titleKey);
  return row.title || row.id;
}

function fieldDescription(row: ConfigRow): string {
  if (row.descriptionKey && te(row.descriptionKey)) return t(row.descriptionKey);
  return row.description || "";
}

function isManaged(packId: string): boolean {
  return managedModes.value[packId] === "managed";
}

function isReadonly(row: ConfigRow): boolean {
  return row.managedReadonly && isManaged(row.packId ?? "");
}

function inputType(row: ConfigRow): string {
  if (row.kind === "secret") return revealed.value[row.settingKey] ? "text" : "password";
  if (row.kind === "number") return "number";
  return "text";
}

function coerceForInput(row: ConfigRow, raw: unknown): unknown {
  switch (row.kind) {
    case "boolean":
      return raw === true;
    case "number":
      if (typeof raw === "number") return raw;
      if (raw != null && String(raw).trim() !== "") {
        const n = Number(raw);
        return Number.isFinite(n) ? n : 0;
      }
      return 0;
    default:
      return typeof raw === "string" ? raw : raw == null ? "" : String(raw);
  }
}

function normalizeForSave(row: ConfigRow, val: unknown): unknown {
  switch (row.kind) {
    case "boolean":
      return val === true;
    case "number": {
      const n = Number(val);
      return Number.isFinite(n) ? n : 0;
    }
    default: {
      let text = val == null ? "" : String(val).trim();
      // 连接地址类（settingKey 以 Url 结尾）去掉尾斜杠，与迁移前一致。
      if (/url$/i.test(row.settingKey)) text = text.replace(/\/+$/, "");
      return text;
    }
  }
}

function toggleReveal(row: ConfigRow): void {
  revealed.value[row.settingKey] = !revealed.value[row.settingKey];
}

async function load() {
  try {
    // 服务配置页直接进入时，确保已拉到最新 Pack 贡献（含 config 声明）。
    await hydrateFeaturePackContributions();
  } catch {
    /* 拉取失败保留上次贡献缓存 */
  }
  let doc: SettingsDocument | null = null;
  try {
    doc = await hydrateSettings();
  } catch {
    doc = null;
  }
  const v: Record<string, unknown> = {};
  for (const item of configExtras.value) {
    if (!item.settingKey) continue;
    const raw = doc?.values[item.settingKey];
    v[item.settingKey] = coerceForInput(item as ConfigRow, raw);
  }
  values.value = v;
  original.value = { ...v };
  try {
    const records = await invoke<ManagedRecord[]>("managed_dependency_list");
    const m: Record<string, string> = {};
    for (const r of records) m[r.packId] = r.mode;
    managedModes.value = m;
  } catch {
    managedModes.value = {};
  }
}

/** 重启受影响的托管服务：stop 仅作用于 AFM 自启 pid；附着/未拉起时 stop 会报错，忽略后 ensure 重新附着或拉起。 */
async function restartManagedPack(packId: string): Promise<void> {
  try {
    await invoke("managed_dependency_stop", { id: packId });
  } catch {
    /* 非自启实例，无需停 */
  }
  try {
    await invoke("managed_dependency_ensure", { id: packId });
  } catch (err) {
    toastError(`${packId}: ${err instanceof Error ? err.message : String(err)}`);
  }
}

async function save() {
  saving.value = true;
  try {
    const patch: Record<string, unknown> = {};
    const affected = new Set<string>();
    for (const item of configExtras.value) {
      if (!item.settingKey) continue;
      const row = item as ConfigRow;
      const next = normalizeForSave(row, values.value[row.settingKey]);
      patch[row.settingKey] = next;
      const prev = original.value[row.settingKey];
      const changed = String(next ?? "") !== String(prev ?? "");
      if (changed && isManaged(row.packId ?? "")) affected.add(row.packId ?? "");
    }
    const doc = await invoke<SettingsDocument>("settings_set", { patch });
    rememberSettingsDocument(doc);
    for (const packId of affected) {
      await restartManagedPack(packId);
    }
    original.value = { ...values.value };
    toastSuccess(t("settings.extensions.saved"));
  } catch (err) {
    toastError(err instanceof Error ? err.message : String(err));
  } finally {
    saving.value = false;
  }
}

onMounted(() => {
  void load();
});
</script>

<template>
  <section class="ext-config">
    <header class="ext-config__header">
      <h2 class="section-title">{{ t("settings.extensions.title") }}</h2>
      <p class="fluent-page-meta">{{ t("settings.extensions.meta") }}</p>
    </header>

    <div v-if="groups.length" class="ext-config__grid">
      <section
        v-for="group in groups"
        :key="group.packId"
        class="fluent-surface section-card"
      >
        <h3 class="section-card__title">{{ group.packName }}</h3>

        <div v-for="row in group.items" :key="row.id" class="field">
          <label class="field-label">{{ fieldLabel(row) }}</label>
          <p v-if="fieldDescription(row)" class="field-desc">{{ fieldDescription(row) }}</p>

          <!-- 文本 / 密钥 / 数字 -->
          <div
            v-if="row.kind === 'string' || row.kind === 'secret' || row.kind === 'number'"
            class="input-row"
          >
            <input
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

          <p v-if="isReadonly(row)" class="field-hint">{{ t("settings.extensions.managedReadonly") }}</p>
        </div>
      </section>
    </div>

    <button
      v-if="groups.length"
      class="fluent-control fluent-control--accent"
      type="button"
      :disabled="saving"
      @click="save"
    >
      {{ t("settings.extensions.save") }}
    </button>
  </section>
</template>

<style scoped>
.ext-config {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}

.ext-config__header {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-2xs, 0.25rem);
}

.section-title {
  margin: 0;
  font-size: var(--fluent-font-size-subtitle);
}

.ext-config__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(100%, 22rem), 1fr));
  gap: var(--fluent-space-md);
}

.section-card {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  padding: var(--fluent-space-lg);
  min-width: 0;
}

.section-card__title {
  margin: 0;
  font-size: var(--fluent-font-size-body);
  font-weight: var(--fluent-font-weight-semibold);
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

@media (max-width: 720px) {
  .ext-config__grid {
    grid-template-columns: 1fr;
  }
}
</style>
