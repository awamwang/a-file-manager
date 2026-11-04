// SPDX-License-Identifier: GPL-3.0-or-later

import { createRouter, createWebHistory } from "vue-router";
import HealthView from "../views/HealthView.vue";
import NavigatorView from "../views/NavigatorView.vue";
import SettingsView from "../views/SettingsView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", redirect: "/health" },
    { path: "/health", name: "health", component: HealthView },
    { path: "/navigator", name: "navigator", component: NavigatorView },
    { path: "/settings", name: "settings", component: SettingsView },
  ],
});

export default router;
