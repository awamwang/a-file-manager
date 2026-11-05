// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-78: Navigator drops in-page title/status; shell Fluent status bar owns counts.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");

for (const banned of [
  "fluent-page-header",
  "fluent-page-title",
  "navigator.metaConnected",
  "navigator.metaDisconnected",
  "Kernel OS browse",
]) {
  if (view.includes(banned)) {
    console.error(`NavigatorView still contains banned in-page chrome: ${banned}`);
    process.exit(1);
  }
}

if (!view.includes("pane-path")) {
  console.error("NavigatorView missing pane-path (path strip must remain)");
  process.exit(1);
}

if (!shell.includes('class="statusbar"') && !shell.includes("class='statusbar'")) {
  console.error("AppShell missing Fluent statusbar");
  process.exit(1);
}

for (const needle of [
  "browseStatusText",
  "healthStatusText",
  "chrome.navigatorCounts",
  "status-primary",
  "status-trailing",
]) {
  if (!shell.includes(needle)) {
    console.error(`AppShell status bar missing ${needle}`);
    process.exit(1);
  }
}

if (!shell.includes("address-") && !shell.includes("crumb-")) {
  console.error("AppShell missing path/address chrome");
  process.exit(1);
}

console.log("navigator-status-bar: ok");
