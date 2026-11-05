<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import AppShell from "./components/shell/AppShell.vue";
import { applyAppLocaleFromValues } from "./i18n/index";
import { preloadProviderCache } from "./state/providerCache";
import { hydrateSettings } from "./state/settingsHydrate";
import {
  STARTUP_PAGE_SETTING_KEY,
  startupRouteFor,
} from "./state/startupPage";
import { applyUiChromeFromValues } from "./state/uiChrome";

const router = useRouter();

onMounted(() => {
  void (async () => {
    let values: Record<string, unknown> = {};
    try {
      // AIA-181: single-flight settings hydrate (shared with close/viewMode/etc.).
      const doc = await hydrateSettings();
      values = doc.values ?? {};
      applyUiChromeFromValues(values);
      applyAppLocaleFromValues(values);
    } catch {
      applyUiChromeFromValues({});
      applyAppLocaleFromValues({});
    }
    // AIA-101: honor startup page once on cold launch (after `/` → /navigator redirect).
    const target = startupRouteFor(values[STARTUP_PAGE_SETTING_KEY]);
    const current = router.currentRoute.value.path;
    if (
      (current === "/" || current === "/navigator") &&
      target !== "/navigator"
    ) {
      await router.replace(target);
    }
    // AIA-76: warm depth-1 provider listings at app start (not Providers route).
    void preloadProviderCache();
  })();
});
</script>

<template>
  <AppShell>
    <RouterView />
  </AppShell>
</template>
