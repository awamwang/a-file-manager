// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 7.20 architecture gate (6.5 rule 6): terminal / IDE discovery and spawning
 * are OS-specific behaviour, so they belong in `platform/`, not in the two
 * command modules. The same gate also forbids new Windows-style path rewrites
 * in the modules that produce real filesystem paths.
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

const platformMod = read("src-tauri/src/platform/mod.rs");
const windows = read("src-tauri/src/platform/windows.rs");
const macos = read("src-tauri/src/platform/macos.rs");
const terminal = read("src-tauri/src/terminal/mod.rs");
const ide = read("src-tauri/src/ide_launch/mod.rs");
const pkg = JSON.parse(read("package.json"));

// 1. Neutral contract: one type + five trait methods + five free entry points.
assert.ok(platformMod.includes("pub struct LaunchTarget"), "LaunchTarget missing");
for (const fn of [
  "fn terminal_targets",
  "fn ide_targets",
  "fn configured_launch_target",
  "fn launch_terminal",
  "fn launch_ide",
  "pub fn terminal_targets()",
  "pub fn ide_targets()",
  "pub fn configured_launch_target(",
  "pub fn launch_terminal(",
  "pub fn launch_ide(",
]) {
  assert.ok(platformMod.includes(fn), `platform/mod.rs must expose ${fn}`);
}

// 2. Both adapters implement the whole contract.
for (const [name, src] of [
  ["windows", windows],
  ["macos", macos],
]) {
  for (const fn of [
    "fn terminal_targets",
    "fn ide_targets",
    "fn configured_launch_target",
    "fn launch_terminal",
    "fn launch_ide",
  ]) {
    assert.ok(src.includes(fn), `${name} adapter must implement ${fn}`);
  }
}

// 3. Windows presets were moved verbatim, not simplified away. This replaces
//    the two IDE_DEFS assertions that lived in ide_launch's Rust tests.
for (const needle of [
  '"wt"',
  '"powershell"',
  '"pwsh"',
  '"cmd"',
  '"gitbash"',
  "Windows Terminal",
  "Windows PowerShell",
  "Git Bash",
  "code.cmd",
  "Qoder CN",
  "Coder CN",
  "Corder CN",
  "ShellExecuteExW",
]) {
  assert.ok(windows.includes(needle), `Windows adapter lost preset: ${needle}`);
}

// 4. macOS preset ids must match SettingsView's OPEN_TOOL_IDS, and launching
//    must go through /usr/bin/open (never a Windows .exe path).
for (const id of ["vscode", "cursor", "trae", "qoder"]) {
  assert.ok(macos.includes(`"${id}"`), `macOS adapter missing IDE id ${id}`);
}
for (const id of ["terminal", "iterm", "warp"]) {
  assert.ok(macos.includes(`"${id}"`), `macOS adapter missing terminal id ${id}`);
}
assert.ok(macos.includes("/usr/bin/open"), "macOS adapter must use /usr/bin/open");
assert.ok(macos.includes(".app"), "macOS adapter must discover .app bundles");
assert.ok(
  macos.includes("supports_elevation: false"),
  "macOS must not claim UAC elevation",
);

// 5. Business modules are thin wire adapters: no OS-specific discovery,
//    no environment variables, no Win32 details, no separator rewrites.
const windowsify = 'replace(\'/\', "\\\\")';
for (const [rel, src] of [
  ["src-tauri/src/terminal/mod.rs", terminal],
  ["src-tauri/src/ide_launch/mod.rs", ide],
]) {
  for (const forbidden of [
    "USERNAME",
    "C:\\Program Files",
    "win_process",
    "#[cfg(windows)]",
    windowsify,
  ]) {
    assert.ok(!src.includes(forbidden), `${rel} must not contain ${forbidden} (7.20)`);
  }
}
assert.ok(terminal.includes("platform::terminal_targets"));
assert.ok(terminal.includes("platform::launch_terminal"));
assert.ok(ide.includes("platform::ide_targets"));
assert.ok(ide.includes("platform::launch_ide"));
assert.ok(ide.includes("platform::configured_launch_target"));
assert.ok(ide.includes("core.openTool.{id}.enabled"), "settings keys must not drift");

// 6. Real-path producers delegate to crate::paths — one remaining local
//    rewrite is exactly the bug that produced "Users\\wangmeng".
const realPathConsumers = [
  "src-tauri/src/ai/mcp.rs",
  "src-tauri/src/ai/mod.rs",
  "src-tauri/src/ai/agents.rs",
  "src-tauri/src/ai/harvest.rs",
  "src-tauri/src/ai/harness.rs",
  "src-tauri/src/ai/open_editor.rs",
  "src-tauri/src/visit_stats/mod.rs",
  "src-tauri/src/projects/index.rs",
];
for (const rel of realPathConsumers) {
  const src = read(rel);
  assert.ok(!src.includes(windowsify), `${rel} must not rewrite separators locally (7.20)`);
}
assert.ok(read("src-tauri/src/ai/mcp.rs").includes("crate::paths::join_relative"));
assert.ok(read("src-tauri/src/ai/mod.rs").includes("crate::paths::join_relative"));
assert.ok(
  read("src-tauri/src/projects/index.rs").includes("normalize_path_for_comparison"),
  "active-project root check must use the platform comparison key",
);

// 7. Dependency health text must not keep Windows-only guidance now that the
//    terminal presets come from the active adapter.
const depsProbe = read("src-tauri/src/deps_probe.rs");
const terminalStatus = depsProbe.slice(
  depsProbe.indexOf("fn terminal_status"),
  depsProbe.indexOf("fn ide_status"),
);
assert.ok(terminalStatus.length > 0, "terminal_status must exist");
assert.ok(terminalStatus.includes("a.name.as_str()"), "show adapter display names");
for (const forbidden of ["cmd.exe", "powershell.exe", "Windows Terminal", "Git Bash"]) {
  assert.ok(
    !terminalStatus.includes(forbidden),
    `terminal_status must not hardcode ${forbidden} (7.20)`,
  );
}

// 8. The gate itself stays wired into the full test run.
assert.equal(
  pkg.scripts["test:launch-targets"],
  "node scripts/check-launch-targets.mjs",
  "register test:launch-targets",
);
assert.ok(pkg.scripts.test.includes("test:launch-targets"), "npm test must include the gate");

console.log("check-launch-targets: ok (adapters own launchers + no Windows path rewrites)");
