// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const places = readFileSync(join(root, "src-tauri/src/navigator/places.rs"), "utf8");
const guide = readFileSync(join(root, "src/components/navigator/NavigatorGuide.vue"), "utf8");
const tree = readFileSync(join(root, "src/components/shell/FolderTree.vue"), "utf8");
const state = readFileSync(join(root, "src/state/navigatorGuide.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));

assert.ok(places.includes("struct ShellPlaces"));
assert.ok(places.includes("onedrive"));
assert.ok(places.includes("list_network_places"));
assert.ok(state.includes("navigator_shell_places"));
assert.ok(guide.includes("shellPlaces.onedrive"));
assert.ok(guide.includes("shellPlaces.libraries"));
assert.ok(guide.includes("shellPlaces.network"));
// AIA-236: sidebar is This PC + drive expand tree — shell places stay on Guide / places menu.
assert.ok(!tree.includes("shellPlaces.onedrive"));
assert.ok(!tree.includes("shellPlaces.libraries"));
assert.ok(!tree.includes("shellPlaces.network"));
assert.ok(tree.includes("drivesList"));
assert.equal(en.shellPlaces.onedrive, "OneDrive");

const rust = spawnSync(
  "cargo",
  [
    "test",
    "--manifest-path",
    join(root, "src-tauri/Cargo.toml"),
    "shell_places_libraries",
    "--",
    "--nocapture",
  ],
  { cwd: root, encoding: "utf8" },
);
if (rust.stdout) process.stdout.write(rust.stdout);
if (rust.stderr) process.stderr.write(rust.stderr);
assert.equal(rust.status, 0);

console.log("Shell places checks OK (AIA-194).");
