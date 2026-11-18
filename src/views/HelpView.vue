<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/** /help view — hosts the Help Center and handles `?entry=` / `?tour=1`. */
import { computed } from "vue";
import { useRoute } from "vue-router";
import { useI18n } from "vue-i18n";
import HelpCenter from "../help/components/HelpCenter.vue";

const route = useRoute();
const { t } = useI18n();

const entry = computed<string>(() => {
  const value = route.query["entry"];
  return typeof value === "string" ? value : "";
});

const autoTour = computed<boolean>(() => route.query["tour"] === "1");
</script>

<template>
  <section class="fluent-page help-view">
    <header class="fluent-page-header">
      <h1 class="fluent-page-title">{{ t("help.title") }}</h1>
      <p class="fluent-page-meta">{{ t("help.meta") }}</p>
    </header>
    <HelpCenter :initial-entry="entry" :auto-tour="autoTour" />
  </section>
</template>

<style scoped>
.help-view {
  overflow-y: auto;
}
</style>
