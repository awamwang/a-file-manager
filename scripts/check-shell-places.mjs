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
const state = readFileSync(join(root, "src/state/guide/index.ts"), "utf8");
const en = JSON.parse(readFileSync(join(root, "src/i18n/locales/en.json"), "utf8"));
const winAdapter = readFileSync(
  join(root, "src-tauri/src/platform/windows.rs"),
  "utf8",
);
const listMod = readFileSync(join(root, "src-tauri/src/navigator/list.rs"), "utf8");

assert.ok(places.includes("struct ShellPlaces"));
assert.ok(places.includes("onedrive"));
// ADR 0026: fast mapped list + lazy LAN discovery command, both UNC-keyed.
assert.ok(places.includes("mapped_network_places"));
assert.ok(places.includes("fn network_shares"));
assert.ok(places.includes("discovered_network_places"));
assert.ok(places.includes("fn list_server_shares"));
// ADR 0026: discovery reads the Shell Network folder (WS-Discovery), NOT the
// legacy SMB1 `net view` browser (which hangs on Win10/11). Pin the source so a
// future edit cannot silently revert to `net view` for the computer list.
assert.ok(
  winAdapter.includes("FOLDERID_NetworkFolder") &&
    winAdapter.includes("fn network_computers"),
  "Windows discovery must enumerate the Shell Network folder",
);
assert.ok(
  !winAdapter.includes('args(["/C", "net view"])'),
  "must not use the hanging bare `net view` browser for discovery",
);
// Two-level tree: navigator_list special-cases a bare `\\server` root.
assert.ok(
  listMod.includes("bare_server_unc") && listMod.includes("list_server_shares"),
  "list_directory must expand bare server roots into shares",
);
assert.ok(state.includes("navigator_shell_places"));
assert.ok(state.includes("navigator_network_shares"));
assert.ok(guide.includes("shellPlaces.onedrive"));
assert.ok(guide.includes("shellPlaces.libraries"));
assert.ok(guide.includes("shellPlaces.network"));
// AIA-236 (as amended by ADR 0026): the tree stays drive-root based and keeps
// OneDrive / Libraries on the Guide only, but now owns a top-level Network
// node fed by the lazy discovery command. Its label may reuse the shared
// shellPlaces.network i18n key, so only the *other* shell places stay banned.
assert.ok(!tree.includes("shellPlaces.onedrive"));
assert.ok(!tree.includes("shellPlaces.libraries"));
assert.ok(tree.includes("drivesList"));
assert.ok(tree.includes('data-tree-root="network"'));
assert.ok(tree.includes("loadNetworkShares"));
assert.ok(tree.includes("shellPlaces.network"));
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
