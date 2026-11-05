// SPDX-License-Identifier: GPL-3.0-or-later
/** Guard: GUI release must not spawn where.exe / bare rclone / cmd helpers without CREATE_NO_WINDOW. */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const mainRs = read("src-tauri/src/main.rs");
const winProcess = read("src-tauri/src/win_process.rs");
const terminal = read("src-tauri/src/terminal/mod.rs");
const ide = read("src-tauri/src/ide_launch/mod.rs");
const rclone = read("src-tauri/src/providers/rclone.rs");
const openWith = read("src-tauri/src/open_with/mod.rs");
const menu = read("src/components/navigator/NavigatorContextMenu.vue");
const lib = read("src-tauri/src/lib.rs");

assert.ok(
  /windows_subsystem\s*=\s*"windows"/.test(mainRs),
  "release main.rs must set windows_subsystem = windows",
);
assert.ok(lib.includes("mod win_process"), "lib must register win_process");
assert.ok(
  winProcess.includes("CREATE_NO_WINDOW") && winProcess.includes("resolve_on_path"),
  "win_process must expose CREATE_NO_WINDOW + PATH resolve",
);

assert.ok(
  !/Command::new\(\s*"where\.exe"\s*\)/.test(terminal),
  "terminal must not spawn where.exe",
);
assert.ok(
  terminal.includes("win_process::resolve_on_path"),
  "terminal PATH probe must use win_process",
);

assert.ok(
  !/Command::new\(\s*"where\.exe"\s*\)/.test(ide),
  "ide_launch must not spawn where.exe",
);
assert.ok(
  ide.includes("win_process::resolve_on_path") && ide.includes("win_process::command"),
  "ide_launch must use silent PATH resolve + command",
);

assert.ok(
  rclone.includes("win_process::command"),
  "rclone runner must use CREATE_NO_WINDOW via win_process::command",
);
assert.ok(
  !/Command::new\(\s*&?self\.exe\s*\)/.test(rclone),
  "rclone must not spawn via bare Command::new",
);

assert.ok(
  openWith.includes("win_process::command"),
  "open_with Windows helpers must use win_process::command",
);

assert.ok(
  menu.includes("preloadLaunchersIfNeeded"),
  "context menu must warm launchers via preloadLaunchersIfNeeded",
);
// Every-open re-probe was the "隔一段时间又弹" loop (where.exe storm on each right-click).
assert.ok(
  !/props\.open[\s\S]{0,400}void loadLaunchers\(\)/.test(menu),
  "context menu open must not unconditionally re-invoke loadLaunchers",
);

console.log("No-console-flash checks OK (AIA-177).");
