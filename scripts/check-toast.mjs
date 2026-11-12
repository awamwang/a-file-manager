// SPDX-License-Identifier: GPL-3.0-or-later
// AIA-93: floating toast host + favorites/ops notices out of document flow.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const toast = readFileSync(join(root, "src/state/toast.ts"), "utf8");
const host = readFileSync(join(root, "src/components/shell/ToastHost.vue"), "utf8");
const shell = readFileSync(join(root, "src/components/shell/AppShell.vue"), "utf8");
const nav = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const discovery = readFileSync(join(root, "src/views/DiscoveryView.vue"), "utf8");
const projects = readFileSync(join(root, "src/views/ProjectsView.vue"), "utf8");
const workGroups = readFileSync(join(root, "src/views/WorkGroupsView.vue"), "utf8");
const extensions = readFileSync(join(root, "src/views/ExtensionsView.vue"), "utf8");
const ctx = readFileSync(join(root, "src/components/navigator/NavigatorContextMenu.vue"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(toast.includes("showToast"));
assert.ok(toast.includes("toastSuccess"));
assert.ok(toast.includes("pauseToast"));
assert.ok(toast.includes("resumeToast"));
assert.ok(toast.includes("ERROR_MS"));
assert.ok(host.includes("position: fixed"));
assert.ok(host.includes("toast-host"));
assert.ok(host.includes("bottom:"));
assert.ok(host.includes("right:"));
assert.ok(host.includes("common.dismiss"));
assert.ok(host.includes("pauseToast"));
assert.ok(host.includes("resumeToast"));
assert.ok(host.includes("user-select: text"));
assert.ok(!host.includes(":aria-label=\"'Dismiss'\""));
assert.ok(shell.includes("ToastHost"));
assert.ok(shell.includes('id="afm-toast-host"') || host.includes('id="afm-toast-host"'));
assert.ok(nav.includes("toastSuccess") || nav.includes("toastError"));
assert.ok(!nav.includes('fluent-banner--success">{{ notice }}'));
assert.ok(discovery.includes("toastSuccess"));
assert.ok(discovery.includes("syntaxHint") || discovery.includes("discovery.syntaxHint"));
assert.ok(projects.includes("toastSuccess"));
assert.ok(!projects.includes("const notice = ref"));
assert.ok(workGroups.includes("toastSuccess") || workGroups.includes("toastInfo"));
assert.ok(extensions.includes("toastSuccess"));
assert.ok(!extensions.includes("const notice = ref"));
assert.ok(ctx.includes("favoriteAdded"));
assert.ok(!shell.includes("chrome-wg-banner"));
assert.ok(en.discovery.syntaxHint);
assert.equal(en.common.dismiss, "Dismiss");

console.log("toast checks OK (AIA-93/97).");
