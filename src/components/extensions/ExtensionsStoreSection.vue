<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 扩展 → 商店分区：从 feed 列出可安装的 Feature Pack。
 * ADR-0023: 从 ExtensionsView 提取为独立分区组件。
 */
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { toastSuccess } from "../../state/toast";
import { reloadContributions } from "../../state/uiReload";
import FluentIcon from "../icons/FluentIcon.vue";

const { t } = useI18n();

type InstallablePack = {
  packId: string;
  name: string;
  version: string;
  installUrl: string;
  sha256: string;
  installed: boolean;
  installedVersion: string | null;
};

type PackInstallResult = {
  success: boolean;
  pack: { id: string } | null;
  error: string | null;
};

const installable = ref<InstallablePack[]>([]);
const feedError = ref<string | null>(null);
const packBusy = ref(false);
const loading = ref(false);

function invokeErrorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

async function load() {
  loading.value = true;
  feedError.value = null;
  try {
    installable.value = await invoke<InstallablePack[]>("extension_host_list_installable");
  } catch (err) {
    installable.value = [];
    feedError.value = invokeErrorMessage(err);
  } finally {
    loading.value = false;
  }
}

async function installFromFeed(item: InstallablePack) {
  packBusy.value = true;
  try {
    const result = await invoke<PackInstallResult>("extension_host_install_pack_from_feed", {
      packId: item.packId,
    });
    if (!result.success) {
      return;
    }
    toastSuccess(t("extensions.noticePackInstalled", { id: result.pack?.id ?? "" }));
    await load();
    await reloadContributions();
  } catch {
    // feed 安装失败由按钮 disabled + toast 通知处理
  } finally {
    packBusy.value = false;
  }
}

defineExpose({ refresh: load });

onMounted(() => {
  void load();
});
</script>

<template>
  <div class="store-section">
    <div class="row-head">
      <h3 class="section-title">
        <span class="fluent-icon-wash"><FluentIcon name="cloud" size="md" /></span>
        {{ t("extensions.storeTitle") }}
      </h3>
      <button class="fluent-control" type="button" :disabled="loading" @click="load">
        {{ t("common.refresh") }}
      </button>
    </div>
    <p class="fluent-page-meta">{{ t("extensions.storeMeta") }}</p>

    <p v-if="loading" class="fluent-page-meta">{{ t("common.loading") }}</p>
    <p v-else-if="feedError" class="fluent-banner fluent-banner--info">
      {{ t("extensions.storeUnavailable", { error: feedError }) }}
    </p>
    <p v-else-if="!installable.length" class="fluent-page-meta">{{ t("extensions.storeEmpty") }}</p>
    <ul v-else class="store-list">
      <li v-for="item in installable" :key="item.packId">
        <div>
          <strong>{{ item.name }}</strong>
          <span class="fluent-page-meta"> {{ item.packId }} · v{{ item.version }}</span>
          <span
            class="fluent-chip state-chip"
            :class="item.installed ? 'fluent-chip--ok' : 'fluent-chip--stub'"
          >{{ item.installed ? t("extensions.storeInstalled") : t("extensions.storeAvailable") }}</span>
        </div>
        <div class="actions">
          <button
            class="fluent-control fluent-control--accent"
            type="button"
            :disabled="packBusy || item.installed"
            @click="installFromFeed(item)"
          >
            {{ t("extensions.storeInstall") }}
          </button>
        </div>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.store-section {
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
.store-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}
.store-list li {
  display: flex;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  flex-wrap: wrap;
  padding-bottom: var(--fluent-space-md);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}
.state-chip {
  min-height: 22px;
  pointer-events: none;
  cursor: default;
  font-size: var(--fluent-font-size-caption);
}
</style>
