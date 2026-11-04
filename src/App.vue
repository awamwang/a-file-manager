<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import AppShell from "./components/shell/AppShell.vue";
import { applyAppLocaleFromValues } from "./i18n/index";
import { preloadProviderCache } from "./state/providerCache";
import { applyUiChromeFromValues } from "./state/uiChrome";

type SettingsDocument = {
  version: number;
  values: Record<string, unknown>;
};

onMounted(() => {
  void (async () => {
    try {
      const doc = await invoke<SettingsDocument>("settings_get");
      applyUiChromeFromValues(doc.values ?? {});
      applyAppLocaleFromValues(doc.values ?? {});
    } catch {
      applyUiChromeFromValues({});
      applyAppLocaleFromValues({});
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
