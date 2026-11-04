<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
</script>

<template>
  <section class="panel">
    <h1>Settings</h1>
    <p class="empty">
      Settings schema not loaded. Rust schema authority arrives in S3 — this page is an S0 placeholder.
    </p>
  </section>
</template>

<style scoped>
.panel {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}

h1 {
  margin: 0;
  font-size: var(--fluent-font-size-title);
  font-weight: var(--fluent-font-weight-semibold);
}

.empty {
  margin: 0;
  padding: var(--fluent-space-lg);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
  color: var(--fluent-color-text-secondary);
}
</style>
