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
  <section class="panel">
    <h1>Health</h1>
    <p class="muted">S0 smoke — Rust core reports identity via <code>health_get</code>.</p>

    <p v-if="loading">Loading…</p>
    <p v-else-if="error" class="error">Health failed: {{ error }}</p>
    <dl v-else-if="health" class="grid">
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
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  max-width: 40rem;
}

h1 {
  margin: 0;
  font-size: var(--fluent-font-size-title);
  font-weight: var(--fluent-font-weight-semibold);
}

.muted {
  margin: 0;
  color: var(--fluent-color-text-secondary);
}

.error {
  color: #c42b1c;
}

.grid {
  display: grid;
  grid-template-columns: 8rem 1fr;
  gap: var(--fluent-space-sm) var(--fluent-space-md);
  margin: 0;
  padding: var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-control);
}

dt {
  color: var(--fluent-color-text-secondary);
}

dd {
  margin: 0;
  font-family: Consolas, "Courier New", monospace;
}

button {
  align-self: flex-start;
}
</style>
