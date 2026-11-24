<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** 后端托管服务健康收敛事件名（与 managed_dependency::commands::MANAGED_HEALTH_EVENT 一致）。 */
const MANAGED_HEALTH_EVENT = "managed-dependency:health-changed";

defineProps<{ embedded?: boolean }>();
import { invoke } from "@tauri-apps/api/core";
import FluentIcon from "../components/icons/FluentIcon.vue";
import HelpLink from "../help/components/HelpLink.vue";
import HintBubble from "../help/components/HintBubble.vue";
import ProviderErrorBanner from "../components/common/ProviderErrorBanner.vue";
import { entryKindIcon } from "../components/icons/glyphs";
import {
  ensureProviderSession,
  preloadProviderCache,
  providerBackends,
  providerGateway,
  providerRootListing,
  providerSessionError,
  providerSessionHydrating,
  refreshProviderSession,
} from "../state/providerCache";
import { navigateTo } from "../state/navigatorBrowse";
import { openProviderEntry, PROVIDER_ROOT } from "../state/provider";
import { hydrateSettings } from "../state/settingsHydrate";
import PackConfigForm from "../components/extensions/PackConfigForm.vue";

const { t } = useI18n();
const router = useRouter();

const GATEWAY_URL_SETTING_KEY = "power.awamGatewayUrl";
const DEFAULT_GATEWAY_URL = "http://127.0.0.1:18787";

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

const backends = providerBackends;
const gateway = providerGateway;
const gatewayUrlDraft = ref(DEFAULT_GATEWAY_URL);
const gatewayUrlSaving = ref(false);
const refreshing = ref(false);
const uri = ref(PROVIDER_ROOT);
const listing = ref<DirListing | null>(providerRootListing.value);
const loading = ref(false);
const error = ref<string | null>(null);

/**
 * 卡片内联配置：声明来自各 Feature Pack 的 `config[]`（单一真相源），
 * 编辑面板做成卡片的浮层，卡片本身保持统一高度，不会把同排卡片拉高。
 */
const BACKEND_CONFIG: Record<string, { packId: string; ids: string[] }> = {
  prowlarr: { packId: "awam.file-gateway", ids: ["prowlarr-api-key"] },
};

const configOpenFor = ref<string | null>(null);
/** 浮层定位（fixed）：宽度按内容需要，左右/上下都夹在视口内，窄卡片也有足够长的输入框。 */
const configPos = ref<{ left: number; top: number; width: number } | null>(null);
const configPopRef = ref<HTMLElement | null>(null);

/** 浮层在 v-for 内，用函数 ref 取元素（普通 ref 在这里会变成数组）。 */
function setConfigPop(el: unknown) {
  configPopRef.value = el instanceof HTMLElement ? el : null;
}

function configSpecFor(backend: { id: string }): { packId: string; ids: string[] } | null {
  return BACKEND_CONFIG[backend.id] ?? null;
}

function closeConfig() {
  configOpenFor.value = null;
  configPos.value = null;
}

async function toggleConfig(backendId: string, event: MouseEvent) {
  if (configOpenFor.value === backendId) {
    closeConfig();
    return;
  }
  const trigger = event.currentTarget as HTMLElement | null;
  const anchor = (trigger?.closest(".backend-cell") as HTMLElement | null) ?? trigger;
  const rect = anchor?.getBoundingClientRect();
  if (!rect) return;
  const width = Math.min(26 * 16, window.innerWidth - 16);
  const left = Math.min(Math.max(8, rect.left), Math.max(8, window.innerWidth - width - 8));
  configOpenFor.value = backendId;
  configPos.value = { left, top: rect.bottom + 6, width };
  await nextTick();
  const pop = configPopRef.value?.getBoundingClientRect();
  if (pop && pop.bottom > window.innerHeight - 8) {
    const above = rect.top - pop.height - 6;
    configPos.value = { left, top: Math.max(8, above), width };
  }
}

function onDocumentPointerDown(event: PointerEvent) {
  if (!configOpenFor.value) return;
  const target = event.target as HTMLElement | null;
  if (target?.closest(".backend-cell--config-open")) return;
  closeConfig();
}

function onDocumentKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") closeConfig();
}

/** 浮层是 fixed 定位，页面滚动/窗口尺寸变化后位置失效，直接收起。 */
function onViewportChange() {
  if (configOpenFor.value) closeConfig();
}

/** 配置保存后网关会重启注入新值，重探一次状态，卡片随之更新。 */
function onConfigSaved() {
  void refreshStatus();
}

watch(providerRootListing, (next) => {
  if (uri.value === PROVIDER_ROOT || !uri.value.trim()) {
    listing.value = next;
  }
});

watch(providerSessionError, (next) => {
  if (next) error.value = next;
});

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

async function listUri(target?: string) {
  loading.value = true;
  error.value = null;
  try {
    if (target) uri.value = target;
    // 根目录优先复用会话缓存；其它 URI 走 provider_list（depth-1 仍可命中 Rust 缓存）。
    if (uri.value === PROVIDER_ROOT && providerRootListing.value && !refreshing.value) {
      listing.value = providerRootListing.value;
      if (listing.value.error) {
        error.value = listing.value.error;
      }
      return;
    }
    listing.value = await invoke<DirListing>("provider_list", { uri: uri.value });
    if (uri.value === PROVIDER_ROOT) {
      providerRootListing.value = listing.value;
    }
    if (listing.value.error) {
      error.value = listing.value.error;
    }
  } catch (err) {
    listing.value = null;
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
  // 与 Navigator 共用统一打开流程：本地路径定位 / 远程文件下载后定位 / URL 打开。
  try {
    await openProviderEntry(entry.uri);
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
    await refreshProviderSession();
    if (gateway.value?.baseUrl) {
      gatewayUrlDraft.value = gateway.value.baseUrl;
    }
    const target = uri.value.trim() || PROVIDER_ROOT;
    if (target === PROVIDER_ROOT) {
      listing.value = providerRootListing.value;
    } else {
      await listUri(target);
    }
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
    .then(() => ensureProviderSession())
    .then(() => {
      if (gateway.value?.baseUrl && gatewayUrlDraft.value === DEFAULT_GATEWAY_URL) {
        gatewayUrlDraft.value = gateway.value.baseUrl;
      }
      listing.value = providerRootListing.value;
      if (providerRootListing.value?.error) {
        error.value = providerRootListing.value.error;
      }
    })
    .catch((err) => {
      error.value = err instanceof Error ? err.message : String(err);
    });
  // 托管网关启动收敛（启动中→就绪）后自动重探，避免启动早期缓存的「网关不可用」停留到需手动刷新。
  void listen<string>(MANAGED_HEALTH_EVENT, (event) => {
    if (event.payload === "awam.file-gateway") {
      void refreshStatus();
    }
  }).then((fn) => {
    unlistenManagedHealth = fn;
  });
  window.addEventListener("pointerdown", onDocumentPointerDown);
  window.addEventListener("keydown", onDocumentKeydown);
  window.addEventListener("resize", onViewportChange);
  window.addEventListener("scroll", onViewportChange, true);
});

let unlistenManagedHealth: UnlistenFn | null = null;

onUnmounted(() => {
  unlistenManagedHealth?.();
  unlistenManagedHealth = null;
  window.removeEventListener("pointerdown", onDocumentPointerDown);
  window.removeEventListener("keydown", onDocumentKeydown);
  window.removeEventListener("resize", onViewportChange);
  window.removeEventListener("scroll", onViewportChange, true);
});
</script>

<template>
  <section class="fluent-page" :class="{ 'fluent-page--embedded': embedded }">
    <header v-if="!embedded" class="fluent-page-header" data-tour-id="tour-providers-header">
      <h1 class="fluent-page-title">{{ t("providers.title") }}</h1>
      <div class="fluent-page-help-row">
        <p class="fluent-page-meta">{{ t("providers.meta") }}</p>
        <HelpLink entry-id="providers" tour />
      </div>
    </header>

    <p
      v-if="gateway"
      class="gateway-status"
      :class="gateway.online ? 'gateway-status--ok' : 'gateway-status--down'"
    >
      <span class="gateway-dot" aria-hidden="true"></span>
      <span>{{
        gateway.online
          ? t("providers.gatewayOnline", {
              count: gateway.backendCount ?? backends.length,
              url: gateway.baseUrl,
            })
          : t("providers.gatewayOffline")
      }}</span>
    </p>

    <div class="uri-row gateway-row" data-tour-id="tour-providers-gateway">
      <span class="gateway-label">{{ t("providers.gatewayUrlLabel") }}</span>
      <code class="gateway-url-readonly">{{ gatewayUrlDraft }}</code>
      <span class="gateway-edit-hint">{{ t("providers.gatewayEditHint") }}</span>
      <button
        class="fluent-control gateway-refresh"
        data-tour-id="tour-providers-refresh"
        type="button"
        :disabled="refreshing || gatewayUrlSaving || providerSessionHydrating"
        :title="t('providers.refresh')"
        :aria-label="t('providers.refresh')"
        @click="refreshStatus"
      >
        <FluentIcon name="refresh" size="sm" />
        {{ refreshing ? t("providers.refreshing") : t("providers.refresh") }}
      </button>
    </div>

    <p v-if="!backends.length" class="fluent-banner fluent-banner--info">
      {{ t("providers.emptyBackends") }}
    </p>
    <div v-else class="backends" data-tour-id="tour-providers-backends">
      <div
        v-for="backend in backends"
        :key="backend.id"
        class="backend-cell"
        :class="{ 'backend-cell--config-open': configOpenFor === backend.id }"
      >
        <button
          type="button"
          class="backend"
          :class="{
            'fluent-accent-edge--ok': !backend.interfaceOnly && backend.healthy,
            'fluent-accent-edge--down': !backend.interfaceOnly && !backend.healthy,
            'fluent-accent-edge--stub': backend.interfaceOnly,
            'backend--configurable': !!configSpecFor(backend),
          }"
          @click="openInExplorer(`awam://${backend.id}`)"
        >
          <strong>{{ backend.name }}</strong>
          <span class="backend-chips">
            <span
              class="fluent-chip fluent-chip--status"
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
            <span v-if="backend.source === 'gateway'" class="fluent-chip fluent-chip--sm">{{
              t("providers.sourceGateway")
            }}</span>
            <span v-else-if="backend.source === 'builtin'" class="fluent-chip fluent-chip--sm">{{
              t("providers.sourceBuiltin")
            }}</span>
          </span>
          <span class="detail">{{ backend.detail || backend.description }}</span>
        </button>

        <!-- 卡片内联配置入口：面板是浮层，卡片高度与同排保持一致。 -->
        <button
          v-if="configSpecFor(backend)"
          type="button"
          class="backend-config-trigger"
          :class="{ 'backend-config-trigger--active': configOpenFor === backend.id }"
          :title="t('settings.extensions.packConfigHeading')"
          :aria-label="t('settings.extensions.packConfigHeading')"
          :aria-expanded="configOpenFor === backend.id ? 'true' : 'false'"
          @click="toggleConfig(backend.id, $event)"
        >
          <FluentIcon name="settings" size="sm" />
        </button>

        <div
          v-if="configOpenFor === backend.id && configPos"
          :ref="setConfigPop"
          class="backend-config-pop"
          role="dialog"
          :aria-label="t('settings.extensions.packConfigHeading')"
          :style="{
            left: `${configPos.left}px`,
            top: `${configPos.top}px`,
            width: `${configPos.width}px`,
          }"
        >
          <p class="backend-config-pop__title">
            {{ t("settings.extensions.packConfigHeading") }}
          </p>
          <PackConfigForm
            :pack-id="configSpecFor(backend)?.packId ?? ''"
            :only-ids="configSpecFor(backend)?.ids ?? []"
            @saved="onConfigSaved"
          />
        </div>
      </div>
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

    <ProviderErrorBanner v-if="error" :error="error" />
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
    <HintBubble entry-id="providers" />
  </section>
</template>

<style scoped>
.fluent-page--embedded {
  padding: 0;
}

.backends {
  display: grid;
  /* 提高列宽下限：宽窗口下避免堆出过多过窄卡片导致文字被挤碎。 */
  grid-template-columns: repeat(auto-fill, minmax(17rem, 1fr));
  /* 每张卡片各按自身内容高度排布，行高不再被相邻卡片拉齐。 */
  align-items: start;
  gap: var(--fluent-space-sm);
}

/* 卡片与配置浮层的定位容器；打开配置时抬高层级，浮层盖住下方卡片。 */
.backend-cell {
  position: relative;
  display: flex;
  min-width: 0;
  z-index: 1;
}

.backend-cell--config-open {
  z-index: 30;
}

.backend {
  flex: 1 1 auto;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--fluent-space-xs);
  min-width: 0;
  min-height: var(--fluent-control-height-comfortable);
  padding: var(--fluent-space-md);
  padding-inline-start: var(--fluent-space-lg);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition: background-color 80ms ease;
}

/* 带配置入口的卡片给右上角按钮让位，文字不钻到按钮下面。 */
.backend--configurable {
  padding-inline-end: 2.25rem;
}

.backend:hover {
  background: var(--fluent-color-fill-control-hover);
}

/* 卡片右上角的配置入口：低对比常驻，悬停/聚焦/展开时提亮。 */
.backend-config-trigger {
  position: absolute;
  inset-block-start: var(--fluent-space-xs);
  inset-inline-end: var(--fluent-space-xs);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  inline-size: 1.5rem;
  block-size: 1.5rem;
  padding: 0;
  border: 0;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: var(--fluent-color-text-secondary);
  opacity: 0.55;
  cursor: pointer;
  transition: opacity 80ms ease, background-color 80ms ease, color 80ms ease;
}

.backend-cell:hover .backend-config-trigger,
.backend-config-trigger:focus-visible,
.backend-config-trigger--active {
  opacity: 1;
}

.backend-config-trigger:hover,
.backend-config-trigger--active {
  background: var(--fluent-color-fill-control-hover);
  color: var(--fluent-color-text-primary);
}

.backend-config-trigger:focus-visible {
  outline: 2px solid var(--fluent-color-stroke-focus);
  outline-offset: -2px;
}

/* 配置浮层：fixed 定位（位置由打开时按卡片与视口计算），宽度独立于卡片，
   输入框因此始终有足够长度；卡片本身高度不变，同排卡片不会被拉高。 */
.backend-config-pop {
  position: fixed;
  z-index: 60;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  max-block-size: min(70vh, 32rem);
  overflow-y: auto;
  padding: var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  box-shadow: var(--fluent-shadow-flyout);
}

.backend-config-pop__title {
  margin: 0;
  font-size: var(--fluent-font-size-caption);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-secondary);
}

.backend > strong {
  font-weight: var(--fluent-font-weight-semibold);
}

/* 状态点标签 + 来源标签共用一行紧凑标签，避免多枚 pill 竖向堆叠。 */
.backend-chips {
  display: inline-flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fluent-space-xs);
}

.detail {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  max-width: 100%;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  white-space: normal;
  overflow-wrap: anywhere;
}

.uri-row {
  display: flex;
  gap: var(--fluent-space-sm);
  align-items: center;
  flex-wrap: wrap;
}

.uri-row .fluent-control[type="text"],
.uri-row .fluent-control[type="url"] {
  flex: 1 1 10rem;
  min-width: 0;
}

.gateway-row {
  margin-block: var(--fluent-space-sm);
}

.gateway-status {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  -webkit-user-select: text;
  user-select: text;
}

.gateway-dot {
  flex-shrink: 0;
  inline-size: 8px;
  block-size: 8px;
  border-radius: 50%;
  background: var(--fluent-color-text-disabled);
}

.gateway-status--ok .gateway-dot {
  background: var(--fluent-color-status-success);
}

.gateway-status--down .gateway-dot {
  background: var(--fluent-color-status-danger);
}

.gateway-label {
  flex: 0 0 auto;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.gateway-refresh {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  flex: 0 0 auto;
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
