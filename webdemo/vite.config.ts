import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL as _URL } from "node:url";

// root = the a-file-manager repo (parent of webdemo/). Mocks live in webdemo/mocks.
const mock = (f: string) => fileURLToPath(new _URL("./mocks/" + f, import.meta.url));

export default defineConfig({
  root: fileURLToPath(new _URL("..", import.meta.url)),
  // Demo is hosted under /demo/ of the published landing site, so all built
  // asset URLs are emitted as /demo/assets/... (absolute) and resolve correctly.
  base: process.env.DEMO_BASE || "/demo/",
  plugins: [vue()],
  resolve: {
    alias: {
      "@tauri-apps/api/core": mock("core.ts"),
      "@tauri-apps/api/event": mock("event.ts"),
      "@tauri-apps/api/window": mock("window.ts"),
      "@tauri-apps/api/dpi": mock("dpi.ts"),
      "@tauri-apps/api/path": mock("path.ts"),
      "@tauri-apps/api/app": mock("app.ts"),
      "@tauri-apps/plugin-opener": mock("opener.ts"),
      "@tauri-apps/plugin-dialog": mock("dialog.ts"),
      "@crabnebula/tauri-plugin-drag": mock("drag.ts"),
    },
  },
  clearScreen: false,
  build: {
    // Inline ALL view/component CSS into the single entry stylesheet.
    // Reason: under a non-root base the Vite module-preload helper does not
    // inject the per-async-chunk CSS <link> at runtime, so every lazy route
    // (work-groups / projects / extensions / settings …) renders unstyled.
    // With cssCodeSplit:false the entry index-*.css carries all rules and is
    // loaded upfront by index.html — no lazy CSS to miss, no style flash.
    cssCodeSplit: false,
  },
  server: { host: "127.0.0.1", port: Number(process.env.DEMO_PORT || 1420), strictPort: true },
});
