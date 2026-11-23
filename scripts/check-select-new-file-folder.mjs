// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * New file/folder must end up selected (Explorer-style feedback).
 *
 * Surfaces: keyboard shortcuts in NavigatorView and context-menu actions in
 * NavigatorContextMenu. Both create paths must, after the listing refresh,
 * select the path returned by the backend and scroll it into view.
 * Static wiring gate: the selection primitives themselves are shared state
 * (`navigatorSelection.ts` / `revealInNavigator.ts`) already covered elsewhere.
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const menu = readFileSync(
  join(root, "src/components/navigator/NavigatorContextMenu.vue"),
  "utf8",
);

/** Extract a top-level function body by brace matching. */
function functionBody(source, signature) {
  const start = source.indexOf(signature);
  assert.ok(start >= 0, `missing function: ${signature}`);
  let open = source.indexOf("{", start);
  assert.ok(open >= 0, `missing body: ${signature}`);
  let depth = 0;
  for (let i = open; i < source.length; i += 1) {
    if (source[i] === "{") depth += 1;
    else if (source[i] === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(start, i + 1);
    }
  }
  assert.fail(`unterminated body: ${signature}`);
}

const cases = [
  ["view / new folder", view, "async function shortcutNewFolder()", "createFolder("],
  ["view / new file", view, "async function shortcutNewFile()", "createFile("],
  ["menu / new folder", menu, "async function runNewFolder()", "createFolder("],
  ["menu / new file", menu, "async function runNewFile(defaultName?: string)", "createFile("],
];

for (const [label, source, signature, createCall] of cases) {
  const body = functionBody(source, signature);
  assert.ok(body.includes(createCall), `${label}: missing ${createCall}`);
  assert.match(
    body,
    /const createdPath = result\.paths\[0\];/,
    `${label}: created path from the create result is not captured`,
  );
  assert.match(
    body,
    /if \(createdPath\) recordCreateUndo\(createdPath\);/,
    `${label}: undo must record the same path that gets selected`,
  );
  const refreshAt = body.indexOf("await refreshListing();");
  assert.ok(refreshAt >= 0, `${label}: listing is not refreshed after create`);
  const afterRefresh = body.slice(refreshAt);
  assert.match(
    afterRefresh,
    /if \(createdPath\) \{[\s\S]*selectOnly\(createdPath\);[\s\S]*scrollSelectionIntoView\(createdPath\);[\s\S]*\}/,
    `${label}: must select the new item and scroll it into view after refresh`,
  );
}

// The shared selection/scroll primitives must remain available to both surfaces.
assert.match(
  view,
  /import \{[\s\S]*?\bselectOnly\b[\s\S]*?\} from "\.\.\/state\/navigatorSelection(?:\.ts)?"/,
  "NavigatorView lost its selectOnly import",
);
assert.ok(
  view.includes('from "../state/revealInNavigator"'),
  "NavigatorView lost its revealInNavigator import",
);
assert.ok(
  menu.includes('from "../../state/navigatorSelection"'),
  "NavigatorContextMenu lost its navigatorSelection import",
);
assert.ok(
  menu.includes('from "../../state/revealInNavigator"'),
  "NavigatorContextMenu lost its revealInNavigator import",
);

console.log("select-new-file-folder OK (4 create surfaces select the new item).");
