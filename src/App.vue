<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted } from "vue";
import AppShell from "./components/shell/AppShell.vue";
import { loadUiChromeFromSettings } from "./state/uiChrome";

onMounted(() => {
  void loadUiChromeFromSettings();
});
</script>

<template>
  <AppShell>
    <RouterView />
  </AppShell>
</template>
