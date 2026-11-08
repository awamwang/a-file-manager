// SPDX-License-Identifier: GPL-3.0-or-later

import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import { applyAppLocaleFromValues, i18n } from "./i18n";
import { revealMainWindowAfterReady } from "./state/revealMainWindow";
import { hydrateSettings } from "./state/settingsHydrate";
import { applyUiChromeFromValues } from "./state/uiChrome";
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

/**
 * AIA-265: hydrate settings and apply locale/chrome *before* first Vue paint.
 * Otherwise createI18n defaults to `en` and the shell flashes English→zh-CN
 * (or whatever `core.language` is) after App.vue's async onMounted.
 */
async function bootstrap(): Promise<void> {
  let values: Record<string, unknown> = {};
  try {
    const doc = await hydrateSettings();
    values = doc.values ?? {};
  } catch {
    // IPC unavailable (vite-only): keep schema defaults via empty values.
  }
  applyUiChromeFromValues(values);
  applyAppLocaleFromValues(values);

  createApp(App).use(router).use(i18n).mount("#app");
  // AIA-272: show after first paint (window starts visible:false; no black/maximize flash).
  void revealMainWindowAfterReady();
}

void bootstrap();
