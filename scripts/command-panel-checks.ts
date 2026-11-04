// SPDX-License-Identifier: GPL-3.0-or-later
// Run via: node --experimental-strip-types scripts/command-panel-checks.ts

import assert from "node:assert/strict";
import {
  filterCommands,
  fuzzyScore,
  groupRankedCommands,
} from "../src/commands/fuzzyFilter.ts";
import { buildAppCommands } from "../src/commands/registry.ts";
import { isToggleCommandPanelShortcut } from "../src/state/commandPanel.ts";
import type { AppCommand } from "../src/commands/types.ts";

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
assert.ok(ids.has("view.work-groups"));
assert.ok(ids.has("action.refresh-health"));

assert.equal(fuzzyScore("set", "Open Settings"), fuzzyScore("set", "Open Settings"));
assert.ok(
  (fuzzyScore("set", "Open Settings") ?? -1) >
    (fuzzyScore("set", "Open Health") ?? -1),
);
assert.equal(fuzzyScore("zzz", "Open Settings"), null);

const filtered = filterCommands(commands, "sett");
assert.ok(filtered.some((r) => r.command.id === "view.settings"));
assert.ok(
  filtered.every(
    (r) =>
      fuzzyScore("sett", r.command.title) !== null ||
      fuzzyScore("sett", [r.command.title, ...(r.command.keywords ?? [])].join(" ")) !==
        null,
  ),
);

const settingsFirst = filterCommands(commands, "settings");
assert.equal(settingsFirst[0]?.command.id, "view.settings");

const sections = groupRankedCommands(filterCommands(commands, ""));
assert.deepEqual(
  sections.map((s) => s.group),
  ["Navigate", "View", "Actions"],
);

await commands.find((c) => c.id === "view.settings")!.run();
assert.deepEqual(pushes, ["/settings"]);

await commands.find((c) => c.id === "action.refresh-health")!.run();
assert.ok(runs.includes("refresh-health"));

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
  isToggleCommandPanelShortcut(
    keyEvent({ ctrlKey: true, shiftKey: true, key: "p" }),
  ),
  true,
);
assert.equal(
  isToggleCommandPanelShortcut(keyEvent({ ctrlKey: true, key: "K" })),
  false,
);
assert.equal(
  isToggleCommandPanelShortcut(
    keyEvent({ ctrlKey: true, shiftKey: true, key: "P", isComposing: true }),
  ),
  false,
);

// Empty query keeps registry order groups intact for UI headers.
const empty = filterCommands(commands as AppCommand[], "   ");
assert.equal(empty.length, commands.length);

console.log(
  `Command Panel checks OK (${commands.length} commands, fuzzy + shortcut + registry).`,
);
