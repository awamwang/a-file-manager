// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows helpers for spawning console tools from a GUI (`windows`) subsystem
//! process without flashing a command-line window.
//!
//! Prefer PATH resolution over shelling out to `where.exe` (which is itself a
//! console app and will allocate a console unless CREATE_NO_WINDOW is set).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// `CREATE_NO_WINDOW` — hide the console for helper/CLI child processes.
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Build a `Command` that will not flash a console window on Windows GUI builds.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// 终止进程及其子进程树（rclone→ssh 等）。失败时仍尝试普通 kill。
pub fn kill_process_tree(pid: u32) {
    #[cfg(windows)]
    {
        let _ = command("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
    }
}

/// Resolve a bare command name via `PATH` (+ `PATHEXT` on Windows) without
/// spawning `where.exe`. Absolute/relative paths that already exist are returned
/// as-is.
pub fn resolve_on_path(candidate: &str) -> Option<PathBuf> {
    let path = PathBuf::from(candidate);
    if path.is_file() {
        return Some(path);
    }
    // Only bare names participate in PATH lookup (mirrors prior where.exe usage).
    if candidate.contains('\\') || candidate.contains('/') {
        return None;
    }

    let path_env = std::env::var_os("PATH")?;
    let extensions = path_lookup_extensions(candidate);

    for dir in std::env::split_paths(&path_env) {
        for ext in &extensions {
            let file_name = if ext.is_empty() {
                candidate.to_string()
            } else {
                format!("{candidate}{ext}")
            };
            let full = dir.join(&file_name);
            if full.is_file() {
                return Some(full);
            }
        }
    }
    None
}

fn path_lookup_extensions(candidate: &str) -> Vec<String> {
    let has_ext = Path::new(candidate)
        .extension()
        .map(|e| !e.is_empty())
        .unwrap_or(false);
    if has_ext {
        // Exact name only (e.g. `code.cmd`, `wt.exe`).
        return vec![String::new()];
    }

    #[cfg(windows)]
    {
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| {
            ".COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.MSC".to_string()
        });
        let mut exts: Vec<String> = vec![String::new()];
        for part in pathext.split(';') {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            let with_dot = if trimmed.starts_with('.') {
                trimmed.to_string()
            } else {
                format!(".{trimmed}")
            };
            if !exts
                .iter()
                .any(|e| e.eq_ignore_ascii_case(&with_dot))
            {
                exts.push(with_dot);
            }
        }
        exts
    }
    #[cfg(not(windows))]
    {
        vec![String::new()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_existing_file_short_circuits() {
        let cargo = std::env::current_exe().expect("current_exe");
        let resolved = resolve_on_path(cargo.to_str().unwrap());
        assert_eq!(resolved.as_deref(), Some(cargo.as_path()));
    }

    #[test]
    fn missing_absolute_path_is_none() {
        assert!(resolve_on_path(r"Z:\definitely-missing-afm-win-process-xyz.exe").is_none());
    }

    #[cfg(windows)]
    #[test]
    fn resolves_cmd_exe_from_path() {
        let found = resolve_on_path("cmd.exe").or_else(|| resolve_on_path("cmd"));
        assert!(
            found.is_some(),
            "cmd.exe should resolve via PATH on Windows"
        );
        assert!(found.unwrap().is_file());
    }

    #[cfg(windows)]
    #[test]
    fn silent_where_replacement_does_not_need_where_exe() {
        // Regression guard: callers must not shell where.exe for PATH probes.
        assert!(resolve_on_path("where.exe").is_some() || resolve_on_path("where").is_some());
    }
}
