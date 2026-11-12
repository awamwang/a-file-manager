<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { openConfirmDialog } from "../state/confirmDialog";
import { openPromptDialog } from "../state/promptDialog";
import { toastInfo, toastSuccess } from "../state/toast";
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

type InstalledPack = {
  id: string;
  name: string;
  version: string;
  state: string;
  permissions: string[];
  installPath: string;
  lastError: string | null;
};

type PackInstallResult = {
  success: boolean;
  pack: InstalledPack | null;
  error: string | null;
};

const status = ref<ExtensionHostStatus | null>(null);
const catalog = ref<MarketplaceCatalog | null>(null);
const installed = ref<InstalledExtension[]>([]);
const packs = ref<InstalledPack[]>([]);
const hooks = ref<FsHook[]>([]);
const error = ref<string | null>(null);
const loading = ref(false);
const packBusy = ref(false);

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

function invokeErrorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function packStateLabel(state: string): string {
  const key = `extensions.packState.${state}`;
  const label = t(key);
  return label === key ? state : label;
}

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    status.value = await invoke<ExtensionHostStatus>("extension_host_status");
    catalog.value = await invoke<MarketplaceCatalog>("extension_host_marketplace");
    installed.value = await invoke<InstalledExtension[]>("extension_host_list_installed");
    packs.value = await invoke<InstalledPack[]>("extension_host_list_packs");
    hooks.value = await invoke<FsHook[]>("extension_host_list_fs_provider_hooks");
    if (catalog.value.fetchesRemote || status.value.fetchesRemote) {
      error.value = t("extensions.errorRemoteFetch");
    }
  } catch (err) {
    error.value = invokeErrorMessage(err);
  } finally {
    loading.value = false;
  }
}

async function installDemo() {
  const result = await invoke<InstallResult>("extension_host_install", {
    manifest: DEMO_MANIFEST,
  });
  if (!result.success) {
    error.value = result.error ?? t("extensions.errorInstallFailed");
    return;
  }
  toastSuccess(t("extensions.noticeInstalled", { id: result.extension?.id ?? "" }));
  await refresh();
}

async function setEnabled(id: string, enabled: boolean) {
  await invoke("extension_host_set_enabled", { extensionId: id, enabled });
  await refresh();
}

async function uninstall(id: string) {
  await invoke("extension_host_uninstall", { extensionId: id });
  toastSuccess(t("extensions.noticeUninstalled", { id }));
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
  if (result.ok) {
    toastSuccess(t("extensions.noticeHelloOk", { id }));
  } else {
    toastInfo(
      t("extensions.noticeHelloFail", {
        error: result.error ?? t("extensions.unknownError"),
      }),
    );
  }
  await refresh();
}

/** 从本机文件夹或 zip 安装 Feature Pack（非商店）。 */
async function installPackFromLocalPath() {
  const prompted = await openPromptDialog({
    title: t("extensions.installPathTitle"),
    message: t("extensions.installPathHint"),
    label: t("extensions.installPathLabel"),
    placeholder: t("extensions.installPathPlaceholder"),
    confirmLabel: t("extensions.installFromFolder"),
    cancelLabel: t("common.cancel"),
  });
  if (!prompted.confirmed) return;
  const sourcePath = prompted.value.trim();
  if (!sourcePath) return;
  packBusy.value = true;
  error.value = null;
  try {
    const result = await invoke<PackInstallResult>("extension_host_install_pack", {
      sourcePath,
    });
    if (!result.success) {
      error.value = result.error ?? t("extensions.errorInstallFailed");
      return;
    }
    toastSuccess(
      t("extensions.noticePackInstalled", { id: result.pack?.id ?? "" }),
    );
    await refresh();
  } catch (err) {
    error.value = invokeErrorMessage(err);
  } finally {
    packBusy.value = false;
  }
}

/** 启用或禁用已装 Feature Pack。 */
async function setPackEnabled(id: string, enabled: boolean) {
  packBusy.value = true;
  error.value = null;
  try {
    await invoke("extension_host_set_pack_enabled", { id, enabled });
    toastSuccess(
      enabled
        ? t("extensions.noticePackEnabled", { id })
        : t("extensions.noticePackDisabled", { id }),
    );
  } catch (err) {
    error.value = invokeErrorMessage(err);
  } finally {
    packBusy.value = false;
    await refresh();
  }
}

function onPackEnabledChange(pack: InstalledPack, event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  void setPackEnabled(pack.id, checked);
}

/** 从本机卸载 Feature Pack。 */
async function uninstallPack(pack: InstalledPack) {
  const confirmed = await openConfirmDialog({
    title: t("extensions.uninstallPackTitle"),
    message: t("extensions.uninstallPackMessage", {
      name: pack.name,
      id: pack.id,
    }),
    confirmLabel: t("extensions.uninstall"),
    cancelLabel: t("common.cancel"),
    danger: true,
  });
  if (!confirmed.confirmed) return;
  packBusy.value = true;
  error.value = null;
  try {
    await invoke("extension_host_uninstall_pack", { id: pack.id });
    toastSuccess(t("extensions.noticePackUninstalled", { id: pack.id }));
  } catch (err) {
    error.value = invokeErrorMessage(err);
  } finally {
    packBusy.value = false;
    await refresh();
  }
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

    <section class="fluent-surface card">
      <div class="row-head">
        <h2 class="section-title">
          <span class="fluent-icon-wash"><FluentIcon name="library" size="md" /></span>
          {{ t("extensions.packsTitle") }}
        </h2>
        <button
          class="fluent-control fluent-control--accent"
          type="button"
          :disabled="packBusy"
          @click="installPackFromLocalPath"
        >
          {{ t("extensions.installFromFolder") }}
        </button>
      </div>
      <p class="fluent-page-meta">{{ t("extensions.packsMeta") }}</p>
      <p class="fluent-banner fluent-banner--info">{{ t("extensions.packsLocalOnly") }}</p>
      <p class="fluent-page-meta">{{ t("extensions.reloadHint") }}</p>
      <p v-if="!packs.length" class="fluent-page-meta">{{ t("extensions.noPacks") }}</p>
      <ul v-else class="installed">
        <li v-for="pack in packs" :key="pack.id">
          <div>
            <strong>{{ pack.name }}</strong>
            <span class="fluent-page-meta"> {{ pack.id }} · v{{ pack.version }}</span>
            <span
              class="fluent-chip state-chip"
              :class="{
                'fluent-chip--ok': pack.state === 'enabled',
                'fluent-chip--stub': pack.state === 'disabled' || pack.state === 'installed',
                'fluent-chip--down': pack.state === 'crashed',
              }"
            >{{ packStateLabel(pack.state) }}</span>
            <p class="fluent-page-meta" :title="pack.installPath">{{ pack.installPath }}</p>
            <p v-if="pack.lastError" class="fluent-banner fluent-banner--error inline">
              {{ pack.lastError }}
            </p>
          </div>
          <div class="actions">
            <label class="pack-toggle">
              <input
                type="checkbox"
                class="fluent-field-check"
                :checked="pack.state === 'enabled'"
                :disabled="packBusy || pack.state === 'crashed'"
                @change="onPackEnabledChange(pack, $event)"
              />
              <span>{{ t("extensions.packEnable") }}</span>
            </label>
            <button
              class="fluent-control"
              type="button"
              :disabled="packBusy"
              @click="uninstallPack(pack)"
            >
              {{ t("extensions.uninstall") }}
            </button>
          </div>
        </li>
      </ul>
    </section>

    <div v-if="status" class="grid">
      <article class="fluent-surface card" :class="{ 'card--warn': status.crashedCount > 0 }">
        <h2 class="section-title">
          <span class="fluent-icon-wash"><FluentIcon name="heartPulse" size="md" /></span>
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
          {{ t("extensions.demoTitle") }}
        </h2>
        <button class="fluent-control fluent-control--accent" type="button" @click="installDemo">
          {{ t("extensions.installDemo") }}
        </button>
      </div>
      <p class="fluent-page-meta">{{ t("extensions.demoMeta") }}</p>
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
  align-items: center;
}

.pack-toggle {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  min-height: var(--fluent-control-height-standard);
}

@media (max-width: 800px) {
  .grid {
    grid-template-columns: 1fr;
  }
}
</style>
