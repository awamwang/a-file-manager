<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 扩展 → 服务配置（集中页，保留但后续可能下线）。
 *
 * 字段不再硬编码，而是来自各已启用 Feature Pack 的 `contributions.config[]` 声明
 * （经 `extension_host_list_contributions` 聚合、`configExtras` 暴露）。每个 pack 一张卡片，
 * 卡片内容由共享的 [`PackConfigForm`] 渲染与保存；同一份表单也复用到各功能页的
 * 「本服务配置」区块，两处读写同一 settings key，不存在两套真相。
 */
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import PackConfigForm from "../extensions/PackConfigForm.vue";
import {
  configGroups,
  type ConfigGroup,
} from "../../state/extensionConfig";
import { hydrateFeaturePackContributions } from "../../state/featurePack/contributions";

const { t } = useI18n();

const groups = ref<ConfigGroup[]>([]);

function refreshGroups() {
  groups.value = configGroups();
}

onMounted(async () => {
  try {
    // 服务配置页直接进入时，确保已拉到最新 Pack 贡献（含 config 声明）。
    await hydrateFeaturePackContributions();
  } catch {
    /* 拉取失败保留上次贡献缓存 */
  }
  refreshGroups();
});

const hasGroups = computed(() => groups.value.length > 0);
</script>

<template>
  <section class="ext-config">
    <header class="ext-config__header">
      <h2 class="section-title">{{ t("settings.extensions.title") }}</h2>
      <p class="fluent-page-meta">{{ t("settings.extensions.meta") }}</p>
    </header>

    <div v-if="hasGroups" class="ext-config__grid">
      <section v-for="group in groups" :key="group.packId" class="fluent-surface section-card">
        <h3 class="section-card__title">{{ group.packName }}</h3>
        <PackConfigForm :pack-id="group.packId" @saved="refreshGroups" />
      </section>
    </div>
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
  grid-template-columns: repeat(auto-fill, minmax(min(100%, 26rem), 1fr));
  /* 各 pack 的配置项数量不同：卡片按内容高度排列，避免少字段的卡片被撑高。 */
  align-items: start;
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

@media (max-width: 720px) {
  .ext-config__grid {
    grid-template-columns: 1fr;
  }
}
</style>