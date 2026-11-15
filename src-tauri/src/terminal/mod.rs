// SPDX-License-Identifier: GPL-3.0-or-later
//! Enumerate local terminals and open a path's directory.
//!
//! 7.20: OS-specific discovery and spawning now live in `platform/`. This
//! module is a thin wire adapter: it maps neutral `LaunchTarget`s to the
//! frontend contract and keeps the directory-resolution rule platform-free.

use crate::platform;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalAppInfo {
    pub id: String,
    pub name: String,
    pub available: bool,
    pub supports_elevation: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenInTerminalResult {
    pub success: bool,
    pub error: Option<String>,
    /// Id of the launcher that succeeded (platform-specific, e.g. `wt` /
    /// `powershell` on Windows, `terminal` / `iterm` on macOS).
    pub launcher: Option<String>,
}

#[tauri::command]
pub fn terminal_list_apps() -> Vec<TerminalAppInfo> {
    platform::terminal_targets()
        .into_iter()
        .map(|target| TerminalAppInfo {
            id: target.id,
            name: target.name,
            available: target.resolved_path.is_some(),
            supports_elevation: target.supports_elevation,
        })
        .collect()
}

#[tauri::command]
pub fn open_in_terminal(
    path: String,
    terminal_id: Option<String>,
    elevated: Option<bool>,
) -> OpenInTerminalResult {
    let dir = match resolve_directory(&path) {
        Ok(dir) => dir,
        Err(error) => {
            return OpenInTerminalResult {
                success: false,
                error: Some(error),
                launcher: None,
            }
        }
    };
    let elev = elevated.unwrap_or(false);
    let requested = terminal_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let mut last_error: Option<String> = None;
    for target in platform::terminal_targets() {
        if target.resolved_path.is_none() {
            continue;
        }
        if requested.is_some_and(|id| id != target.id) {
            continue;
        }
        match platform::launch_terminal(&target, &dir, elev) {
            Ok(()) => {
                return OpenInTerminalResult {
                    success: true,
                    error: None,
                    launcher: Some(target.id),
                }
            }
            Err(error) => last_error = Some(error),
        }
        if requested.is_some() {
            break;
        }
    }

    OpenInTerminalResult {
        success: false,
        error: Some(match requested {
            Some(id) => last_error.unwrap_or_else(|| format!("Terminal not available: {id}")),
            None => last_error
                .unwrap_or_else(|| format!("Could not launch any terminal for {}", dir.display())),
        }),
        launcher: None,
    }
}

fn resolve_directory(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Path is empty".to_string());
    }
    let p = Path::new(trimmed);
    if !p.exists() {
        return Err(format!("Path not found: {trimmed}"));
    }
    if p.is_dir() {
        return Ok(p.to_path_buf());
    }
    p.parent()
        .filter(|parent| !parent.as_os_str().is_empty() && parent.exists())
        .map(|parent| parent.to_path_buf())
        .ok_or_else(|| format!("No parent directory for: {trimmed}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn empty_path_fails() {
        let result = open_in_terminal(String::new(), None, None);
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn missing_path_fails() {
        let result = open_in_terminal(
            "Z:/definitely-missing-a-file-manager-terminal-xyz".to_string(),
            None,
            None,
        );
        assert!(!result.success);
        assert!(result.launcher.is_none());
    }

    #[test]
    fn resolve_directory_for_file_uses_parent() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let dir = std::env::temp_dir().join(format!("afm-term-{stamp}"));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("note.txt");
        fs::write(&file, b"x").unwrap();

        let resolved = resolve_directory(file.to_str().unwrap()).unwrap();
        assert_eq!(resolved, dir);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_apps_maps_the_adapter_targets_without_drift() {
        let apps = terminal_list_apps();
        let targets = platform::terminal_targets();
        assert!(!apps.is_empty());
        assert_eq!(apps.len(), targets.len());
        for (app, target) in apps.iter().zip(&targets) {
            assert_eq!(app.id, target.id);
            assert_eq!(app.name, target.name);
            assert_eq!(app.available, target.resolved_path.is_some());
            assert_eq!(app.supports_elevation, target.supports_elevation);
        }
    }
}
