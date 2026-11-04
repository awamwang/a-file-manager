<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

type HealthPayload = {
  ok: boolean;
  name: string;
  version: string;
  identifier: string;
};

const health = ref<HealthPayload | null>(null);
const error = ref<string | null>(null);
const loading = ref(true);

async function loadHealth() {
  loading.value = true;
  error.value = null;
  try {
    health.value = await invoke<HealthPayload>("health_get");
  } catch (err) {
    health.value = null;
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  void loadHealth();
});
</script>

<template>
  <section class="fluent-page panel">
    <header class="fluent-page-header">
      <h1 class="fluent-page-title">Health</h1>
      <p class="fluent-page-meta">
        Rust core identity via <code>health_get</code>
      </p>
    </header>

    <p v-if="loading" class="fluent-banner">Loading…</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">
      Health failed: {{ error }}
    </p>
    <dl v-else-if="health" class="fluent-surface grid">
      <dt>ok</dt>
      <dd>{{ health.ok }}</dd>
      <dt>name</dt>
      <dd>{{ health.name }}</dd>
      <dt>version</dt>
      <dd>{{ health.version }}</dd>
      <dt>identifier</dt>
      <dd>{{ health.identifier }}</dd>
    </dl>

    <button class="fluent-control fluent-control--accent" type="button" @click="loadHealth">
      Refresh
    </button>
  </section>
</template>

<style scoped>
.panel {
  max-width: 40rem;
}

.grid {
  display: grid;
  grid-template-columns: 8rem 1fr;
  gap: var(--fluent-space-sm) var(--fluent-space-md);
  margin: 0;
  padding: var(--fluent-space-md) var(--fluent-space-lg);
}

dt {
  color: var(--fluent-color-text-secondary);
}

dd {
  margin: 0;
  font-family: var(--fluent-font-family-mono);
}

button {
  align-self: flex-start;
}
</style>
