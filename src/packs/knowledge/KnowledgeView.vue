<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * ADR-0023 — 知识库 Pack 的扩展分区页面。
 * 只做「服务状态 + 托管启动 + 关键运行参数呈现」，检索能力仍由搜索面板的知识源使用。
 * 托管时服务地址由 feed 固定（只读），凭据类配置统一在「服务配置」页维护。
 */
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import FluentIcon from "../../components/icons/FluentIcon.vue";
import { toastSuccess } from "../../state/toast";

/** 后端托管服务健康收敛事件名（与 managed_dependency::commands::MANAGED_HEALTH_EVENT 一致）。 */
const MANAGED_HEALTH_EVENT = "managed-dependency:health-changed";
/** 本页对应的托管记录 id（= pack id）。 */
const KNOWLEDGE_PACK_ID = "awam.knowledge";

defineProps<{ embedded?: boolean }>();

const { t } = useI18n();
const router = useRouter();

type KnowledgeStatus = {
  online: boolean;
  baseUrl: string;
  detail?: string | null;
  vaultConfigured: boolean;
  indexConfigured: boolean;
  indexRoot?: string | null;
};

type ManagedRecord = {
  id: string;
  packId: string;
  mode: string;
  baseUrl: string;
  health: string;
};

const status = ref<KnowledgeStatus | null>(null);
const managed = ref(false);
const busy = ref(false);
const error = ref<string | null>(null);

const statusText = computed(() => {
  if (!status.value) return "";
  return status.value.online
    ? t("knowledgePage.statusOnline")
    : t("knowledgePage.statusOffline", { detail: status.value.detail ?? "" });
});

const indexText = computed(() => {
  if (!status.value?.indexConfigured) return t("knowledgePage.indexNotConfigured");
  return status.value.indexRoot ?? t("knowledgePage.indexConfigured");
});

const vaultText = computed(() =>
  status.value?.vaultConfigured
    ? t("knowledgePage.vaultConfigured")
    : t("knowledgePage.vaultNotConfigured"),
);

async function loadManaged() {
  try {
    const records = await invoke<ManagedRecord[]>("managed_dependency_list");
    const rec = records.find((r) => r.id === KNOWLEDGE_PACK_ID || r.packId === KNOWLEDGE_PACK_ID);
    managed.value = rec?.mode === "managed";
  } catch {
    managed.value = false;
  }
}

async function refreshStatus() {
  try {
    status.value = await invoke<KnowledgeStatus>("knowledge_service_status");
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  }
}

async function startOrAttach() {
  busy.value = true;
  error.value = null;
  try {
    const record = await invoke<ManagedRecord>("managed_dependency_ensure", {
      id: KNOWLEDGE_PACK_ID,
    });
    if (record?.health === "starting") {
      toastSuccess(t("knowledgePage.starting"));
    } else {
      toastSuccess(t("knowledgePage.started"));
    }
    await refreshStatus();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    busy.value = false;
  }
}

function openServiceConfig() {
  void router.push("/extensions/config");
}

let unlisten: UnlistenFn | null = null;

onMounted(() => {
  void listen<string>(MANAGED_HEALTH_EVENT, (event) => {
    if (event.payload === KNOWLEDGE_PACK_ID) {
      void refreshStatus();
    }
  }).then((fn) => {
    unlisten = fn;
  });
  void (async () => {
    await loadManaged();
    await refreshStatus();
  })();
});

onUnmounted(() => {
  unlisten?.();
  unlisten = null;
});
</script>

<template>
  <section class="fluent-page knowledge" :class="{ 'fluent-page--embedded': embedded }">
    <header v-if="!embedded" class="fluent-page-header">
      <h1 class="fluent-page-title">{{ t("knowledgePage.title") }}</h1>
      <p class="fluent-page-meta">{{ t("knowledgePage.meta") }}</p>
    </header>

    <div class="fluent-surface card">
      <div class="row-head">
        <h2 class="section-title">
          <span class="fluent-icon-wash"><FluentIcon name="library" size="md" /></span>
          <span class="section-title-text">{{ t("knowledgePage.service") }}</span>
        </h2>
        <div class="actions">
          <button
            class="fluent-control fluent-control--accent"
            type="button"
            :disabled="busy"
            @click="startOrAttach"
          >
            {{ busy ? t("knowledgePage.startingNow") : t("knowledgePage.start") }}
          </button>
          <button
            class="fluent-control action-icon"
            type="button"
            :disabled="busy"
            :title="t('common.refresh')"
            :aria-label="t('common.refresh')"
            @click="refreshStatus"
          >
            <FluentIcon name="refresh" size="sm" />
            <span class="action-icon-label">{{ t("common.refresh") }}</span>
          </button>
        </div>
      </div>

      <div
        v-if="status"
        class="fluent-banner"
        :class="status.online ? 'fluent-banner--success' : 'fluent-banner--warning'"
      >
        <span
          class="fluent-chip"
          :class="status.online ? 'fluent-chip--ok' : 'fluent-chip--down'"
          >{{ status.online ? t("common.yes") : t("common.no") }}</span
        >
        <p class="banner-text">{{ statusText }}</p>
      </div>
      <p v-else class="fluent-banner">{{ t("common.loading") }}</p>

      <div class="kv">
        <span class="kv-key">{{ t("knowledgePage.serviceUrl") }}</span>
        <span class="kv-value">{{ status?.baseUrl ?? "—" }}</span>
      </div>
      <p v-if="managed" class="fluent-page-meta">{{ t("knowledgePage.managedHint") }}</p>

      <div class="kv">
        <span class="kv-key">{{ t("knowledgePage.index") }}</span>
        <span class="kv-value">{{ indexText }}</span>
      </div>
      <div class="kv">
        <span class="kv-key">{{ t("knowledgePage.vault") }}</span>
        <span class="kv-value">{{ vaultText }}</span>
      </div>

      <p class="fluent-page-meta">{{ t("knowledgePage.configHint") }}</p>
      <div class="actions">
        <button class="fluent-control" type="button" @click="openServiceConfig">
          {{ t("knowledgePage.openConfig") }}
        </button>
      </div>

      <p v-if="error" class="error-text">{{ error }}</p>
    </div>
  </section>
</template>

<style scoped>
.fluent-page--embedded {
  padding: 0;
}

.knowledge {
  gap: var(--fluent-space-lg);
  min-width: 0;
}

.card {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  padding: var(--fluent-space-lg);
  min-width: 0;
}

.row-head {
  display: flex;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  flex-wrap: wrap;
  align-items: center;
}

.section-title {
  margin: 0;
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  font-size: var(--fluent-font-size-subtitle);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
  align-items: center;
}

.action-icon {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xs);
}

.banner-text {
  margin: 0;
  min-width: 0;
  word-break: break-word;
}

.kv {
  display: flex;
  gap: var(--fluent-space-md);
  align-items: baseline;
  min-width: 0;
}

.kv-key {
  flex: 0 0 auto;
  color: var(--fluent-color-text-secondary);
}

.kv-value {
  min-width: 0;
  word-break: break-all;
}

.error-text {
  margin: 0;
  color: var(--fluent-color-text-critical);
  word-break: break-word;
}
</style>