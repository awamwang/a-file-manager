<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import AppShell from "./components/shell/AppShell.vue";
import { preloadProviderCache } from "./state/providerCache";
import { scheduleIdle } from "./state/scheduleIdle";
import {
  getCachedSettingsValues,
  hydrateSettings,
} from "./state/settingsHydrate";
import {
  STARTUP_PAGE_SETTING_KEY,
  startupRouteFor,
} from "./state/startupPage";
import { shellRemountKey } from "./state/uiReload";

const router = useRouter();

onMounted(() => {
  void (async () => {
    // Locale/chrome already applied in main.ts bootstrap (AIA-265).
    // Reuse the single-flight hydrate for startup-page routing only.
    let values: Record<string, unknown> = getCachedSettingsValues() ?? {};
    try {
      const doc = await hydrateSettings();
      values = doc.values ?? {};
    } catch {
      // Keep cached/empty values; locale already fell back in bootstrap.
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
    // AIA-181 P0: defer provider_preload off the first navigator_list window
    // (idle, or ProvidersView first visit also calls preloadProviderCache).
    scheduleIdle(() => {
      void preloadProviderCache();
    }, 4000);
  })();
});
</script>

<template>
  <!-- AIA-281: soft Reload Window remounts shell + KeepAlive without wiping data. -->
  <AppShell :key="shellRemountKey">
    <!-- AIA-276: keep Explorer warm across Settings/WG/etc. switches; other views remount. -->
    <RouterView v-slot="{ Component }">
      <KeepAlive :include="['NavigatorView']">
        <component :is="Component" />
      </KeepAlive>
    </RouterView>
  </AppShell>
</template>
