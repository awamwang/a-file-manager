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
const chrome = shell.indexOf("navigator-chrome");
const tabstrip = shell.indexOf("tabstrip");

if (headerEnd < 0 || body < 0 || sidenav < 0 || workspace < 0 || chrome < 0) {
  console.error("AppShell missing layout landmarks");
  process.exit(1);
}

if (!(headerEnd < body && body < sidenav && sidenav < workspace && workspace < chrome)) {
  console.error(
    "Expected order: titlebar → body → sidenav → workspace → navigator-chrome",
    { headerEnd, body, sidenav, workspace, chrome },
  );
  process.exit(1);
}

const betweenHeaderBody = shell.slice(headerEnd, body);
if (betweenHeaderBody.includes("navigator-chrome") || betweenHeaderBody.includes("tabstrip")) {
  console.error("Navigator chrome/tabs must not span above the body (full-window)");
  process.exit(1);
}

if (chrome > tabstrip) {
  console.error("tabstrip should follow navigator-chrome");
  process.exit(1);
}

if (!shell.includes(".workspace {")) {
  console.error("missing .workspace flex column styles");
  process.exit(1);
}

if (!shell.includes('v-if="isNavigatorRoute"')) {
  console.error("navigator chrome must remain gated to Navigator route");
  process.exit(1);
}

console.log("shell-sidebar-layout: ok");
