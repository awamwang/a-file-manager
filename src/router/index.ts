// SPDX-License-Identifier: GPL-3.0-or-later

import { createRouter, createWebHistory, type RouteRecordRaw } from "vue-router";
import { AI_ADDON_ENABLED } from "../config/flavor";
// Default startup page stays eager; other routes are lazy (AIA-182).
import NavigatorView from "../views/NavigatorView.vue";

const routes: RouteRecordRaw[] = [
  { path: "/", redirect: "/navigator" },
  // Health is a Settings subsection (AIA-46); keep legacy /health as redirect.
  { path: "/health", redirect: "/settings/health" },
  { path: "/navigator", name: "navigator", component: NavigatorView },
  {
    path: "/discovery",
    name: "discovery",
    component: () => import("../views/DiscoveryView.vue"),
  },
  {
    path: "/providers",
    name: "providers",
    component: () => import("../views/ProvidersView.vue"),
  },
  {
    path: "/work-groups",
    name: "work-groups",
    component: () => import("../views/WorkGroupsView.vue"),
  },
  {
    path: "/projects",
    name: "projects",
    component: () => import("../views/ProjectsView.vue"),
  },
  {
    path: "/extensions",
    name: "extensions",
    component: () => import("../views/ExtensionsView.vue"),
  },
  {
    path: "/settings",
    name: "settings",
    component: () => import("../views/SettingsView.vue"),
  },
  {
    path: "/settings/health",
    name: "settings-health",
    component: () => import("../views/SettingsView.vue"),
  },
  {
    path: "/settings/shortcuts",
    name: "settings-shortcuts",
    component: () => import("../views/SettingsView.vue"),
  },
  {
    path: "/settings/favorites",
    name: "settings-favorites",
    component: () => import("../views/SettingsView.vue"),
  },
  {
    path: "/settings/open-tools",
    name: "settings-open-tools",
    component: () => import("../views/SettingsView.vue"),
  },
];

if (AI_ADDON_ENABLED) {
  routes.splice(routes.findIndex((r) => r.path === "/projects") + 1, 0, {
    path: "/ai",
    name: "ai",
    component: () => import("../views/AiView.vue"),
  });
}

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
