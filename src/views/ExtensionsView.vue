<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * ADR-0023 — 扩展 = 能力后端统一中心（左侧分区导航单页）。
 * 收编原 /providers、/settings/extensions 为内部路由子段。
 * 布局模式同 AiView：左 nav + 右 panel。
 */
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import FluentIcon from "../components/icons/FluentIcon.vue";
import type { FluentGlyphId } from "../components/icons/glyphs.ts";

import ExtensionsOverviewSection from "../components/extensions/ExtensionsOverviewSection.vue";
import ExtensionsInstalledSection from "../components/extensions/ExtensionsInstalledSection.vue";
import ExtensionsStoreSection from "../components/extensions/ExtensionsStoreSection.vue";
import ExtensionsProvidersSection from "../components/extensions/ExtensionsProvidersSection.vue";
import ExtensionsConfigSection from "../components/extensions/ExtensionsConfigSection.vue";

const { t } = useI18n();
const route = useRoute();
const router = useRouter();

type SectionId = "overview" | "installed" | "store" | "providers" | "config" | "tool-runtime";

type SectionDef = {
  id: SectionId;
  labelKey: string;
  icon: FluentGlyphId;
};

const SECTIONS: SectionDef[] = [
  { id: "overview", labelKey: "extensions.sections.overview", icon: "viewAll" },
  { id: "installed", labelKey: "extensions.sections.installed", icon: "library" },
  { id: "store", labelKey: "extensions.sections.store", icon: "cloud" },
  { id: "providers", labelKey: "extensions.sections.providers", icon: "cloud" },
  { id: "config", labelKey: "extensions.sections.config", icon: "settings" },
  { id: "tool-runtime", labelKey: "extensions.sections.toolRuntime", icon: "flash" },
];

const activeSection = ref<SectionId>("overview");

const activeDef = computed(
  () => SECTIONS.find((s) => s.id === activeSection.value) ?? SECTIONS[0],
);

function sectionFromPath(path: string): SectionId {
  const seg = path.replace("/extensions", "").replace(/^\//, "");
  if (seg === "installed") return "installed";
  if (seg === "store") return "store";
  if (seg === "providers") return "providers";
  if (seg === "config") return "config";
  if (seg === "tool-runtime") return "tool-runtime";
  return "overview";
}

function selectSection(id: SectionId) {
  activeSection.value = id;
  const target = id === "overview" ? "/extensions" : `/extensions/${id}`;
  if (route.path !== target) {
    router.push(target);
  }
}

function syncFromRoute() {
  activeSection.value = sectionFromPath(route.path);
}

// Host status (kept at view level, shown in header)
type ExtensionHostStatus = {
  apiVersion: string;
  installedCount: number;
  enabledCount: number;
  crashedCount: number;
};
const status = ref<ExtensionHostStatus | null>(null);

async function loadHostStatus() {
  try {
    status.value = await invoke<ExtensionHostStatus>("extension_host_status");
  } catch {
    status.value = null;
  }
}

onMounted(() => {
  syncFromRoute();
  void loadHostStatus();
});

watch(() => route.path, syncFromRoute);
</script>

<template>
  <section class="fluent-page">
    <header class="fluent-page-header">
      <h1 class="fluent-page-title">{{ t("extensions.title") }}</h1>
      <p class="fluent-page-meta">
        {{ t("extensions.hostPrefix", { version: status?.apiVersion ?? "feature-pack.1" }) }} ·
        {{ t("extensions.meta") }}
      </p>
    </header>

    <div class="ext-layout">
      <aside class="ext-nav fluent-surface" role="tablist" :aria-label="t('extensions.title')">
        <button
          v-for="section in SECTIONS"
          :key="section.id"
          type="button"
          class="ext-nav-item"
          :class="{ 'ext-nav-item--active': activeSection === section.id }"
          role="tab"
          :aria-selected="activeSection === section.id"
          @click="selectSection(section.id)"
        >
          <span class="fluent-icon-wash">
            <FluentIcon :name="section.icon" size="md" />
          </span>
          <span class="ext-nav-label">{{ t(section.labelKey) }}</span>
        </button>
      </aside>

      <article
        class="fluent-surface ext-panel"
        role="tabpanel"
        :data-ext-section="activeSection"
      >
        <header class="ext-panel-head">
          <span class="fluent-icon-wash">
            <FluentIcon :name="activeDef.icon" size="md" />
          </span>
          <div class="ext-panel-titles">
            <h2 class="ext-panel-title">{{ t(activeDef.labelKey) }}</h2>
          </div>
        </header>

        <ExtensionsOverviewSection v-if="activeSection === 'overview'" />
        <ExtensionsInstalledSection v-else-if="activeSection === 'installed'" />
        <ExtensionsStoreSection v-else-if="activeSection === 'store'" />
        <ExtensionsProvidersSection v-else-if="activeSection === 'providers'" />
        <ExtensionsConfigSection v-else-if="activeSection === 'config'" />
        <div v-else-if="activeSection === 'tool-runtime'" class="tool-runtime-placeholder">
          <p class="fluent-page-meta">{{ t("extensions.sections.toolRuntimeHint") }}</p>
          <button
            class="fluent-control fluent-control--accent"
            type="button"
            @click="router.push('/tools')"
          >{{ t("extensions.sections.toolRuntimeOpen") }}</button>
        </div>
      </article>
    </div>
  </section>
</template>

<style scoped>
.ext-layout {
  display: grid;
  grid-template-columns: minmax(14rem, max-content) minmax(0, 1fr);
  gap: var(--fluent-space-lg);
  align-items: start;
  margin-top: var(--fluent-space-lg);
  max-width: min(100%, 88rem);
}

.ext-nav {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
  padding: var(--fluent-space-sm);
  background: var(--fluent-color-fill-layer-alt);
}

.ext-nav-item {
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

.ext-nav-item:hover {
  background: var(--fluent-color-fill-control-hover);
}

.ext-nav-item--active {
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-primary);
  box-shadow: inset 3px 0 0 var(--fluent-color-stroke-accent-soft);
}

.ext-nav-label {
  flex: 1;
  min-width: 0;
  white-space: nowrap;
}

.ext-panel {
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  min-width: 0;
}

.ext-panel-head {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-md);
  margin-bottom: var(--fluent-space-md);
}

.ext-panel-titles {
  flex: 1;
  min-width: 0;
}

.ext-panel-title {
  margin: 0;
  font-size: var(--fluent-font-size-body);
  font-weight: 600;
}

.tool-runtime-placeholder {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--fluent-space-md);
  padding: var(--fluent-space-lg);
}

@media (max-width: 720px) {
  .ext-layout {
    grid-template-columns: 1fr;
  }
}
</style>
