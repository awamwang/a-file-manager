// SPDX-License-Identifier: GPL-3.0-or-later

import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import { i18n } from "./i18n";
import "./styles/fluent-tokens.css";
import "./styles/base.css";
import "./styles/path-cards.css";

/**
 * AIA-130: suppress WebView2/Chromium native context menu globally
 * (Back / Reload / Save as / Inspect…). AFM menus still receive the event;
 * they call preventDefault themselves where they open custom UI.
 */
document.addEventListener(
  "contextmenu",
  (event) => {
    event.preventDefault();
  },
  true,
);

createApp(App).use(router).use(i18n).mount("#app");
