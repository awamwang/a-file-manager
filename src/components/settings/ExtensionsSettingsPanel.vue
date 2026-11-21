<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 设置 → 扩展能力：统一三个受 AFM 托管/连接的后端服务配置，与其它设置分区不混。
 *  - awam-file 网关地址（power.awamGatewayUrl）
 *  - awam-knowledge 服务地址（power.awamKnowledgeServiceUrl）
 *  - awam-knowledge 服务令牌（power.awamKnowledgeServiceToken）
 * managed 启用对应 pack 时地址只读/派生（与 ToolRuntimeView 一致）；config-mode 可编辑。
 * 令牌始终可编辑（既是 AFM 客户端 Bearer，也是注入托管服务端的共享密钥）。
 */
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import {
  hydrateSettings,
  rememberSettingsDocument,
  type SettingsDocument,
} from "../../state/settingsHydrate";
import { toastError, toastSuccess } from "../../state/toast";

const { t } = useI18n();

const GATEWAY_URL_KEY = "power.awamGatewayUrl";
const KNOWLEDGE_URL_KEY = "power.awamKnowledgeServiceUrl";
const KNOWLEDGE_TOKEN_KEY = "power.awamKnowledgeServiceToken";
const KNOWLEDGE_DATA_ROOT_KEY = "power.awamKnowledgeDataRoot";
const KNOWLEDGE_SOURCE_ID_KEY = "power.awamKnowledgeSourceId";
const DEFAULT_GATEWAY_URL = "http://127.0.0.1:18787";
const DEFAULT_KNOWLEDGE_URL = "http://127.0.0.1:18790";
// 托管记录 id = pack id（register_for_enabled_pack 约定）。
const FILE_PACK_ID = "awam.file-gateway";
const KNOWLEDGE_PACK_ID = "awam.knowledge";

type ManagedRecord = { packId: string; mode: string };
type KnowledgeStatus = {
  online: boolean;
  baseUrl: string;
  detail: string | null;
  vaultConfigured: boolean;
  indexConfigured: boolean;
  indexRoot: string | null;
};

const gatewayUrl = ref(DEFAULT_GATEWAY_URL);
const knowledgeUrl = ref(DEFAULT_KNOWLEDGE_URL);
const knowledgeToken = ref("");
const knowledgeDataRoot = ref("");
const knowledgeSourceId = ref("");
const saving = ref(false);

const managedModes = ref<Record<string, string>>({});
const knowledgeStatus = ref<KnowledgeStatus | null>(null);

const gatewayManaged = computed(() => managedModes.value[FILE_PACK_ID] === "managed");
const knowledgeManaged = computed(() => managedModes.value[KNOWLEDGE_PACK_ID] === "managed");

const knowledgeStatusText = computed(() => {
  const s = knowledgeStatus.value;
  if (!s) return "";
  if (!s.online) return t("settings.search.knowledgeStatusOffline");
  if (!s.indexConfigured) return t("settings.extensions.indexUnconfigured");
  return s.vaultConfigured
    ? t("settings.search.knowledgeStatusOnlineVault")
    : t("settings.search.knowledgeStatusOnlineNoVault");
});

const knowledgeIndexRoot = computed(() => knowledgeStatus.value?.indexRoot ?? "");

function normalizeUrl(raw: string, fallback: string): string {
  const trimmed = raw.trim().replace(/\/+$/, "");
  return trimmed || fallback;
}

async function load() {
  try {
    const doc = await hydrateSettings();
    const gw = doc.values[GATEWAY_URL_KEY];
    if (typeof gw === "string" && gw.trim()) gatewayUrl.value = gw.trim().replace(/\/+$/, "");
    const ku = doc.values[KNOWLEDGE_URL_KEY];
    if (typeof ku === "string" && ku.trim()) knowledgeUrl.value = ku.trim().replace(/\/+$/, "");
    const kt = doc.values[KNOWLEDGE_TOKEN_KEY];
    if (typeof kt === "string") knowledgeToken.value = kt;
    const dr = doc.values[KNOWLEDGE_DATA_ROOT_KEY];
    if (typeof dr === "string") knowledgeDataRoot.value = dr;
    const sid = doc.values[KNOWLEDGE_SOURCE_ID_KEY];
    if (typeof sid === "string") knowledgeSourceId.value = sid;
  } catch {
    /* 读取失败保留默认草稿 */
  }
  try {
    const records = await invoke<ManagedRecord[]>("managed_dependency_list");
    const m: Record<string, string> = {};
    for (const r of records) m[r.packId] = r.mode;
    managedModes.value = m;
  } catch {
    managedModes.value = {};
  }
  await refreshKnowledgeStatus();
}

async function refreshKnowledgeStatus() {
  try {
    knowledgeStatus.value = await invoke<KnowledgeStatus>("knowledge_service_status");
  } catch {
    knowledgeStatus.value = null;
  }
}

async function save() {
  saving.value = true;
  try {
    const patch: Record<string, string> = {
      [GATEWAY_URL_KEY]: normalizeUrl(gatewayUrl.value, DEFAULT_GATEWAY_URL),
      [KNOWLEDGE_URL_KEY]: normalizeUrl(knowledgeUrl.value, DEFAULT_KNOWLEDGE_URL),
      [KNOWLEDGE_TOKEN_KEY]: knowledgeToken.value.trim(),
      [KNOWLEDGE_DATA_ROOT_KEY]: knowledgeDataRoot.value.trim(),
      [KNOWLEDGE_SOURCE_ID_KEY]: knowledgeSourceId.value.trim(),
    };
    const doc = await invoke<SettingsDocument>("settings_set", { patch });
    rememberSettingsDocument(doc);
    await refreshKnowledgeStatus();
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

    <div class="ext-config__grid">
      <section class="fluent-surface section-card">
        <h3 class="section-card__title">{{ t("settings.extensions.gatewaySection") }}</h3>
        <div class="field">
          <label class="field-label">{{ t("settings.fields.power.awamGatewayUrl.title") }}</label>
          <p class="field-desc">{{ t("settings.fields.power.awamGatewayUrl.description") }}</p>
          <input
            v-model="gatewayUrl"
            :aria-label="t('settings.fields.power.awamGatewayUrl.title')"
            class="fluent-field"
            type="text"
            :readonly="gatewayManaged"
            spellcheck="false"
          />
          <p v-if="gatewayManaged" class="field-hint">{{ t("settings.extensions.managedReadonly") }}</p>
        </div>
      </section>

      <section class="fluent-surface section-card">
        <h3 class="section-card__title">{{ t("settings.extensions.knowledgeSection") }}</h3>
        <div class="field">
          <label class="field-label">{{ t("settings.fields.power.awamKnowledgeServiceUrl.title") }}</label>
          <p class="field-desc">{{ t("settings.fields.power.awamKnowledgeServiceUrl.description") }}</p>
          <input
            v-model="knowledgeUrl"
            :aria-label="t('settings.fields.power.awamKnowledgeServiceUrl.title')"
            class="fluent-field"
            type="text"
            :readonly="knowledgeManaged"
            spellcheck="false"
          />
          <p v-if="knowledgeManaged" class="field-hint">{{ t("settings.extensions.managedReadonly") }}</p>
          <p v-if="knowledgeStatusText" class="field-status">{{ knowledgeStatusText }}</p>
          <p v-if="knowledgeIndexRoot" class="field-hint">{{ knowledgeIndexRoot }}</p>
        </div>

        <div class="field">
          <label class="field-label">{{ t("settings.fields.power.awamKnowledgeServiceToken.title") }}</label>
          <p class="field-desc">{{ t("settings.fields.power.awamKnowledgeServiceToken.description") }}</p>
          <input
            v-model="knowledgeToken"
            :aria-label="t('settings.fields.power.awamKnowledgeServiceToken.title')"
            class="fluent-field"
            type="text"
            spellcheck="false"
          />
        </div>

        <div class="field">
          <label class="field-label">{{ t("settings.fields.power.awamKnowledgeDataRoot.title") }}</label>
          <p class="field-desc">{{ t("settings.fields.power.awamKnowledgeDataRoot.description") }}</p>
          <input
            v-model="knowledgeDataRoot"
            :aria-label="t('settings.fields.power.awamKnowledgeDataRoot.title')"
            class="fluent-field"
            type="text"
            spellcheck="false"
            :placeholder="t('settings.extensions.dataRootPlaceholder')"
          />
        </div>

        <div class="field">
          <label class="field-label">{{ t("settings.fields.power.awamKnowledgeSourceId.title") }}</label>
          <p class="field-desc">{{ t("settings.fields.power.awamKnowledgeSourceId.description") }}</p>
          <input
            v-model="knowledgeSourceId"
            :aria-label="t('settings.fields.power.awamKnowledgeSourceId.title')"
            class="fluent-field"
            type="text"
            spellcheck="false"
            :placeholder="t('settings.extensions.sourceIdPlaceholder')"
          />
        </div>
      </section>
    </div>

    <button class="fluent-control fluent-control--accent" type="button" :disabled="saving" @click="save">
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
.field-hint,
.field-status {
  margin: 0;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.field-status {
  color: var(--fluent-color-status-success, var(--fluent-color-text-secondary));
}

input.fluent-field {
  width: 100%;
  max-width: 40rem;
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
