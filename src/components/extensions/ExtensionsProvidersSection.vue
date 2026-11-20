<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 扩展 → 提供程序分区：嵌入 ProvidersView，通过 embedded prop 隐藏重复的页面头。
 * ADR-0023: 收编 /providers 为扩展页分区。
 */
import ProvidersView from "../../views/ProvidersView.vue";
</script>

<template>
  <div class="providers-section">
    <ProvidersView embedded />
  </div>
</template>

<style scoped>
.providers-section {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}
</style>
