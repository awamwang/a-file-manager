// SPDX-License-Identifier: GPL-3.0-or-later
//! Open a directory (or a file's parent) in the system terminal (AIA-56).

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenInTerminalResult {
    pub success: bool,
    pub error: Option<String>,
    /// Which launcher was used: `wt` | `powershell` | `cmd` | none on failure.
    pub launcher: Option<String>,
}

#[tauri::command]
pub fn open_in_terminal(path: String) -> OpenInTerminalResult {
    match resolve_directory(&path) {
        Ok(dir) => spawn_terminal(&dir),
        Err(error) => OpenInTerminalResult {
            success: false,
            error: Some(error),
            launcher: None,
        },
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
        .filter(|parent| parent.as_os_str().len() > 0 && parent.exists())
        .map(|parent| parent.to_path_buf())
        .ok_or_else(|| format!("No parent directory for: {trimmed}"))
}

fn spawn_terminal(dir: &Path) -> OpenInTerminalResult {
    let dir_str = dir.to_string_lossy().replace('/', "\\");

    // Prefer Windows Terminal when available.
    if try_spawn(
        Command::new("wt.exe")
            .args(["-d", &dir_str])
            .current_dir(dir),
    ) {
        return OpenInTerminalResult {
            success: true,
            error: None,
            launcher: Some("wt".to_string()),
        };
    }

    if try_spawn(
        Command::new("powershell.exe")
            .args([
                "-NoExit",
                "-Command",
                &format!("Set-Location -LiteralPath '{}'", dir_str.replace('\'', "''")),
            ])
            .current_dir(dir),
    ) {
        return OpenInTerminalResult {
            success: true,
            error: None,
            launcher: Some("powershell".to_string()),
        };
    }

    if try_spawn(
        Command::new("cmd.exe")
            .args(["/K", &format!("cd /d {dir_str}")])
            .current_dir(dir),
    ) {
        return OpenInTerminalResult {
            success: true,
            error: None,
            launcher: Some("cmd".to_string()),
        };
    }

    OpenInTerminalResult {
        success: false,
        error: Some(format!(
            "Could not launch wt/powershell/cmd for {}",
            dir.display()
        )),
        launcher: None,
    }
}

fn try_spawn(command: &mut Command) -> bool {
    match command.spawn() {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn empty_path_fails() {
        let result = open_in_terminal(String::new());
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn missing_path_fails() {
        let result = open_in_terminal(
            "Z:/definitely-missing-a-file-manager-terminal-xyz".to_string(),
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
}
