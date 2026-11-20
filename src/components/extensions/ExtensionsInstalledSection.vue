<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 扩展 → 已安装分区：Feature Pack 列表，启用/禁用/升级/卸载。
 * ADR-0023: 从 ExtensionsView 提取为独立分区组件。
 */
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { openConfirmDialog } from "../../state/confirmDialog";
import { openPromptDialog } from "../../state/promptDialog";
import { toastInfo, toastSuccess } from "../../state/toast";
import { reloadContributions } from "../../state/uiReload";
import FluentIcon from "../icons/FluentIcon.vue";

const { t } = useI18n();

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

type ManagedDep = {
  id: string;
  packId: string;
  mode: string;
  health: string;
  spawnedPid: number | null;
  declaredVersion: string;
  installedVersion: string;
  availableVersion: string;
};

const packs = ref<InstalledPack[]>([]);
const error = ref<string | null>(null);
const loading = ref(false);
const loadedOnce = ref(false);
const packBusy = ref(false);
const managed = ref<Record<string, ManagedDep>>({});
const upgrading = ref<Record<string, boolean>>({});
const checking = ref(false);

function invokeErrorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function packStateLabel(state: string): string {
  const key = `extensions.packState.${state}`;
  const label = t(key);
  return label === key ? state : label;
}

async function refresh() {
  if (!loadedOnce.value) loading.value = true;
  error.value = null;
  try {
    packs.value = await invoke<InstalledPack[]>("extension_host_list_packs");
    await loadManaged();
  } catch (err) {
    error.value = invokeErrorMessage(err);
  } finally {
    loading.value = false;
    loadedOnce.value = true;
  }
}

async function loadManaged() {
  try {
    const records = await invoke<ManagedDep[]>("managed_dependency_list");
    const map: Record<string, ManagedDep> = {};
    for (const r of records) map[r.packId] = r;
    managed.value = map;
  } catch {
    managed.value = {};
  }
}

function upgradeAvailable(packId: string): ManagedDep | null {
  const rec = managed.value[packId];
  return rec && rec.mode === "managed" && rec.availableVersion ? rec : null;
}

async function upgradePack(packId: string) {
  upgrading.value = { ...upgrading.value, [packId]: true };
  error.value = null;
  try {
    await invoke("managed_dependency_upgrade", { id: packId });
    toastSuccess(t("extensions.noticePackUpgraded", { id: packId }));
    await refresh();
  } catch (err) {
    error.value = invokeErrorMessage(err);
  } finally {
    upgrading.value = { ...upgrading.value, [packId]: false };
  }
}

async function checkAllUpdates() {
  checking.value = true;
  error.value = null;
  try {
    const records = await invoke<ManagedDep[]>("managed_dependency_check_all_updates");
    const map: Record<string, ManagedDep> = {};
    for (const r of records) map[r.packId] = r;
    managed.value = map;
    toastSuccess(t("extensions.noticeCheckedUpdates"));
  } catch (err) {
    error.value = invokeErrorMessage(err);
  } finally {
    checking.value = false;
  }
}

async function installPackFromLocalPath() {
  const picked = await open({
    directory: true,
    multiple: false,
    title: t("extensions.installPathTitle"),
  });
  let sourcePath = typeof picked === "string" ? picked.trim() : "";
  if (!sourcePath) {
    const prompted = await openPromptDialog({
      title: t("extensions.installPathTitle"),
      message: t("extensions.installPathHint"),
      label: t("extensions.installPathLabel"),
      placeholder: t("extensions.installPathPlaceholder"),
      confirmLabel: t("extensions.installFromFolder"),
      cancelLabel: t("common.cancel"),
    });
    if (!prompted.confirmed) return;
    sourcePath = prompted.value.trim();
  }
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
    toastSuccess(t("extensions.noticePackInstalled", { id: result.pack?.id ?? "" }));
    await refresh();
    await reloadContributions();
  } catch (err) {
    error.value = invokeErrorMessage(err);
  } finally {
    packBusy.value = false;
  }
}

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
    await refresh();
    await reloadContributions();
    if (enabled) {
      try {
        await invoke("managed_dependency_ensure", { id });
      } catch (err) {
        toastInfo(t("extensions.noticePackEnsureDeferred", { error: invokeErrorMessage(err) }));
      }
    }
  } catch (err) {
    error.value = invokeErrorMessage(err);
  } finally {
    packBusy.value = false;
  }
}

function onPackEnabledChange(pack: InstalledPack, event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  void setPackEnabled(pack.id, checked);
}

async function uninstallPack(pack: InstalledPack) {
  const confirmed = await openConfirmDialog({
    title: t("extensions.uninstallPackTitle"),
    message: t("extensions.uninstallPackMessage", { name: pack.name, id: pack.id }),
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
    await refresh();
    await reloadContributions();
  } catch (err) {
    error.value = invokeErrorMessage(err);
  } finally {
    packBusy.value = false;
  }
}

defineExpose({ refresh });

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="installed-section">
    <div class="row-head">
      <h3 class="section-title">
        <span class="fluent-icon-wash"><FluentIcon name="library" size="md" /></span>
        {{ t("extensions.packsTitle") }}
      </h3>
      <div class="actions">
        <button
          class="fluent-control"
          type="button"
          :disabled="packBusy || checking"
          @click="checkAllUpdates"
        >{{ checking ? t("extensions.checkingUpdates") : t("extensions.checkUpdates") }}</button>
        <button
          class="fluent-control fluent-control--accent"
          type="button"
          :disabled="packBusy"
          @click="installPackFromLocalPath"
        >
          {{ t("extensions.installFromFolder") }}
        </button>
      </div>
    </div>
    <p class="fluent-page-meta">{{ t("extensions.packsMeta") }}</p>
    <p class="fluent-banner fluent-banner--info">{{ t("extensions.packsLocalOnly") }}</p>
    <p class="fluent-page-meta">{{ t("extensions.reloadHint") }}</p>

    <p v-if="loading" class="fluent-page-meta">{{ t("common.loading") }}</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>
    <p v-else-if="!packs.length" class="fluent-page-meta">{{ t("extensions.noPacks") }}</p>
    <ul v-else class="pack-list">
      <li v-for="pack in packs" :key="pack.id">
        <div>
          <strong>{{ pack.name }}</strong>
          <span class="fluent-page-meta"> {{ pack.id }} · v{{ pack.version }}</span>
          <span
            class="fluent-chip fluent-chip--status state-chip"
            :class="{
              'fluent-chip--ok': pack.state === 'enabled',
              'fluent-chip--stub': pack.state === 'disabled' || pack.state === 'installed',
              'fluent-chip--down': pack.state === 'crashed',
            }"
          >{{ packStateLabel(pack.state) }}</span>
          <span
            v-if="upgradeAvailable(pack.id)"
            class="fluent-chip fluent-chip--warn state-chip"
          >{{ t("extensions.packUpgradeAvailable", { to: upgradeAvailable(pack.id)?.availableVersion }) }}</span>
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
            v-if="upgradeAvailable(pack.id)"
            class="fluent-control fluent-control--accent"
            type="button"
            :disabled="packBusy || upgrading[pack.id] === true"
            @click="upgradePack(pack.id)"
          >{{ upgrading[pack.id] ? t("extensions.upgradingPack") : t("extensions.upgradePack") }}</button>
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
  </div>
</template>

<style scoped>
.installed-section {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
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
  font-size: var(--fluent-font-size-subtitle);
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
  align-items: center;
}
.pack-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}
.pack-list li {
  display: flex;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  flex-wrap: wrap;
  padding-bottom: var(--fluent-space-md);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}
.pack-toggle {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  min-height: var(--fluent-control-height-standard);
}
.state-chip {
  min-height: 22px;
  pointer-events: none;
  cursor: default;
  font-size: var(--fluent-font-size-caption);
}
.inline {
  margin-top: var(--fluent-space-sm);
  padding: var(--fluent-space-sm) var(--fluent-space-md);
}
</style>
