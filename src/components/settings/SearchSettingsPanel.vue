<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import {
  loadFuzzy,
  loadScopes,
  persistFuzzy,
  persistScopes,
  SEARCH_PALETTE_SCOPES_STORAGE_KEY,
  toggleScope as applyScopeToggle,
} from "../../search/scope.ts";
import {
  SEARCH_SOURCE_ORDER,
  type SearchSourceId,
} from "../../search/types.ts";
import {
  hydrateSettings,
  rememberSettingsDocument,
  type SettingsDocument,
} from "../../state/settingsHydrate";
import { toastError, toastSuccess } from "../../state/toast";
import { AI_ADDON_ENABLED } from "../../config/flavor.ts";
import {
  initVoiceInput,
  selfTestError,
  selfTestPhase,
  selfTestResult,
  startSelfTest,
  stopSelfTest,
  voiceErrorMessageKey,
} from "../../state/voiceInput.ts";

/** AIA-276：进入搜索设置分区时再加载 Discovery 探测页。 */
const DiscoveryView = defineAsyncComponent(
  () => import("../../views/DiscoveryView.vue"),
);

defineProps<{
  /** 是否展示「优先 Everything」行（受平台能力门控）。 */
  everythingPreferredVisible: boolean;
  everythingPreferred: boolean;
  everythingPreferredSaving: boolean;
}>();

const emit = defineEmits<{
  "update:everythingPreferred": [value: boolean];
}>();

const { t } = useI18n();

const storage =
  typeof window === "undefined" ? undefined : window.localStorage;

const fuzzy = ref(loadFuzzy(storage));
/** null = 未持久化 / 按「全部」处理。 */
const storedScopes = ref<SearchSourceId[] | null>(loadScopes(storage));

const activeScopes = computed<SearchSourceId[]>(() => {
  const available = [...SEARCH_SOURCE_ORDER];
  const stored = storedScopes.value;
  if (!stored) return available;
  const effective = stored.filter((id) => available.includes(id));
  return effective.length ? effective : available;
});

function onFuzzyChange(event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  fuzzy.value = checked;
  persistFuzzy(checked, storage);
}

function onScopeToggle(id: SearchSourceId) {
  const next = applyScopeToggle(
    activeScopes.value,
    [...SEARCH_SOURCE_ORDER],
    id,
  );
  // 全选时回落 null，与面板「全部」语义一致。
  const allSelected =
    next.length === SEARCH_SOURCE_ORDER.length &&
    SEARCH_SOURCE_ORDER.every((sourceId) => next.includes(sourceId));
  storedScopes.value = allSelected ? null : next;
  if (allSelected) {
    try {
      storage?.removeItem(SEARCH_PALETTE_SCOPES_STORAGE_KEY);
    } catch {
      /* 持久化失败不阻断设置页。 */
    }
  } else {
    persistScopes(next, storage);
  }
}

function onEverythingPreferredChange(event: Event) {
  emit(
    "update:everythingPreferred",
    (event.target as HTMLInputElement).checked,
  );
}

const KNOWLEDGE_URL_SETTING_KEY = "power.awamKnowledgeServiceUrl";
const KNOWLEDGE_TOKEN_SETTING_KEY = "power.awamKnowledgeServiceToken";
const KNOWLEDGE_DEFAULT_URL = "http://127.0.0.1:18790";

type KnowledgeServiceStatus = {
  online: boolean;
  baseUrl: string;
  detail: string | null;
  vaultConfigured: boolean;
};

const knowledgeStatus = ref<KnowledgeServiceStatus | null>(null);
const knowledgeUrlDraft = ref(KNOWLEDGE_DEFAULT_URL);
const knowledgeTokenDraft = ref("");
const knowledgeSaving = ref(false);

const knowledgeStatusText = computed(() => {
  const status = knowledgeStatus.value;
  if (!status) return "";
  if (!status.online) return t("settings.search.knowledgeStatusOffline");
  return status.vaultConfigured
    ? t("settings.search.knowledgeStatusOnlineVault")
    : t("settings.search.knowledgeStatusOnlineNoVault");
});

function normalizeKnowledgeUrlInput(raw: string): string {
  return raw.trim().replace(/\/+$/, "");
}

async function refreshKnowledgeStatus() {
  try {
    knowledgeStatus.value = await invoke<KnowledgeServiceStatus>(
      "knowledge_service_status",
    );
  } catch {
    knowledgeStatus.value = null;
  }
}

async function loadKnowledgeDrafts() {
  try {
    const doc = await hydrateSettings();
    const url = doc.values[KNOWLEDGE_URL_SETTING_KEY];
    if (typeof url === "string" && url.trim()) {
      knowledgeUrlDraft.value = normalizeKnowledgeUrlInput(url);
    }
    const token = doc.values[KNOWLEDGE_TOKEN_SETTING_KEY];
    if (typeof token === "string") {
      knowledgeTokenDraft.value = token;
    }
  } catch {
    // 设置读取失败时保留默认 URL / 空令牌草稿。
  }
}

async function saveKnowledgeService() {
  knowledgeSaving.value = true;
  try {
    const doc = await invoke<SettingsDocument>("settings_set", {
      patch: {
        [KNOWLEDGE_URL_SETTING_KEY]:
          normalizeKnowledgeUrlInput(knowledgeUrlDraft.value) ||
          KNOWLEDGE_DEFAULT_URL,
        [KNOWLEDGE_TOKEN_SETTING_KEY]: knowledgeTokenDraft.value.trim(),
      },
    });
    rememberSettingsDocument(doc);
    await refreshKnowledgeStatus();
    toastSuccess(t("settings.search.knowledgeSaved"));
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    toastError(msg);
  } finally {
    knowledgeSaving.value = false;
  }
}

onMounted(() => {
  void loadKnowledgeDrafts();
  void refreshKnowledgeStatus();
  if (AI_ADDON_ENABLED) {
    void loadVoiceDrafts();
  }
});

// ---------------------------------------------------------------------------
// 语音输入（ADR 0018）：OpenAI 兼容转写配置 + 语音自检。
// ---------------------------------------------------------------------------

const VOICE_AUTOSTART_SETTING_KEY = "power.voiceSearchAutoStart";
const VOICE_BASE_URL_SETTING_KEY = "power.sttBaseUrl";
const VOICE_MODEL_SETTING_KEY = "power.sttModel";
const VOICE_API_KEY_SETTING_KEY = "power.sttApiKey";

const voiceAutoStart = ref(false);
const voiceBaseUrlDraft = ref("");
const voiceModelDraft = ref("");
const voiceApiKeyDraft = ref("");
const voiceSaving = ref(false);

async function loadVoiceDrafts() {
  try {
    const doc = await hydrateSettings();
    const autoStart = doc.values[VOICE_AUTOSTART_SETTING_KEY];
    if (typeof autoStart === "boolean") {
      voiceAutoStart.value = autoStart;
    }
    const baseUrl = doc.values[VOICE_BASE_URL_SETTING_KEY];
    if (typeof baseUrl === "string") {
      voiceBaseUrlDraft.value = baseUrl;
    }
    const model = doc.values[VOICE_MODEL_SETTING_KEY];
    if (typeof model === "string") {
      voiceModelDraft.value = model;
    }
    const apiKey = doc.values[VOICE_API_KEY_SETTING_KEY];
    if (typeof apiKey === "string") {
      voiceApiKeyDraft.value = apiKey;
    }
  } catch {
    // 读取失败保留空草稿；保存前自检会给出可操作的错误。
  }
}

/** 开启「语音搜索」即弹起试录：实测麦克风、权限与转写配置。 */
async function onVoiceAutoStartChange(event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  voiceAutoStart.value = checked;
  try {
    const doc = await invoke<SettingsDocument>("settings_set", {
      patch: { [VOICE_AUTOSTART_SETTING_KEY]: checked },
    });
    rememberSettingsDocument(doc);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    toastError(msg);
    return;
  }
  if (checked) {
    initVoiceInput();
    await startSelfTest();
  }
}

async function saveVoiceConfig() {
  voiceSaving.value = true;
  try {
    const doc = await invoke<SettingsDocument>("settings_set", {
      patch: {
        [VOICE_BASE_URL_SETTING_KEY]: voiceBaseUrlDraft.value.trim(),
        [VOICE_MODEL_SETTING_KEY]: voiceModelDraft.value.trim(),
        [VOICE_API_KEY_SETTING_KEY]: voiceApiKeyDraft.value.trim(),
      },
    });
    rememberSettingsDocument(doc);
    toastSuccess(t("voice.selftest.configSaved"));
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    toastError(msg);
  } finally {
    voiceSaving.value = false;
  }
}

/** 常驻独立测试按钮：空闲 → 起录；录音中 → 停止转写；识别中不动作。 */
function onVoiceTestButtonClick() {
  if (selfTestPhase.value === "idle") {
    initVoiceInput();
    void startSelfTest();
  } else if (selfTestPhase.value === "recording") {
    void stopSelfTest();
  }
}

const voiceSelfTestButtonLabel = computed(() => {
  if (selfTestPhase.value === "recording") return t("voice.selftest.stop");
  return t("voice.selftest.start");
});

/** 自检错误需给出可操作的引导（权限被拦/无设备/死流都指向系统麦克风设置）。 */
const voiceSelfTestNeedsPermissionHint = computed(
  () =>
    selfTestError.value?.code === "permission" ||
    selfTestError.value?.code === "no-device" ||
    selfTestError.value?.code === "no-audio",
);
</script>

<template>
  <div class="search-settings">
    <section class="fluent-surface search-settings-block">
      <h2 class="search-settings-title">{{ t("settings.search.paletteTitle") }}</h2>
      <p class="search-settings-meta">{{ t("settings.search.paletteMeta") }}</p>

      <label class="search-settings-toggle">
        <input
          type="checkbox"
          class="fluent-field-check"
          :checked="fuzzy"
          @change="onFuzzyChange"
        />
        <span>
          <strong>{{ t("searchPalette.fuzzy") }}</strong>
          <span class="search-settings-desc">{{
            t("settings.search.fuzzyHint")
          }}</span>
        </span>
      </label>

      <div
        class="search-settings-scopes"
        role="group"
        :aria-label="t('searchPalette.scopesLabel')"
      >
        <span class="search-settings-scopes-label">{{
          t("searchPalette.scopesLabel")
        }}</span>
        <p class="search-settings-desc">{{ t("settings.search.scopesHint") }}</p>
        <div class="search-settings-scope-chips">
          <label
            v-for="id in SEARCH_SOURCE_ORDER"
            :key="id"
            class="search-settings-scope-chip"
          >
            <input
              type="checkbox"
              class="fluent-field-check"
              :checked="activeScopes.includes(id)"
              @change="onScopeToggle(id)"
            />
            <span>{{ t(`searchPalette.scopes.${id}`) }}</span>
          </label>
        </div>
      </div>

      <label
        v-if="everythingPreferredVisible"
        class="search-settings-toggle"
      >
        <input
          type="checkbox"
          class="fluent-field-check"
          :checked="everythingPreferred"
          :disabled="everythingPreferredSaving"
          @change="onEverythingPreferredChange"
        />
        <span>
          <strong>{{
            t("settings.fields.core.everythingPreferred.title")
          }}</strong>
          <span class="search-settings-desc">{{
            t("settings.fields.core.everythingPreferred.description")
          }}</span>
        </span>
      </label>
    </section>

    <section class="fluent-surface search-settings-block">
      <h2 class="search-settings-title">
        {{ t("settings.search.knowledgeTitle") }}
      </h2>
      <p class="search-settings-meta">
        {{ t("settings.search.knowledgeMeta") }}
      </p>
      <p
        class="search-settings-meta"
        :data-knowledge-online="knowledgeStatus?.online ? 'true' : 'false'"
      >
        {{ knowledgeStatusText }}
      </p>

      <label class="search-settings-field">
        <span class="search-settings-field-label">{{
          t("settings.fields.power.awamKnowledgeServiceUrl.title")
        }}</span>
        <input
          v-model="knowledgeUrlDraft"
          class="fluent-control"
          type="text"
          spellcheck="false"
          :placeholder="KNOWLEDGE_DEFAULT_URL"
        />
        <span class="search-settings-desc">{{
          t("settings.fields.power.awamKnowledgeServiceUrl.description")
        }}</span>
      </label>

      <label class="search-settings-field">
        <span class="search-settings-field-label">{{
          t("settings.fields.power.awamKnowledgeServiceToken.title")
        }}</span>
        <input
          v-model="knowledgeTokenDraft"
          class="fluent-control"
          type="text"
          spellcheck="false"
          autocomplete="off"
        />
        <span class="search-settings-desc">{{
          t("settings.fields.power.awamKnowledgeServiceToken.description")
        }}</span>
      </label>

      <div>
        <button
          type="button"
          class="fluent-control fluent-control--accent"
          :disabled="knowledgeSaving"
          @click="saveKnowledgeService"
        >
          {{
            knowledgeSaving
              ? t("settings.search.knowledgeSaving")
              : t("settings.search.knowledgeSave")
          }}
        </button>
      </div>
    </section>

    <section
      v-if="AI_ADDON_ENABLED"
      class="fluent-surface search-settings-block"
      data-voice-settings
    >
      <h2 class="search-settings-title">
        {{ t("settings.search.voiceTitle") }}
      </h2>
      <p class="search-settings-meta">
        {{ t("settings.search.voiceMeta") }}
      </p>

      <label class="search-settings-toggle">
        <input
          type="checkbox"
          class="fluent-field-check"
          :checked="voiceAutoStart"
          @change="onVoiceAutoStartChange"
        />
        <span>
          <strong>{{
            t("settings.fields.power.voiceSearchAutoStart.title")
          }}</strong>
          <span class="search-settings-desc">{{
            t("settings.fields.power.voiceSearchAutoStart.description")
          }}</span>
        </span>
      </label>

      <label class="search-settings-field">
        <span class="search-settings-field-label">{{
          t("settings.fields.power.sttBaseUrl.title")
        }}</span>
        <input
          v-model="voiceBaseUrlDraft"
          class="fluent-control"
          type="text"
          spellcheck="false"
          placeholder="https://api.openai.com/v1"
        />
        <span class="search-settings-desc">{{
          t("settings.fields.power.sttBaseUrl.description")
        }}</span>
      </label>

      <label class="search-settings-field">
        <span class="search-settings-field-label">{{
          t("settings.fields.power.sttModel.title")
        }}</span>
        <input
          v-model="voiceModelDraft"
          class="fluent-control"
          type="text"
          spellcheck="false"
          placeholder="whisper-1"
        />
        <span class="search-settings-desc">{{
          t("settings.fields.power.sttModel.description")
        }}</span>
      </label>

      <label class="search-settings-field">
        <span class="search-settings-field-label">{{
          t("settings.fields.power.sttApiKey.title")
        }}</span>
        <input
          v-model="voiceApiKeyDraft"
          class="fluent-control"
          type="text"
          spellcheck="false"
          autocomplete="off"
        />
        <span class="search-settings-desc">{{
          t("settings.fields.power.sttApiKey.description")
        }}</span>
      </label>

      <div>
        <button
          type="button"
          class="fluent-control fluent-control--accent"
          :disabled="voiceSaving"
          @click="saveVoiceConfig"
        >
          {{ t("settings.search.knowledgeSave") }}
        </button>
      </div>

      <div
        class="search-settings-voice-selftest"
        :data-selftest-phase="selfTestPhase"
      >
        <div class="search-settings-voice-selftest-row">
          <button
            type="button"
            class="fluent-control"
            data-selftest-button
            @click="onVoiceTestButtonClick"
          >
            {{ voiceSelfTestButtonLabel }}
          </button>
          <span
            v-if="selfTestPhase !== 'idle'"
            class="search-settings-meta"
            data-selftest-status
          >
            {{
              selfTestPhase === "recording"
                ? t("voice.selftest.recording")
                : t("voice.selftest.transcribing")
            }}
          </span>
        </div>
        <p
          v-if="selfTestResult"
          class="search-settings-meta"
          data-selftest-result
        >
          {{ t("voice.selftest.result") }}: {{ selfTestResult.text }}
          · {{ (selfTestResult.elapsedMs / 1000).toFixed(1) }}s
        </p>
        <p
          v-if="selfTestError"
          class="search-settings-meta"
          data-selftest-error
        >
          {{ t(voiceErrorMessageKey(selfTestError.code)) }}
        </p>
        <p
          v-if="voiceSelfTestNeedsPermissionHint"
          class="search-settings-desc"
          data-selftest-permission-hint
        >
          {{ t("voice.selftest.permissionHint") }}
        </p>
      </div>
    </section>

    <section class="search-settings-probe">
      <h2 class="search-settings-title">{{ t("settings.search.probeTitle") }}</h2>
      <p class="search-settings-meta">{{ t("settings.search.probeMeta") }}</p>
      <DiscoveryView mode="test" />
    </section>
  </div>
</template>

<style scoped>
.search-settings {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-lg);
  min-width: 0;
}

.search-settings-block {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  padding: var(--fluent-space-md) var(--fluent-space-lg);
}

.search-settings-title {
  margin: 0;
  font-size: var(--fluent-font-size-body);
  font-weight: var(--fluent-font-weight-semibold);
}

.search-settings-meta,
.search-settings-desc {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  line-height: 1.45;
}

.search-settings-toggle {
  display: flex;
  align-items: flex-start;
  gap: var(--fluent-space-sm);
  cursor: pointer;
}

.search-settings-toggle span {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.search-settings-field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.search-settings-field-label {
  font-weight: var(--fluent-font-weight-semibold);
  font-size: var(--fluent-font-size-caption);
}

.search-settings-scopes {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
}

.search-settings-scopes-label {
  font-weight: var(--fluent-font-weight-semibold);
}

.search-settings-scope-chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
}

.search-settings-scope-chip {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  cursor: pointer;
}

.search-settings-voice-selftest {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
  padding-top: var(--fluent-space-sm);
  border-top: 1px solid var(--fluent-color-stroke);
}

.search-settings-voice-selftest-row {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
}

.search-settings-probe {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  min-width: 0;
}

.search-settings-probe :deep(.fluent-page) {
  padding-block: 0;
}

.search-settings-probe :deep(.fluent-page-header) {
  display: none;
}
</style>
