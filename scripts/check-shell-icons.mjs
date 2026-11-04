// SPDX-License-Identifier: GPL-3.0-or-later
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const rust = readFileSync(join(root, "src-tauri/src/shell_icons/mod.rs"), "utf8");
const lib = readFileSync(join(root, "src-tauri/src/lib.rs"), "utf8");
const cargo = readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8");
const view = readFileSync(join(root, "src/views/NavigatorView.vue"), "utf8");
const state = readFileSync(join(root, "src/state/shellIcons.ts"), "utf8");

assert.ok(cargo.includes("file_icon_provider"));
assert.ok(lib.includes("shell_icons::shell_icon_get"));
assert.ok(rust.includes("get_file_icon"));
assert.ok(rust.includes("image_thumbnail_data_url"));
assert.ok(state.includes("shell_icon_get_many"));
assert.ok(view.includes("requestShellIcons"));
assert.ok(view.includes("shell-icon"));

console.log("Shell icons checks OK (AIA-65).");
