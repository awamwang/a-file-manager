<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import AppShell from "./components/shell/AppShell.vue";
import { applyAppLocaleFromValues } from "./i18n/index";
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
  })();
});
</script>

<template>
  <AppShell>
    <RouterView />
  </AppShell>
</template>
