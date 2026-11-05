// SPDX-License-Identifier: GPL-3.0-or-later
//! Launch folders/files in VS Code / Cursor / Trae (Sigma-aligned IDE/Agent trio).

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdeAppInfo {
    pub id: String,
    pub name: String,
    pub available: bool,
    pub resolved_path: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdeLaunchResult {
    pub success: bool,
    pub error: Option<String>,
}

const IDE_DEFS: &[(&str, &str, &[&str])] = &[
    (
        "vscode",
        "Visual Studio Code",
        &[
            "code.cmd",
            "code",
            r"C:\Users\{user}\AppData\Local\Programs\Microsoft VS Code\Code.exe",
            r"C:\Program Files\Microsoft VS Code\Code.exe",
        ],
    ),
    (
        "cursor",
        "Cursor",
        &[
            "cursor.cmd",
            "cursor",
            r"C:\Users\{user}\AppData\Local\Programs\cursor\Cursor.exe",
            r"C:\Program Files\Cursor\Cursor.exe",
        ],
    ),
    (
        "trae",
        "Trae",
        &[
            "trae.cmd",
            "trae",
            r"C:\Users\{user}\AppData\Local\Programs\Trae\Trae.exe",
            r"C:\Program Files\Trae\Trae.exe",
        ],
    ),
];

fn expand_user(path: &str) -> String {
    if let Ok(user) = std::env::var("USERNAME") {
        return path.replace("{user}", &user);
    }
    path.to_string()
}

fn resolve_candidate(candidate: &str) -> Option<PathBuf> {
    let expanded = expand_user(candidate);
    let path = PathBuf::from(&expanded);
    if path.is_file() {
        return Some(path);
    }
    // PATH lookup for bare commands
    if !expanded.contains('\\') && !expanded.contains('/') {
        if let Ok(output) = Command::new("where.exe").arg(&expanded).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = stdout.lines().next() {
                    let found = PathBuf::from(line.trim());
                    if found.is_file() {
                        return Some(found);
                    }
                }
            }
        }
    }
    None
}

fn resolve_ide(id: &str) -> Option<PathBuf> {
    let def = IDE_DEFS.iter().find(|(ide_id, _, _)| *ide_id == id)?;
    for candidate in def.2 {
        if let Some(path) = resolve_candidate(candidate) {
            return Some(path);
        }
    }
    None
}

#[tauri::command]
pub fn ide_list_apps() -> Vec<IdeAppInfo> {
    IDE_DEFS
        .iter()
        .map(|(id, name, _)| {
            let resolved = resolve_ide(id);
            IdeAppInfo {
                id: (*id).to_string(),
                name: (*name).to_string(),
                available: resolved.is_some(),
                resolved_path: resolved.map(|p| p.to_string_lossy().into_owned()),
            }
        })
        .collect()
}

#[tauri::command]
pub fn ide_open_path(ide_id: String, path: String) -> IdeLaunchResult {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return IdeLaunchResult {
            success: false,
            error: Some("Path is empty".to_string()),
        };
    }
    let target = Path::new(trimmed);
    if !target.exists() {
        return IdeLaunchResult {
            success: false,
            error: Some(format!("Path not found: {trimmed}")),
        };
    }
    let Some(exe) = resolve_ide(ide_id.trim()) else {
        return IdeLaunchResult {
            success: false,
            error: Some(format!("IDE not found: {ide_id}")),
        };
    };

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        match Command::new(&exe)
            .arg(trimmed)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
        {
            Ok(_) => IdeLaunchResult {
                success: true,
                error: None,
            },
            Err(err) => IdeLaunchResult {
                success: false,
                error: Some(format!("Failed to launch IDE: {err}")),
            },
        }
    }
    #[cfg(not(windows))]
    {
        match Command::new(&exe).arg(trimmed).spawn() {
            Ok(_) => IdeLaunchResult {
                success: true,
                error: None,
            },
            Err(err) => IdeLaunchResult {
                success: false,
                error: Some(format!("Failed to launch IDE: {err}")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_three_ides() {
        let apps = ide_list_apps();
        assert_eq!(apps.len(), 3);
        assert_eq!(apps[0].id, "vscode");
        assert_eq!(apps[1].id, "cursor");
        assert_eq!(apps[2].id, "trae");
    }

    #[test]
    fn empty_path_fails() {
        let result = ide_open_path("vscode".into(), String::new());
        assert!(!result.success);
    }
}
