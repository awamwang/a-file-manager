// SPDX-License-Identifier: GPL-3.0-or-later

import { createRouter, createWebHistory } from "vue-router";
import DiscoveryView from "../views/DiscoveryView.vue";
import ExtensionsView from "../views/ExtensionsView.vue";
import NavigatorView from "../views/NavigatorView.vue";
import ProjectsView from "../views/ProjectsView.vue";
import ProvidersView from "../views/ProvidersView.vue";
import SettingsView from "../views/SettingsView.vue";
import WorkGroupsView from "../views/WorkGroupsView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", redirect: "/navigator" },
    // Health is a Settings subsection (AIA-46); keep legacy /health as redirect.
    { path: "/health", redirect: "/settings/health" },
    { path: "/navigator", name: "navigator", component: NavigatorView },
    { path: "/discovery", name: "discovery", component: DiscoveryView },
    { path: "/providers", name: "providers", component: ProvidersView },
    { path: "/work-groups", name: "work-groups", component: WorkGroupsView },
    { path: "/projects", name: "projects", component: ProjectsView },
    { path: "/extensions", name: "extensions", component: ExtensionsView },
    { path: "/settings", name: "settings", component: SettingsView },
    { path: "/settings/health", name: "settings-health", component: SettingsView },
    {
      path: "/settings/shortcuts",
      name: "settings-shortcuts",
      component: SettingsView,
    },
  ],
});

export default router;
