<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import FluentIcon from "../components/icons/FluentIcon.vue";
import { entryKindIcon } from "../components/icons/glyphs";
import { preloadProviderCache } from "../state/providerCache";
import { navigateTo } from "../state/navigatorBrowse";
import { PROVIDER_ROOT } from "../state/provider";
import {
  hydrateSettings,
  rememberSettingsDocument,
  type SettingsDocument,
} from "../state/settingsHydrate";

const { t } = useI18n();
const router = useRouter();

const GATEWAY_URL_SETTING_KEY = "power.awamGatewayUrl";
const DEFAULT_GATEWAY_URL = "http://127.0.0.1:18787";

type BackendInfo = {
  id: string;
  name: string;
  description: string;
  healthy: boolean;
  capabilities: string[];
  detail?: string | null;
  interfaceOnly?: boolean;
  source?: string | null;
};

type GatewayStatus = {
  online: boolean;
  baseUrl: string;
  detail?: string | null;
  backendCount?: number | null;
};

type ProviderEntry = {
  name: string;
  uri: string;
  isDir: boolean;
  isFile: boolean;
  size: number;
  localPath?: string | null;
  extra?: Record<string, unknown>;
};

type DirListing = {
  uri: string;
  backend: string;
  entries: ProviderEntry[];
  error?: string | null;
};

type ResolveResult = {
  uri: string;
  localPath?: string | null;
  action?: string | null;
  actionArgs?: Record<string, unknown>;
  error?: string | null;
};

const backends = ref<BackendInfo[]>([]);
const gateway = ref<GatewayStatus | null>(null);
const gatewayUrlDraft = ref(DEFAULT_GATEWAY_URL);
const gatewayUrlSaving = ref(false);
const refreshing = ref(false);
const uri = ref("awam://root");
const listing = ref<DirListing | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

function normalizeGatewayUrlInput(raw: string): string {
  return raw.trim().replace(/\/+$/, "");
}

async function loadGatewayUrlFromSettings() {
  try {
    const doc = await hydrateSettings();
    const raw = doc.values[GATEWAY_URL_SETTING_KEY];
    if (typeof raw === "string" && raw.trim()) {
      gatewayUrlDraft.value = normalizeGatewayUrlInput(raw) || DEFAULT_GATEWAY_URL;
      return;
    }
  } catch {
    // 设置读取失败时仍可用探测到的 baseUrl / 默认值。
  }
  if (gateway.value?.baseUrl) {
    gatewayUrlDraft.value = gateway.value.baseUrl;
  }
}

async function loadBackends() {
  // AIA-226: 优先 awam-file 网关发现；离线时回退本机内置提供方。
  const [status, list] = await Promise.all([
    invoke<GatewayStatus>("provider_gateway_status"),
    invoke<BackendInfo[]>("provider_backends"),
  ]);
  gateway.value = status;
  backends.value = list;
  if (!gatewayUrlDraft.value.trim() && status.baseUrl) {
    gatewayUrlDraft.value = status.baseUrl;
  }
}

/** 保存网关地址到设置并重新发现后端。 */
async function saveGatewayUrl() {
  const next = normalizeGatewayUrlInput(gatewayUrlDraft.value) || DEFAULT_GATEWAY_URL;
  gatewayUrlDraft.value = next;
  gatewayUrlSaving.value = true;
  error.value = null;
  try {
    const doc = await invoke<SettingsDocument>("settings_set", {
      patch: { [GATEWAY_URL_SETTING_KEY]: next },
    });
    rememberSettingsDocument(doc);
    await loadBackends();
    await listUri(PROVIDER_ROOT);
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    gatewayUrlSaving.value = false;
  }
}

async function listUri(target?: string) {
  loading.value = true;
  error.value = null;
  listing.value = null;
  try {
    if (target) uri.value = target;
    listing.value = await invoke<DirListing>("provider_list", { uri: uri.value });
    if (listing.value.error) {
      error.value = listing.value.error;
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    loading.value = false;
  }
}

function entrySecondary(entry: ProviderEntry): string {
  const url = entry.extra?.url;
  if (typeof url === "string" && url) return url;
  return entry.uri;
}

async function openEntry(entry: ProviderEntry) {
  if (entry.isDir) {
    void openInExplorer(entry.uri);
    return;
  }
  try {
    const resolved = await invoke<ResolveResult>("provider_resolve", { uri: entry.uri });
    if (resolved.error) {
      error.value = resolved.error;
      return;
    }
    const local = resolved.localPath?.trim();
    if (local) {
      const opened = await invoke<{ success: boolean; error: string | null }>(
        "open_path_default",
        { path: local },
      );
      if (!opened.success) {
        error.value = opened.error ?? t("providers.openUnsupported");
      }
      return;
    }
    if (resolved.action === "open_url") {
      const url = resolved.actionArgs?.url;
      if (typeof url === "string" && url) {
        await openUrl(url);
        return;
      }
    }
    error.value = t("providers.openUnsupported");
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  }
}

/** 在资源管理器中打开 Provider 路径。 */
async function openInExplorer(target: string) {
  error.value = null;
  try {
    await router.push("/navigator");
    await navigateTo(target);
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  }
}

/** 刷新网关状态、后端列表与当前 URI 列出结果。 */
async function refreshStatus() {
  if (refreshing.value || gatewayUrlSaving.value) return;
  refreshing.value = true;
  error.value = null;
  try {
    await loadBackends();
    await listUri(uri.value.trim() || PROVIDER_ROOT);
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    refreshing.value = false;
  }
}

onMounted(() => {
  // AIA-181: if idle preload has not finished (or was deferred), warm on first visit.
  void preloadProviderCache();
  void loadGatewayUrlFromSettings()
    .then(() => loadBackends())
    .then(() => listUri(PROVIDER_ROOT))
    .catch((err) => {
      error.value = err instanceof Error ? err.message : String(err);
    });
});
</script>

<template>
  <section class="fluent-page">
    <header class="fluent-page-header providers-header">
      <div class="providers-header-text">
        <h1 class="fluent-page-title">{{ t("providers.title") }}</h1>
        <p class="fluent-page-meta">{{ t("providers.meta") }}</p>
      </div>
      <button
        class="fluent-control"
        type="button"
        :disabled="refreshing || gatewayUrlSaving"
        :title="t('providers.refresh')"
        :aria-label="t('providers.refresh')"
        @click="refreshStatus"
      >
        <FluentIcon name="refresh" size="sm" />
        {{ refreshing ? t("providers.refreshing") : t("providers.refresh") }}
      </button>
    </header>

    <p
      v-if="gateway && gateway.online"
      class="fluent-banner fluent-banner--info"
    >
      {{
        t("providers.gatewayOnline", {
          count: gateway.backendCount ?? backends.length,
          url: gateway.baseUrl,
        })
      }}
    </p>
    <p
      v-else-if="gateway && !gateway.online"
      class="fluent-banner fluent-banner--info"
    >
      {{ t("providers.gatewayOffline") }}
    </p>

    <form class="uri-row gateway-row" @submit.prevent="saveGatewayUrl">
      <label class="gateway-label" for="providers-gateway-url">{{
        t("providers.gatewayUrlLabel")
      }}</label>
      <input
        id="providers-gateway-url"
        v-model="gatewayUrlDraft"
        class="fluent-control"
        type="url"
        spellcheck="false"
        autocomplete="off"
        :placeholder="t('providers.gatewayUrlPlaceholder')"
      />
      <button
        class="fluent-control fluent-control--accent"
        type="submit"
        :disabled="gatewayUrlSaving"
      >
        {{ gatewayUrlSaving ? t("providers.gatewayUrlSaving") : t("providers.gatewayUrlSave") }}
      </button>
    </form>

    <p v-if="!backends.length" class="fluent-banner fluent-banner--info">
      {{ t("providers.emptyBackends") }}
    </p>
    <div v-else class="backends">
      <button
        v-for="backend in backends"
        :key="backend.id"
        type="button"
        class="backend"
        :class="{
          'backend--ok': !backend.interfaceOnly && backend.healthy,
          'backend--down': !backend.interfaceOnly && !backend.healthy,
          'backend--stub': backend.interfaceOnly,
          'fluent-accent-edge--ok': !backend.interfaceOnly && backend.healthy,
          'fluent-accent-edge--down': !backend.interfaceOnly && !backend.healthy,
          'fluent-accent-edge--stub': backend.interfaceOnly,
        }"
        @click="openInExplorer(`awam://${backend.id}`)"
      >
        <strong>{{ backend.name }}</strong>
        <span
          class="fluent-chip status-chip"
          :class="{
            'fluent-chip--stub': backend.interfaceOnly,
            'fluent-chip--ok': !backend.interfaceOnly && backend.healthy,
            'fluent-chip--down': !backend.interfaceOnly && !backend.healthy,
          }"
        >{{
          backend.interfaceOnly
            ? t("providers.interfaceOnly")
            : backend.healthy
              ? t("providers.statusOk")
              : t("providers.statusDown")
        }}</span>
        <span
          v-if="backend.source === 'gateway'"
          class="fluent-chip status-chip fluent-chip--ok"
        >{{ t("providers.sourceGateway") }}</span>
        <span
          v-else-if="backend.source === 'builtin'"
          class="fluent-chip status-chip fluent-chip--stub"
        >{{ t("providers.sourceBuiltin") }}</span>
        <span class="detail">{{ backend.detail || backend.description }}</span>
      </button>
    </div>

    <form class="uri-row" @submit.prevent="listUri()">
      <input
        v-model="uri"
        class="fluent-control"
        type="text"
        spellcheck="false"
        :placeholder="t('providers.uriPlaceholder')"
      />
      <button class="fluent-control fluent-control--accent" type="submit" :disabled="loading">
        {{ loading ? t("providers.listing") : t("providers.list") }}
      </button>
      <button
        class="fluent-control"
        type="button"
        :disabled="!uri.trim()"
        @click="openInExplorer(uri.trim() || PROVIDER_ROOT)"
      >
        {{ t("providers.openInExplorer") }}
      </button>
    </form>

    <p v-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>
    <p v-else-if="listing && listing.entries.length === 0" class="fluent-banner fluent-banner--info">
      {{ t("providers.emptyListing") }}
    </p>

    <div v-if="listing && listing.entries.length" class="fluent-surface list">
      <button
        v-for="entry in listing.entries"
        :key="entry.uri"
        type="button"
        class="fluent-list-row fluent-list-row--density row"
        @dblclick="openEntry(entry)"
      >
        <span
          class="kind"
          :data-kind="entry.isDir ? 'dir' : 'file'"
          :title="entry.isDir ? t('providers.folder') : t('providers.file')"
        >
          <FluentIcon :name="entryKindIcon(entry.isDir)" size="md" />
        </span>
        <span class="name">{{ entry.name }}</span>
        <span class="path">{{ entrySecondary(entry) }}</span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.providers-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--fluent-space-md);
}

.providers-header-text {
  min-width: 0;
  flex: 1;
}

.providers-header .fluent-control {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  flex: 0 0 auto;
}

.backends {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
  gap: var(--fluent-space-sm);
}

.backend {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--fluent-space-xs);
  min-height: var(--fluent-control-height-comfortable);
  padding: var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.backend--ok {
  background: color-mix(in srgb, var(--fluent-color-status-success-bg) 55%, var(--fluent-color-fill-control));
  border-color: var(--fluent-color-status-success);
}

.backend--down {
  background: color-mix(in srgb, var(--fluent-color-status-danger-bg) 45%, var(--fluent-color-fill-control));
  border-color: var(--fluent-color-status-danger);
}

.backend--stub {
  background: var(--fluent-color-fill-layer-alt);
  opacity: 0.85;
}

.detail {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.status-chip {
  justify-self: start;
  min-height: var(--fluent-control-height-standard);
  pointer-events: none;
  cursor: default;
  font-size: var(--fluent-font-size-caption);
}

.uri-row {
  display: flex;
  gap: var(--fluent-space-sm);
  align-items: center;
  flex-wrap: wrap;
}

.uri-row .fluent-control[type="text"],
.uri-row .fluent-control[type="url"] {
  flex: 1;
  min-width: 12rem;
}

.gateway-row {
  margin-block: var(--fluent-space-sm);
}

.gateway-label {
  flex: 0 0 auto;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.list {
  display: flex;
  flex-direction: column;
}

.row {
  width: 100%;
  border: 0;
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: 0;
  background: transparent;
  color: inherit;
  text-align: left;
  display: grid;
  grid-template-columns: 2rem 10rem 1fr;
  gap: var(--fluent-space-sm);
  cursor: pointer;
}

.row:last-child {
  border-bottom: 0;
}

.row:hover {
  background: var(--fluent-color-fill-subtle);
}

.kind {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--fluent-color-text-secondary);
}

.kind[data-kind="dir"] {
  color: var(--fluent-color-fill-accent);
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: var(--fluent-font-weight-semibold);
}

.path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
</style>
