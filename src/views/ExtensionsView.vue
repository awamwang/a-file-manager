<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import FluentIcon from "../components/icons/FluentIcon.vue";

const { t } = useI18n();

type ExtensionHostStatus = {
  apiVersion: string;
  installedCount: number;
  enabledCount: number;
  crashedCount: number;
  marketplaceEmpty: boolean;
  fetchesRemote: boolean;
  fsProviderHooks: number;
};

type MarketplaceCatalog = {
  apiVersion: string;
  extensions: unknown[];
  empty: boolean;
  detail: string;
  fetchesRemote: boolean;
};

type InstalledExtension = {
  id: string;
  name: string;
  version: string;
  state: string;
  permissions: string[];
  fsProviders: string[];
  uiCommands: string[];
  lastError: string | null;
};

type FsHook = {
  extensionId: string;
  backendId: string;
  registered: boolean;
};

type InstallResult = {
  success: boolean;
  extension: InstalledExtension | null;
  error: string | null;
};

const status = ref<ExtensionHostStatus | null>(null);
const catalog = ref<MarketplaceCatalog | null>(null);
const installed = ref<InstalledExtension[]>([]);
const hooks = ref<FsHook[]>([]);
const error = ref<string | null>(null);
const notice = ref<string | null>(null);
const loading = ref(false);

const DEMO_MANIFEST = {
  id: "demo.ext",
  name: "Demo Extension",
  version: "0.1.0",
  apiVersion: "vnext.1",
  description: "Local dogfood stub — not from marketplace",
  permissions: ["uiCommands", "fsProvider"],
  fsProviders: ["demo-fs"],
  uiCommands: ["hello", "__crash__"],
};

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    status.value = await invoke<ExtensionHostStatus>("extension_host_status");
    catalog.value = await invoke<MarketplaceCatalog>("extension_host_marketplace");
    installed.value = await invoke<InstalledExtension[]>("extension_host_list_installed");
    hooks.value = await invoke<FsHook[]>("extension_host_list_fs_provider_hooks");
    if (catalog.value.fetchesRemote || status.value.fetchesRemote) {
      error.value = t("extensions.errorRemoteFetch");
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    loading.value = false;
  }
}

async function installDemo() {
  notice.value = null;
  const result = await invoke<InstallResult>("extension_host_install", {
    manifest: DEMO_MANIFEST,
  });
  if (!result.success) {
    error.value = result.error ?? t("extensions.errorInstallFailed");
    return;
  }
  notice.value = t("extensions.noticeInstalled", { id: result.extension?.id ?? "" });
  await refresh();
}

async function setEnabled(id: string, enabled: boolean) {
  await invoke("extension_host_set_enabled", { extensionId: id, enabled });
  await refresh();
}

async function uninstall(id: string) {
  await invoke("extension_host_uninstall", { extensionId: id });
  notice.value = t("extensions.noticeUninstalled", { id });
  await refresh();
}

async function runHello(id: string) {
  const result = await invoke<{
    ok: boolean;
    error: string | null;
    isolatedFailure: boolean;
  }>("extension_host_dispatch_ui_command", {
    request: { extensionId: id, commandId: "hello", args: { ping: true } },
  });
  notice.value = result.ok
    ? t("extensions.noticeHelloOk", { id })
    : t("extensions.noticeHelloFail", {
        error: result.error ?? t("extensions.unknownError"),
      });
  await refresh();
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <section class="fluent-page">
    <header class="fluent-page-header">
      <h1 class="fluent-page-title">{{ t("extensions.title") }}</h1>
      <p class="fluent-page-meta">
        {{ t("extensions.hostPrefix", { version: status?.apiVersion ?? "vnext.1" }) }} ·
        {{ t("extensions.meta") }}
      </p>
    </header>

    <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>
    <p v-if="notice" class="fluent-banner fluent-banner--info banner--soft">
      {{ notice }}
      <button type="button" class="linkish" @click="notice = null">{{ t("common.dismiss") }}</button>
    </p>

    <div v-if="status" class="grid">
      <article class="fluent-surface card" :class="{ 'card--warn': status.crashedCount > 0 }">
        <h2 class="section-title">
          <span class="fluent-icon-wash"><FluentIcon name="heart" size="md" /></span>
          {{ t("extensions.hostStatus") }}
        </h2>
        <ul>
          <li>{{ t("extensions.api", { version: status.apiVersion }) }}</li>
          <li class="counts">
            <span class="fluent-chip fluent-chip--stub count-chip">{{
              t("extensions.counts", {
                installed: status.installedCount,
                enabled: status.enabledCount,
                crashed: status.crashedCount,
              })
            }}</span>
            <span
              v-if="status.crashedCount > 0"
              class="fluent-chip fluent-chip--down count-chip"
            >crashed: {{ status.crashedCount }}</span>
          </li>
          <li>{{ t("extensions.fsHooksCount", { count: status.fsProviderHooks }) }}</li>
          <li>{{
            t("extensions.marketplaceFlags", {
              empty: status.marketplaceEmpty ? t("common.yes") : t("common.no"),
              remote: status.fetchesRemote ? t("common.yes") : t("common.no"),
            })
          }}</li>
        </ul>
        <button class="fluent-control" type="button" @click="refresh">
          {{ t("common.refresh") }}
        </button>
      </article>

      <article class="fluent-surface card">
        <h2 class="section-title">
          <span class="fluent-icon-wash"><FluentIcon name="cloud" size="md" /></span>
          {{ t("extensions.marketplace") }}
        </h2>
        <p class="fluent-page-meta">{{ catalog?.detail }}</p>
        <p v-if="catalog?.empty" class="empty-mark">{{ t("extensions.catalogEmpty") }}</p>
        <p class="fluent-page-meta">{{ t("extensions.marketplacePolicy") }}</p>
      </article>
    </div>

    <section class="fluent-surface card">
      <div class="row-head">
        <h2 class="section-title">
          <span class="fluent-icon-wash"><FluentIcon name="puzzle" size="md" /></span>
          {{ t("extensions.installed") }}
        </h2>
        <button class="fluent-control fluent-control--accent" type="button" @click="installDemo">
          {{ t("extensions.installDemo") }}
        </button>
      </div>
      <p v-if="!installed.length" class="fluent-page-meta">{{ t("extensions.noInstalled") }}</p>
      <ul v-else class="installed">
        <li v-for="ext in installed" :key="ext.id">
          <div>
            <strong>{{ ext.name }}</strong>
            <span class="fluent-page-meta"> {{ ext.id }} · v{{ ext.version }}</span>
            <span
              class="fluent-chip state-chip"
              :class="{
                'fluent-chip--ok': ext.state === 'enabled',
                'fluent-chip--stub': ext.state === 'disabled',
                'fluent-chip--down': ext.state === 'crashed',
              }"
            >{{ ext.state }}</span>
            <p v-if="ext.lastError" class="fluent-banner fluent-banner--error inline">{{ ext.lastError }}</p>
          </div>
          <div class="actions">
            <button class="fluent-control" type="button" @click="runHello(ext.id)">
              {{ t("extensions.runHello") }}
            </button>
            <button class="fluent-control" type="button" @click="setEnabled(ext.id, true)">
              {{ t("extensions.enable") }}
            </button>
            <button class="fluent-control" type="button" @click="setEnabled(ext.id, false)">
              {{ t("extensions.disable") }}
            </button>
            <button class="fluent-control" type="button" @click="uninstall(ext.id)">
              {{ t("extensions.uninstall") }}
            </button>
          </div>
        </li>
      </ul>
    </section>

    <section class="fluent-surface card">
      <h2 class="section-title">
        <span class="fluent-icon-wash"><FluentIcon name="folder" size="md" /></span>
        {{ t("extensions.fsHooks") }}
      </h2>
      <p class="fluent-page-meta">{{ t("extensions.fsHooksMeta") }}</p>
      <ul>
        <li v-for="hook in hooks" :key="`${hook.extensionId}:${hook.backendId}`">
          {{ hook.extensionId }} → {{ hook.backendId }}
          <span class="fluent-page-meta">{{
            t("extensions.registered", { value: hook.registered })
          }}</span>
        </li>
        <li v-if="!hooks.length" class="fluent-page-meta">{{ t("extensions.noHooks") }}</li>
      </ul>
    </section>
  </section>
</template>

<style scoped>
.banner--soft {
  display: flex;
  justify-content: space-between;
  gap: var(--fluent-space-md);
}

.inline {
  margin-top: var(--fluent-space-sm);
  padding: var(--fluent-space-sm) var(--fluent-space-md);
}

.counts {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
  list-style: none;
}

.count-chip,
.state-chip {
  min-height: var(--fluent-control-height-standard);
  pointer-events: none;
  cursor: default;
  font-size: var(--fluent-font-size-caption);
}

.linkish {
  border: 0;
  background: transparent;
  color: var(--fluent-color-fill-accent);
  cursor: pointer;
  font: inherit;
}

.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--fluent-space-md);
}

.card {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  padding: var(--fluent-space-lg);
}

.card--warn {
  background: color-mix(in srgb, var(--fluent-color-status-warning-bg) 40%, var(--fluent-color-fill-control));
  border-color: var(--fluent-color-status-warning);
}

.card h2,
.section-title {
  margin: 0;
  font-size: var(--fluent-font-size-subtitle);
}

.section-title {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
}

.card ul {
  margin: 0;
  padding-left: var(--fluent-space-lg);
}

.empty-mark {
  margin: 0;
  font-weight: var(--fluent-font-weight-semibold);
}

.row-head {
  display: flex;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  flex-wrap: wrap;
  align-items: center;
}

.installed {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}

.installed li {
  display: flex;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  flex-wrap: wrap;
  padding-bottom: var(--fluent-space-md);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
}

@media (max-width: 800px) {
  .grid {
    grid-template-columns: 1fr;
  }
}
</style>
