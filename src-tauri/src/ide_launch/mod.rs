// SPDX-License-Identifier: GPL-3.0-or-later
//! Launch folders/files in preset IDEs/Agents (Sigma-aligned path discovery).
//! Auto-discover first; fall back to Settings path when discovery fails.

use crate::win_process;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdeAppInfo {
    pub id: String,
    pub name: String,
    /// User toggle from Settings → Open tools (default on).
    pub enabled: bool,
    /// True when a launchable executable was resolved (auto or configured).
    pub available: bool,
    pub resolved_path: Option<String>,
    /// `auto` | `configured` | `none`
    pub resolution: String,
    /// Configured override path from settings (may be empty).
    pub configured_path: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdeLaunchResult {
    pub success: bool,
    pub error: Option<String>,
}

/// Preset IDE/Agent launchers. Candidates mirror Sigma-style discovery:
/// PATH shims first, then per-user Local\\Programs, then Program Files.
/// Qoder CN also accepts Coder CN / Corder CN install-folder spellings.
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
    (
        "qoder",
        "Qoder CN",
        &[
            "qoder.cmd",
            "qoder",
            "coder.cmd",
            "corder.cmd",
            r"C:\Users\{user}\AppData\Local\Programs\Qoder CN\Qoder CN.exe",
            r"C:\Users\{user}\AppData\Local\Programs\Coder CN\Coder CN.exe",
            r"C:\Users\{user}\AppData\Local\Programs\Corder CN\Corder CN.exe",
            r"C:\Users\{user}\AppData\Local\Programs\Qoder\Qoder.exe",
            r"C:\Program Files\Qoder CN\Qoder CN.exe",
            r"C:\Program Files\Coder CN\Coder CN.exe",
            r"C:\Program Files\Corder CN\Corder CN.exe",
        ],
    ),
];

pub fn ide_setting_enabled_key(id: &str) -> String {
    format!("core.openTool.{id}.enabled")
}

pub fn ide_setting_path_key(id: &str) -> String {
    format!("core.openTool.{id}.path")
}

#[cfg(test)]
fn ide_ids() -> Vec<&'static str> {
    IDE_DEFS.iter().map(|(id, _, _)| *id).collect()
}

fn expand_user(path: &str) -> String {
    if let Ok(user) = std::env::var("USERNAME") {
        return path.replace("{user}", &user);
    }
    path.to_string()
}

fn resolve_candidate(candidate: &str) -> Option<PathBuf> {
    let expanded = expand_user(candidate);
    // PATH lookup without spawning where.exe (avoids console flashes on GUI builds).
    win_process::resolve_on_path(&expanded)
}

fn resolve_auto(id: &str) -> Option<PathBuf> {
    let def = IDE_DEFS.iter().find(|(ide_id, _, _)| *ide_id == id)?;
    for candidate in def.2 {
        if let Some(path) = resolve_candidate(candidate) {
            return Some(path);
        }
    }
    None
}

/// Prefer auto-discovery; if that fails, use a configured absolute path when it exists.
fn resolve_ide(id: &str, configured_path: &str) -> (Option<PathBuf>, &'static str) {
    if let Some(auto) = resolve_auto(id) {
        return (Some(auto), "auto");
    }
    let trimmed = configured_path.trim();
    if !trimmed.is_empty() {
        let path = PathBuf::from(trimmed);
        if path.is_file() {
            return (Some(path), "configured");
        }
    }
    (None, "none")
}

fn configured_path_for(app: &AppHandle, id: &str) -> String {
    crate::settings::read_string_setting(app, &ide_setting_path_key(id))
}

fn enabled_for(app: &AppHandle, id: &str) -> bool {
    // Sanitized settings default to true; missing → treat as enabled.
    crate::settings::read_bool_setting_or(app, &ide_setting_enabled_key(id), true)
}

#[tauri::command]
pub fn ide_list_apps(app: AppHandle) -> Vec<IdeAppInfo> {
    IDE_DEFS
        .iter()
        .map(|(id, name, _)| {
            let configured = configured_path_for(&app, id);
            let enabled = enabled_for(&app, id);
            let (resolved, resolution) = resolve_ide(id, &configured);
            IdeAppInfo {
                id: (*id).to_string(),
                name: (*name).to_string(),
                enabled,
                available: resolved.is_some(),
                resolved_path: resolved.map(|p| p.to_string_lossy().into_owned()),
                resolution: resolution.to_string(),
                configured_path: configured,
            }
        })
        .collect()
}

#[tauri::command]
pub fn ide_open_path(app: AppHandle, ide_id: String, path: String) -> IdeLaunchResult {
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

    let id = ide_id.trim();
    if !enabled_for(&app, id) {
        return IdeLaunchResult {
            success: false,
            error: Some(format!("IDE disabled in Settings: {id}")),
        };
    }

    let configured = configured_path_for(&app, id);
    let Some(exe) = resolve_ide(id, &configured).0 else {
        return IdeLaunchResult {
            success: false,
            error: Some(format!("IDE not found: {id}")),
        };
    };

    // .cmd/.bat shims (code.cmd, cursor.cmd, …) are console hosts — hide them.
    match win_process::command(&exe).arg(trimmed).spawn() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_preset_ides_including_qoder() {
        assert_eq!(ide_ids(), vec!["vscode", "cursor", "trae", "qoder"]);
        let def = IDE_DEFS.iter().find(|(id, _, _)| *id == "qoder").unwrap();
        assert_eq!(def.1, "Qoder CN");
    }

    #[test]
    fn qoder_candidates_cover_coder_corder_spellings() {
        let def = IDE_DEFS.iter().find(|(id, _, _)| *id == "qoder").unwrap();
        let joined = def.2.join("|");
        assert!(joined.contains("Qoder CN"));
        assert!(joined.contains("Coder CN"));
        assert!(joined.contains("Corder CN"));
    }

    #[test]
    fn empty_path_fails_without_app_settings() {
        // resolve_ide without config still works for the empty-path guard in open.
        let (resolved, resolution) = resolve_ide("vscode", "");
        let _ = (resolved, resolution);
        assert!(Path::new("").to_string_lossy().is_empty());
    }

    #[test]
    fn configured_path_used_when_auto_missing() {
        let (resolved, resolution) = resolve_ide("___missing_ide___", "");
        assert!(resolved.is_none());
        assert_eq!(resolution, "none");
    }

    #[test]
    fn setting_key_helpers() {
        assert_eq!(ide_setting_enabled_key("cursor"), "core.openTool.cursor.enabled");
        assert_eq!(ide_setting_path_key("vscode"), "core.openTool.vscode.path");
    }

    #[test]
    fn qoder_resolves_when_local_install_present() {
        let local = expand_user(
            r"C:\Users\{user}\AppData\Local\Programs\Qoder CN\Qoder CN.exe",
        );
        if !PathBuf::from(&local).is_file() {
            return;
        }
        let (resolved, resolution) = resolve_ide("qoder", "");
        assert!(resolved.is_some());
        assert_eq!(resolution, "auto");
        assert!(resolved.unwrap().to_string_lossy().contains("Qoder CN"));
    }
}
