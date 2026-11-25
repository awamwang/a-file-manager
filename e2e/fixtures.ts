// SPDX-License-Identifier: GPL-3.0-or-later
// Shared fixture: capture console errors + uncaught page errors per test.
// Each Playwright test gets a fresh browser context → fresh page load → the
// webdemo in-memory vfs (mocks/vfs.ts) reseeds automatically, so destructive
// UI flows (rename/collapse/etc.) are repeatable WITHOUT touching real disk.
import { test as base, expect } from "@playwright/test";

export interface ConsoleLogs {
  errors: string[]; // console.error text
  pageErrors: string[]; // uncaught exceptions (page 'pageerror')
}

export const test = base.extend<{ logs: ConsoleLogs }>({
  logs: async ({ page }, use) => {
    const errors: string[] = [];
    const pageErrors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") errors.push(msg.text());
    });
    page.on("pageerror", (err) => pageErrors.push(String(err && err.message ? err.message : err)));
    await use({ errors, pageErrors });
  },
});

export { expect };

// The webdemo boots the SPA under the /demo/ base. Absolute path resolves
// against the config baseURL origin → http://127.0.0.1:<port>/demo/.
export const DEMO_ROUTE = "/demo/";

// Robust entry: don't wait for the full 'load' event (vite dev cold-transforms
// route chunks on demand and the load event can lag), just require the document
// to be parsed and Vue to have mounted SOMETHING into #app. Per-test
// assertions (toBeVisible) then auto-wait for the specific node.
export async function openDemo(page: import("@playwright/test").Page, route = DEMO_ROUTE) {
  // 'commit' returns as soon as response headers arrive → a fast, SPECIFIC
  // failure if the dev server isn't answering, instead of a black-hole timeout.
  const resp = await page.goto(route, { waitUntil: "commit", timeout: 15_000 });
  if (!resp) throw new Error(`openDemo: no response from ${route} (webServer not answering?)`);
  const status = resp.status();
  if (!resp.ok()) {
    const snippet = (await resp.text().catch(() => "")).slice(0, 300);
    throw new Error(`openDemo: ${route} -> HTTP ${status}\n${snippet}`);
  }
  try {
    await page.waitForSelector("#app :is(*)", { state: "attached", timeout: 30_000 });
  } catch {
    const title = await page.title().catch(() => "?");
    const body = (await page.locator("body").innerText().catch(() => "")).slice(0, 300);
    throw new Error(
      `openDemo: ${route} served (HTTP ${status}, title="${title}") but #app never mounted.\nbody: ${body}`,
    );
  }
}
