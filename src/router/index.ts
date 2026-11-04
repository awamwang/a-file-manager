// SPDX-License-Identifier: GPL-3.0-or-later

import { createRouter, createWebHistory } from "vue-router";
import DiscoveryView from "../views/DiscoveryView.vue";
import HealthView from "../views/HealthView.vue";
import NavigatorView from "../views/NavigatorView.vue";
import ProvidersView from "../views/ProvidersView.vue";
import SettingsView from "../views/SettingsView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", redirect: "/navigator" },
    { path: "/health", name: "health", component: HealthView },
    { path: "/navigator", name: "navigator", component: NavigatorView },
    { path: "/discovery", name: "discovery", component: DiscoveryView },
    { path: "/providers", name: "providers", component: ProvidersView },
    { path: "/settings", name: "settings", component: SettingsView },
  ],
});

export default router;
