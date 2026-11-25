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
