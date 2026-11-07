<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * AIA-214 / A1 + AIA-215 / A2 — AI Add-on main menu.
 * Agents section hosts the Agent Target registry; other sections stay placeholders.
 */
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { AI_MENU_SECTIONS, type AiAddonStatus } from "../ai";
import AgentTargetsPanel from "../ai/AgentTargetsPanel.vue";
import SkillsLibraryPanel from "../ai/SkillsLibraryPanel.vue";
import McpCatalogPanel from "../ai/McpCatalogPanel.vue";
import HarnessInitPanel from "../ai/HarnessInitPanel.vue";
import FluentIcon from "../components/icons/FluentIcon.vue";

const { t } = useI18n();

const status = ref<AiAddonStatus | null>(null);
const error = ref<string | null>(null);
const loading = ref(false);

const readyById = computed(() => {
  const map = new Map<string, boolean>();
  for (const section of status.value?.sections ?? []) {
    map.set(section.id, section.ready);
  }
  return map;
});

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    status.value = await invoke<AiAddonStatus>("ai_addon_status");
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <section class="fluent-page">
    <header class="fluent-page-header">
      <h1 class="fluent-page-title">{{ t("ai.title") }}</h1>
      <p class="fluent-page-meta">{{ t("ai.meta") }}</p>
      <p class="fluent-page-meta usage-hint">{{ t("ai.usageHint") }}</p>
    </header>

    <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>
    <p v-else-if="status" class="fluent-banner">
      {{ t("ai.shellStatus", { version: status.shellVersion, layer: status.layer }) }}
    </p>

    <div class="ai-sections" role="list">
      <article
        v-for="section in AI_MENU_SECTIONS"
        :key="section.id"
        class="fluent-surface ai-section"
        role="listitem"
        :data-ai-section="section.id"
      >
        <div class="ai-section-head">
          <span class="fluent-icon-wash">
            <FluentIcon :name="section.icon" size="md" />
          </span>
          <div class="ai-section-titles">
            <h2 class="ai-section-title">{{ t(section.titleKey) }}</h2>
            <p class="ai-section-body">{{ t(section.bodyKey) }}</p>
          </div>
          <span
            class="fluent-chip"
            :class="
              readyById.get(section.id)
                ? 'fluent-chip--ok'
                : 'ai-section-chip--later'
            "
          >
            {{
              readyById.get(section.id)
                ? t("ai.sectionReady")
                : t("ai.sectionPlaceholder")
            }}
          </span>
        </div>

        <AgentTargetsPanel v-if="section.id === 'agents'" />
        <SkillsLibraryPanel v-else-if="section.id === 'skills'" />
        <McpCatalogPanel v-else-if="section.id === 'mcp'" />
      </article>
    </div>

    <article class="fluent-surface ai-section harness-block" data-ai-section="harness">
      <HarnessInitPanel />
    </article>
  </section>
</template>

<style scoped>
.usage-hint {
  opacity: 0.85;
}

.ai-sections {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  margin-top: var(--fluent-space-lg);
  max-width: 56rem;
}

.harness-block {
  margin-top: var(--fluent-space-md);
  max-width: 56rem;
  padding: var(--fluent-space-md) var(--fluent-space-lg);
}

.ai-section {
  padding: var(--fluent-space-md) var(--fluent-space-lg);
}

.ai-section-head {
  display: flex;
  align-items: flex-start;
  gap: var(--fluent-space-md);
}

.ai-section-titles {
  flex: 1;
  min-width: 0;
}

.ai-section-title {
  margin: 0 0 var(--fluent-space-xs);
  font-size: var(--fluent-font-size-body);
  font-weight: 600;
}

.ai-section-body {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  line-height: 1.45;
}

.ai-section-chip--later {
  flex-shrink: 0;
  opacity: 0.75;
}
</style>
