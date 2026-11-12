<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * AIA-214 / A1 + AIA-215 / A2 + AIA-263 — AI Add-on settings shell.
 * Left nav (Settings-style) separates Providers / MCP / Skills / Agents / Harness.
 */
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  AI_MENU_SECTIONS,
  aiAddonStatus,
  type AiAddonStatus,
  type AiMenuSectionId,
} from "../ai";
import AgentTargetsPanel from "../ai/AgentTargetsPanel.vue";
import SkillsLibraryPanel from "../ai/SkillsLibraryPanel.vue";
import McpCatalogPanel from "../ai/McpCatalogPanel.vue";
import HarnessInitPanel from "../ai/HarnessInitPanel.vue";
import ProvidersPanel from "../ai/ProvidersPanel.vue";
import FluentIcon from "../components/icons/FluentIcon.vue";

const { t } = useI18n();

const status = ref<AiAddonStatus | null>(null);
const error = ref<string | null>(null);
const loading = ref(false);
const activeSection = ref<AiMenuSectionId>(AI_MENU_SECTIONS[0]?.id ?? "providers");

const readyById = computed(() => {
  const map = new Map<string, boolean>();
  for (const section of status.value?.sections ?? []) {
    map.set(section.id, section.ready);
  }
  return map;
});

const activeDef = computed(
  () => AI_MENU_SECTIONS.find((section) => section.id === activeSection.value) ?? null,
);

function selectSection(id: AiMenuSectionId) {
  activeSection.value = id;
}

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    status.value = await aiAddonStatus();
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
  <section class="fluent-page" data-ai-settings>
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

    <div class="ai-layout">
      <aside class="ai-nav fluent-surface" role="tablist" :aria-label="t('ai.title')">
        <button
          v-for="section in AI_MENU_SECTIONS"
          :key="section.id"
          type="button"
          class="ai-nav-item"
          :class="{ 'ai-nav-item--active': activeSection === section.id }"
          role="tab"
          :aria-selected="activeSection === section.id"
          :data-ai-nav="section.id"
          @click="selectSection(section.id)"
        >
          <span class="fluent-icon-wash">
            <FluentIcon :name="section.icon" size="md" />
          </span>
          <span class="ai-nav-label">{{ t(section.titleKey) }}</span>
          <span
            class="fluent-chip ai-nav-chip"
            :class="readyById.get(section.id) ? 'fluent-chip--ok' : 'ai-nav-chip--later'"
          >
            {{
              readyById.get(section.id) ? t("ai.sectionReady") : t("ai.sectionPlaceholder")
            }}
          </span>
        </button>
      </aside>

      <article
        v-if="activeDef"
        class="fluent-surface ai-panel"
        role="tabpanel"
        :data-ai-section="activeDef.id"
      >
        <header class="ai-panel-head">
          <span class="fluent-icon-wash">
            <FluentIcon :name="activeDef.icon" size="md" />
          </span>
          <div class="ai-panel-titles">
            <h2 class="ai-panel-title">{{ t(activeDef.titleKey) }}</h2>
            <p class="ai-panel-body">{{ t(activeDef.bodyKey) }}</p>
          </div>
        </header>

        <ProvidersPanel v-if="activeDef.id === 'providers'" />
        <McpCatalogPanel v-else-if="activeDef.id === 'mcp'" />
        <SkillsLibraryPanel v-else-if="activeDef.id === 'skills'" />
        <AgentTargetsPanel v-else-if="activeDef.id === 'agents'" />
        <HarnessInitPanel v-else-if="activeDef.id === 'harness'" />
      </article>
    </div>
  </section>
</template>

<style scoped>
.usage-hint {
  opacity: 0.85;
}

.ai-layout {
  display: grid;
  grid-template-columns: minmax(18rem, max-content) minmax(0, 1fr);
  gap: var(--fluent-space-lg);
  align-items: start;
  margin-top: var(--fluent-space-lg);
  max-width: min(100%, 88rem);
}

.ai-nav {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
  padding: var(--fluent-space-sm);
  background: var(--fluent-color-fill-layer-alt);
}

.ai-nav-item {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  width: 100%;
  min-height: var(--fluent-control-height-comfortable);
  padding: var(--fluent-space-sm);
  border: 0;
  border-radius: var(--fluent-radius-md);
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.ai-nav-item:hover {
  background: var(--fluent-color-fill-control-hover);
}

.ai-nav-item--active {
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-primary);
  box-shadow: inset 3px 0 0 var(--fluent-color-stroke-accent-soft);
}

.ai-nav-label {
  flex: 1;
  min-width: 0;
  white-space: nowrap;
}

.ai-nav-chip {
  flex-shrink: 0;
  pointer-events: none;
}

.ai-nav-chip--later {
  opacity: 0.75;
}

.ai-panel {
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  min-width: 0;
}

.ai-panel-head {
  display: flex;
  align-items: flex-start;
  gap: var(--fluent-space-md);
  margin-bottom: var(--fluent-space-md);
}

.ai-panel-titles {
  flex: 1;
  min-width: 0;
}

.ai-panel-title {
  margin: 0 0 var(--fluent-space-xs);
  font-size: var(--fluent-font-size-body);
  font-weight: 600;
}

.ai-panel-body {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  line-height: 1.45;
}

@media (max-width: 720px) {
  .ai-layout {
    grid-template-columns: 1fr;
  }
}
</style>
