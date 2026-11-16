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
// 7.20: launcher discovery / spawn moved into the platform adapters.
const windowsAdapter = read("src-tauri/src/platform/windows.rs");
const macosAdapter = read("src-tauri/src/platform/macos.rs");
const platformMod = read("src-tauri/src/platform/mod.rs");
const sidecar = read("src-tauri/src/tool_runtime/sidecar.rs");
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
// 7.23: process-tree termination and the default sidecar interpreter are host
// capabilities, not `#[cfg]` branches in business code.
assert.ok(
  winProcess.includes("crate::platform::kill_process_tree")
    && !/\bkill_process_tree[\s\S]{0,200}taskkill/.test(winProcess),
  "win_process::kill_process_tree must delegate to the platform adapter",
);
assert.ok(
  platformMod.includes("fn kill_process_tree") && platformMod.includes("fn python_command"),
  "process-tree kill / python interpreter must be part of the neutral contract",
);
assert.ok(
  windowsAdapter.includes("fn kill_process_tree") && windowsAdapter.includes("taskkill")
    && macosAdapter.includes("fn kill_process_tree") && macosAdapter.includes("sysinfo"),
  "Windows keeps /T; Unix walks the PPID graph via sysinfo",
);
const pythonFnBody = (source) => {
  const start = source.indexOf("fn python_command(");
  return start < 0 ? "" : source.slice(start, start + 220);
};
assert.ok(
  pythonFnBody(windowsAdapter).includes('"python"')
    && pythonFnBody(macosAdapter).includes('"python3"'),
  "sidecar interpreter must stay python on Windows and python3 on Unix",
);
assert.ok(
  sidecar.includes("crate::platform::python_command()")
    && !sidecar.includes('command: "python".to_string()'),
  "sidecar default contribution must come from the platform contract",
);

assert.ok(
  !/Command::new\(\s*"where\.exe"\s*\)/.test(terminal),
  "terminal must not spawn where.exe",
);
assert.ok(
  !/Command::new\(\s*"where\.exe"\s*\)/.test(windowsAdapter) &&
    windowsAdapter.includes("win_process::resolve_on_path"),
  "Windows launcher PATH probe must use win_process, now in platform/windows.rs (7.20)",
);
assert.ok(
  macosAdapter.includes("win_process::resolve_on_path"),
  "macOS PATH CLI probe must reuse win_process (7.20)",
);

assert.ok(
  !/Command::new\(\s*"where\.exe"\s*\)/.test(ide),
  "ide_launch must not spawn where.exe",
);
assert.ok(
  windowsAdapter.includes("win_process::command"),
  "Windows IDE launch must keep CREATE_NO_WINDOW via win_process::command, now in the adapter",
);
assert.ok(
  ide.includes("platform::launch_ide"),
  "ide_launch must delegate spawning to the platform adapter (7.20)",
);

assert.ok(
  rclone.includes("win_process::command"),
  "rclone runner must use CREATE_NO_WINDOW via win_process::command",
);
assert.ok(
  !/Command::new\(\s*&?self\.exe\s*\)/.test(rclone),
  "rclone must not spawn via bare Command::new",
);
// AIA-182: timeout must kill the child — not advisory-only `output()`.
assert.ok(rclone.includes("child.kill()"), "rclone must kill on timeout");
assert.ok(rclone.includes("try_wait()"), "rclone must poll child instead of blocking output()");
{
  const blockStart = rclone.indexOf("impl RcloneRunner for BlockingRcloneRunner");
  assert.ok(blockStart >= 0, "BlockingRcloneRunner impl must exist");
  const blockEnd = rclone.indexOf("\nimpl ", blockStart + 1);
  const block = rclone.slice(blockStart, blockEnd > blockStart ? blockEnd : rclone.length);
  assert.ok(
    /fn run\(\s*&self,\s*args:\s*&\[&str\],\s*timeout:\s*Duration/.test(block),
    "BlockingRcloneRunner must honor timeout parameter",
  );
  assert.ok(!block.includes(".output()"), "must not use blocking Command::output()");
}

assert.ok(
  openWith.includes("win_process::command"),
  "open_with Windows helpers must use win_process::command",
);

assert.ok(
  menu.includes("preloadLaunchersIfNeeded"),
  "context menu must warm launchers via preloadLaunchersIfNeeded",
);
// AIA-181: warm on first open / submenu — not on Navigator mount.
{
  const mountIdx = menu.indexOf("onMounted(() => {");
  assert.ok(mountIdx >= 0, "NavigatorContextMenu must have onMounted");
  const mountEnd = menu.indexOf("});", mountIdx);
  const mountBody = menu.slice(mountIdx, mountEnd >= 0 ? mountEnd : menu.length);
  assert.ok(
    !mountBody.includes("preloadLaunchersIfNeeded();"),
    "onMounted must not warm launchers (deferred to first open)",
  );
}
// Every-open re-probe was the "隔一段时间又弹" loop (where.exe storm on each right-click).
assert.ok(
  !/props\.open[\s\S]{0,400}void loadLaunchers\(\)/.test(menu),
  "context menu open must not unconditionally re-invoke loadLaunchers",
);

console.log("No-console-flash checks OK (AIA-177).");
