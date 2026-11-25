// SPDX-License-Identifier: GPL-3.0-or-later
// Playwright E2E for AFM, driven against the webdemo harness (real src/ Vue UI
// running in a plain browser via @tauri-apps mocks + an in-memory vfs).
// See afm-automation-plan.md §3 (L1). This is deliberately kept OUT of the
// `test:` npm-script namespace so `check-test-hygiene` does not force it into
// the aggregate `npm run test` chain before browsers are installed in CI.
import { defineConfig, devices } from "@playwright/test";

const DEMO_PORT = process.env.DEMO_PORT || "1430"; // avoid AFM dev 1420 / CDP 9223
const baseURL = `http://127.0.0.1:${DEMO_PORT}/demo/`;

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  // Fail on accidentally-committed test.only in CI.
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 2 : undefined,
  reporter: process.env.CI ? [["list"], ["github"]] : [["list"], ["html", { open: "never" }]],
  timeout: 60_000,
  use: {
    baseURL,
    navigationTimeout: 60_000,
    // This machine runs Clash/sing-box system proxies. A clean Playwright
    // profile can route even 127.0.0.1 through them and stall on the local dev
    // server. Force direct connections for the localhost harness.
    launchOptions: { args: ["--no-proxy-server"] },
    // ux-audit hard gate is enforced in-spec; keep artifacts on failure only.
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    // Video requires Playwright's bundled ffmpeg binary. Off by default so
    // reusing a system browser (PW_CHANNEL) needs no extra download; opt in via
    // PW_VIDEO=on (CI, where `playwright install` also fetches ffmpeg).
    video: process.env.PW_VIDEO === "on" ? "retain-on-failure" : "off",
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        // Reuse a locally installed browser to SKIP the Chromium download:
        //   $env:PW_CHANNEL='chrome'   → use system Google Chrome
        //   $env:PW_CHANNEL='msedge'   → use system Edge
        // Unset → Playwright's bundled Chromium (CI default; needs `playwright install`).
        channel: process.env.PW_CHANNEL || undefined,
      },
    },
  ],
  webServer: {
    // Run the webdemo build in dev mode so specs always exercise current src/.
    // node-direct invocation (this machine's npm/npx shims are unreliable).
    command: `node node_modules/vite/bin/vite.js --config webdemo/vite.config.ts --host 127.0.0.1 --port ${DEMO_PORT} --strictPort`,
    url: baseURL,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    env: { DEMO_PORT },
  },
});
