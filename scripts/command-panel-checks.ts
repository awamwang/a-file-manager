// SPDX-License-Identifier: GPL-3.0-or-later
// Run via: node --experimental-strip-types scripts/command-panel-checks.ts

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  filterCommands,
  fuzzyScore,
  groupRankedCommands,
} from "../src/commands/fuzzyFilter.ts";
import { buildAppCommands } from "../src/commands/registry.ts";
import { isToggleCommandPanelShortcut } from "../src/state/commandPanel.ts";
import type { AppCommand } from "../src/commands/types.ts";

const en = JSON.parse(
  readFileSync(
    join(dirname(fileURLToPath(import.meta.url)), "../src/i18n/locales/en.json"),
    "utf8",
  ),
) as Record<string, unknown>;

function t(key: string): string {
  const parts = key.split(".");
  let cur: unknown = en;
  for (const part of parts) {
    if (!cur || typeof cur !== "object") return key;
    cur = (cur as Record<string, unknown>)[part];
  }
  return typeof cur === "string" ? cur : key;
}

function stubRouter(path = "/navigator") {
  return {
    currentRoute: { value: { path } },
    push: async (to: string) => {
      pushes.push(to);
    },
  } as never;
}

const pushes: string[] = [];
const runs: string[] = [];

const commands = buildAppCommands({
  router: stubRouter(),
  ensureNavigatorThenGoUp: async () => {
    runs.push("go-up");
  },
  refreshHealth: () => {
    runs.push("refresh-health");
  },
  reloadWindow: () => {
    runs.push("reload-window");
  },
  t,
});

assert.ok(commands.length >= 8, "expected built-in commands");
assert.deepEqual(
  [...new Set(commands.map((c) => c.group))].sort(),
  ["Actions", "Navigate", "View"],
  "expected Navigate / View / Actions groups",
);

const ids = new Set(commands.map((c) => c.id));
assert.ok(ids.has("nav.go-up"));
assert.ok(ids.has("view.settings"));
assert.ok(ids.has("view.projects"));
assert.ok(ids.has("view.ai"));
assert.ok(ids.has("view.work-groups"));
assert.ok(ids.has("action.refresh-health"));
assert.ok(ids.has("action.reload-window"));
assert.equal(commands.find((c) => c.id === "view.settings")?.title, "Open Settings");
assert.equal(
  commands.find((c) => c.id === "action.reload-window")?.title,
  "Reload Window",
);

assert.equal(fuzzyScore("set", "Open Settings"), fuzzyScore("set", "Open Settings"));
assert.ok(
  (fuzzyScore("set", "Open Settings") ?? -1) >
    (fuzzyScore("set", "Open Health") ?? -1),
);
assert.equal(fuzzyScore("zzz", "Open Settings"), null);

const filtered = filterCommands(commands, "sett");
assert.ok(filtered.some((r) => r.command.id === "view.settings"));

const settingsFirst = filterCommands(commands, "settings");
assert.equal(settingsFirst[0]?.command.id, "view.settings");

const sections = groupRankedCommands(filterCommands(commands, ""));
assert.deepEqual(
  sections.map((s) => s.group),
  ["Navigate", "View", "Actions"],
);

await commands.find((c) => c.id === "view.settings")!.run();
assert.deepEqual(pushes, ["/settings"]);

await commands.find((c) => c.id === "view.health")!.run();
assert.deepEqual(pushes, ["/settings", "/settings/health"]);
assert.equal(
  commands.find((c) => c.id === "view.health")?.title,
  "Open Settings · Health",
);

await commands.find((c) => c.id === "action.refresh-health")!.run();
assert.ok(runs.includes("refresh-health"));

await commands.find((c) => c.id === "action.reload-window")!.run();
assert.ok(runs.includes("reload-window"));

const reloadHits = filterCommands(commands, "reload window");
assert.ok(reloadHits.some((r) => r.command.id === "action.reload-window"));

function keyEvent(partial: Partial<KeyboardEvent>): KeyboardEvent {
  return {
    key: "P",
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    isComposing: false,
    ...partial,
  } as KeyboardEvent;
}

assert.equal(
  isToggleCommandPanelShortcut(
    keyEvent({ ctrlKey: true, shiftKey: true, key: "P" }),
  ),
  true,
);
assert.equal(
  isToggleCommandPanelShortcut(keyEvent({ ctrlKey: true, key: "K" })),
  false,
);

const empty = filterCommands(commands as AppCommand[], "   ");
assert.equal(empty.length, commands.length);

console.log(
  `Command Panel checks OK (${commands.length} commands, fuzzy + shortcut + registry).`,
);
