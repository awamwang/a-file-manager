<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
</script>

<template>
  <div class="app fluent-chrome">
    <!-- S0 titlebar stub region (Snap / real caption = later). Full chrome zones = S1. -->
    <header class="titlebar-stub" data-tauri-drag-region>
      <span class="brand">A File Manager</span>
      <span class="phase">S0 scaffold</span>
    </header>

    <nav class="nav" aria-label="Primary">
      <RouterLink to="/health">Health</RouterLink>
      <RouterLink to="/navigator">Navigator</RouterLink>
      <RouterLink to="/settings">Settings</RouterLink>
    </nav>

    <main class="content">
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  min-height: 100%;
  height: 100%;
}

.titlebar-stub {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--fluent-titlebar-height);
  padding-inline: var(--fluent-space-lg);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
  user-select: none;
}

.brand {
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.phase {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.nav {
  display: flex;
  gap: var(--fluent-space-md);
  align-items: center;
  min-height: var(--fluent-toolbar-height);
  padding-inline: var(--fluent-space-lg);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
}

.nav a {
  color: var(--fluent-color-text-secondary);
  text-decoration: none;
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-radius: var(--fluent-radius-sm);
}

.nav a.router-link-active {
  color: var(--fluent-color-text-primary);
  background: var(--fluent-color-fill-subtle);
  font-weight: var(--fluent-font-weight-semibold);
}

.content {
  flex: 1;
  padding: var(--fluent-space-lg);
  overflow: auto;
}
</style>
