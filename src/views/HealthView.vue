<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";

type HealthPayload = {
  ok: boolean;
  name: string;
  version: string;
  identifier: string;
  aiAddon?: boolean;
};

type DependencyItem = {
  id: string;
  kind: string;
  status: string;
  title: string;
  detail: string;
  fix: string;
};

type DependencyReport = {
  generatedAtMs: number;
  items: DependencyItem[];
  requiredOk: boolean;
  missingRequired: number;
  missingOptional: number;
};

const { t, te } = useI18n();
const health = ref<HealthPayload | null>(null);
const deps = ref<DependencyReport | null>(null);
const error = ref<string | null>(null);
const loading = ref(true);

function depTitle(item: DependencyItem): string {
  const key = `health.deps.items.${item.id}.title`;
  return te(key) ? String(t(key)) : item.title;
}

function statusLabel(status: string): string {
  const key = `health.deps.status.${status}`;
  return te(key) ? String(t(key)) : status;
}

function kindLabel(kind: string): string {
  const key = `health.deps.kind.${kind}`;
  return te(key) ? String(t(key)) : kind;
}

const summaryText = computed(() => {
  if (!deps.value) return "";
  if (!deps.value.requiredOk) {
    return t("health.deps.summaryRequiredMissing", {
      count: deps.value.missingRequired,
    });
  }
  if (deps.value.missingOptional > 0) {
    return t("health.deps.summaryOptionalMissing", {
      count: deps.value.missingOptional,
    });
  }
  return t("health.deps.summaryAllOk");
});

async function loadHealth() {
  loading.value = true;
  error.value = null;
  try {
    const [h, d] = await Promise.all([
      invoke<HealthPayload>("health_get"),
      invoke<DependencyReport>("deps_probe_get"),
    ]);
    health.value = h;
    deps.value = d;
  } catch (err) {
    health.value = null;
    deps.value = null;
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
      <h1 class="fluent-page-title">{{ t("health.title") }}</h1>
      <p class="fluent-page-meta">{{ t("health.meta") }}</p>
    </header>

    <p v-if="loading" class="fluent-banner">{{ t("health.loading") }}</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">
      {{ t("health.failed", { error }) }}
    </p>
    <template v-else>
      <p
        v-if="health"
        class="fluent-banner"
        :class="health.ok ? 'fluent-banner--success' : 'fluent-banner--error'"
      >
        <span
          class="fluent-chip"
          :class="health.ok ? 'fluent-chip--ok' : 'fluent-chip--down'"
        >{{ health.ok ? "ok" : "down" }}</span>
        {{ health.name }} · v{{ health.version }}
      </p>
      <dl v-if="health" class="fluent-surface grid">
        <dt>{{ t("health.fieldOk") }}</dt>
        <dd :class="health.ok ? 'fluent-status-text--ok' : 'fluent-status-text--down'">
          {{ health.ok }}
        </dd>
        <dt>{{ t("health.fieldName") }}</dt>
        <dd>{{ health.name }}</dd>
        <dt>{{ t("health.fieldVersion") }}</dt>
        <dd>{{ health.version }}</dd>
        <dt>{{ t("health.fieldIdentifier") }}</dt>
        <dd>{{ health.identifier }}</dd>
        <dt>{{ t("health.fieldAiAddon") }}</dt>
        <dd>{{ health.aiAddon ? t("health.aiAddonOn") : t("health.aiAddonOff") }}</dd>
      </dl>

      <h2 class="deps-heading">{{ t("health.deps.title") }}</h2>
      <p class="fluent-page-meta">{{ t("health.deps.meta") }}</p>
      <p
        v-if="deps"
        class="fluent-banner"
        :class="
          !deps.requiredOk
            ? 'fluent-banner--error'
            : deps.missingOptional > 0
              ? 'fluent-banner--warn'
              : 'fluent-banner--success'
        "
      >
        {{ summaryText }}
      </p>

      <ul v-if="deps" class="deps-list" data-health-deps>
        <li
          v-for="item in deps.items"
          :key="item.id"
          class="deps-card fluent-surface"
          :class="`deps-card--${item.status}`"
          :data-dep-id="item.id"
          :data-dep-status="item.status"
        >
          <div class="deps-card__head">
            <span
              class="fluent-chip"
              :class="{
                'fluent-chip--ok': item.status === 'ok',
                'fluent-chip--down': item.status === 'missing',
                'fluent-chip--warn': item.status === 'warn',
              }"
            >{{ statusLabel(item.status) }}</span>
            <span class="deps-kind">{{ kindLabel(item.kind) }}</span>
            <strong class="deps-title">{{ depTitle(item) }}</strong>
          </div>
          <p class="deps-detail">{{ item.detail }}</p>
          <p class="deps-fix">
            <span class="deps-fix-label">{{ t("health.deps.fixLabel") }}</span>
            {{ item.fix }}
          </p>
        </li>
      </ul>
    </template>

    <button class="fluent-control fluent-control--accent" type="button" @click="loadHealth">
      {{ t("health.refresh") }}
    </button>
  </section>
</template>

<style scoped>
.panel {
  max-width: 44rem;
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

.fluent-banner {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fluent-space-sm);
}

.fluent-banner--warn {
  background: color-mix(in srgb, var(--fluent-color-status-caution-bg, #fff4ce) 70%, transparent);
  border: 1px solid var(--fluent-color-status-caution, #9a6700);
  color: var(--fluent-color-text-primary);
  border-radius: var(--fluent-radius-md, 6px);
  padding: var(--fluent-space-sm) var(--fluent-space-md);
}

.fluent-chip {
  pointer-events: none;
  cursor: default;
  font-size: var(--fluent-font-size-caption);
}

.fluent-chip--warn {
  background: color-mix(in srgb, var(--fluent-color-status-caution, #9a6700) 18%, transparent);
  color: var(--fluent-color-status-caution, #9a6700);
}

.deps-heading {
  margin: var(--fluent-space-lg) 0 var(--fluent-space-xs);
  font-size: var(--fluent-font-size-subtitle);
}

.deps-list {
  list-style: none;
  margin: 0 0 var(--fluent-space-md);
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.deps-card {
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  border-left: 3px solid var(--fluent-color-stroke-divider, #d1d1d1);
}

.deps-card--ok {
  border-left-color: var(--fluent-color-status-success, #0f7b0f);
}

.deps-card--missing {
  border-left-color: var(--fluent-color-status-danger, #c50f1f);
}

.deps-card--warn {
  border-left-color: var(--fluent-color-status-caution, #9a6700);
}

.deps-card__head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fluent-space-sm);
  margin-bottom: var(--fluent-space-xs);
}

.deps-kind {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.deps-title {
  font-weight: 600;
}

.deps-detail,
.deps-fix {
  margin: var(--fluent-space-2xs, 0.25rem) 0 0;
  font-size: var(--fluent-font-size-body);
  line-height: 1.45;
}

.deps-fix {
  color: var(--fluent-color-text-secondary);
}

.deps-fix-label {
  font-weight: 600;
  color: var(--fluent-color-text-primary);
  margin-right: 0.35rem;
}

button {
  align-self: flex-start;
}
</style>
