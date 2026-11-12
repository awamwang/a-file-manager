// SPDX-License-Identifier: GPL-3.0-or-later

import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import { applyAppLocaleFromValues, i18n } from "./i18n";
import { applySessionSettingsFromValues } from "./state/applySessionSettings";
import { revealMainWindowAfterReady } from "./state/revealMainWindow";
import { hydrateFeaturePackContributions } from "./state/featurePack/contributions";
import { hydrateSettings } from "./state/settingsHydrate";
import { startupMark } from "./state/startupTiming";
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
 * Session refs (showHidden / viewMode / closeBehavior) apply sync here so
 * OpenState bootstrap need not await another settings round-trip.
 */
async function bootstrap(): Promise<void> {
  startupMark("bootstrap:start");
  let values: Record<string, unknown> = {};
  try {
    const doc = await hydrateSettings();
    values = doc.values ?? {};
    startupMark("bootstrap:settings_hydrated");
  } catch {
    // IPC unavailable (vite-only): keep schema defaults via empty values.
  }
  applyUiChromeFromValues(values);
  applyAppLocaleFromValues(values);
  applySessionSettingsFromValues(values);

  try {
    await hydrateFeaturePackContributions();
  } catch {
    // IPC unavailable (vite-only): keep built-in side nav / routes / commands.
  }

  createApp(App).use(router).use(i18n).mount("#app");
  startupMark("bootstrap:mounted");
  // AIA-272: show after first paint (window starts visible:false; no black/maximize flash).
  void revealMainWindowAfterReady();
}

void bootstrap();
