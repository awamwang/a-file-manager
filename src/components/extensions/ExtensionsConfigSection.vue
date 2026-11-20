<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 扩展 → 服务配置分区：嵌入 ExtensionsSettingsPanel（后端服务 URL/令牌配置）。
 * ADR-0023: 收编 /settings/extensions 为扩展页分区。
 */
import ExtensionsSettingsPanel from "../settings/ExtensionsSettingsPanel.vue";
</script>

<template>
  <div class="config-section">
    <ExtensionsSettingsPanel />
  </div>
</template>

<style scoped>
.config-section {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}
.config-section :deep(.card) {
  padding: 0;
}
</style>
