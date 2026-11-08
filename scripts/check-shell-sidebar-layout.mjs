// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-91: sidenav full-height under titlebar; navigator chrome only in right workspace.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");

const headerEnd = shell.indexOf("</header>");
const body = shell.indexOf('class="body"');
const sidenav = shell.indexOf('class="sidenav');
const workspace = shell.indexOf('class="workspace"');
const chromeHost = shell.indexOf("navigator-chrome-host");
const chrome = shell.indexOf('class="navigator-chrome"');
const tabstrip = shell.indexOf('class="tabstrip"');

if (headerEnd < 0 || body < 0 || sidenav < 0 || workspace < 0 || chrome < 0) {
  console.error("AppShell missing layout landmarks");
  process.exit(1);
}

const chromeLandmark = chromeHost >= 0 ? chromeHost : chrome;
if (!(headerEnd < body && body < sidenav && sidenav < workspace && workspace < chromeLandmark)) {
  console.error(
    "Expected order: titlebar → body → sidenav → workspace → navigator chrome",
    { headerEnd, body, sidenav, workspace, chromeLandmark },
  );
  process.exit(1);
}

const betweenHeaderBody = shell.slice(headerEnd, body);
if (
  betweenHeaderBody.includes("navigator-chrome") ||
  betweenHeaderBody.includes('class="tabstrip"')
) {
  console.error("Navigator chrome/tabs must not span above the body (full-window)");
  process.exit(1);
}

// DOM order: tab strip, then address/toolbar chrome (Files-like).
if (tabstrip < 0 || tabstrip > chrome) {
  console.error("tabstrip should precede class=\"navigator-chrome\" toolbar");
  process.exit(1);
}

if (!shell.includes(".workspace {")) {
  console.error("missing .workspace flex column styles");
  process.exit(1);
}

if (
  !shell.includes('v-show="isNavigatorRoute"') &&
  !shell.includes('v-if="isNavigatorRoute"')
) {
  console.error("navigator chrome must remain gated to Navigator route");
  process.exit(1);
}

console.log("shell-sidebar-layout: ok");
