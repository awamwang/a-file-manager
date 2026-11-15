// SPDX-License-Identifier: GPL-3.0-or-later
//! Launch folders/files in preset IDEs/Agents.
//! Auto-discover first; fall back to the Settings path when discovery fails.
//!
//! 7.20: the preset lists and the OS-specific spawn command moved to
//! `platform/`. This module owns only the wire types, the settings override
//! policy and the "auto → configured → none" resolution order.

use crate::platform::{self, LaunchTarget};
use serde::Serialize;
use std::path::Path;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdeAppInfo {
    pub id: String,
    pub name: String,
    /// User toggle from Settings → Open tools (default on).
    pub enabled: bool,
    /// True when a launchable application was resolved (auto or configured).
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

pub fn ide_setting_enabled_key(id: &str) -> String {
    format!("core.openTool.{id}.enabled")
}

pub fn ide_setting_path_key(id: &str) -> String {
    format!("core.openTool.{id}.path")
}

/// Prefer adapter discovery; a configured override is validated by the same
/// adapter (a Windows `.exe` is a file, a macOS `.app` is a directory).
fn resolve_ide(
    id: &str,
    configured_path: &str,
    discovered: &[LaunchTarget],
) -> (Option<LaunchTarget>, &'static str) {
    if let Some(target) = discovered
        .iter()
        .find(|t| t.id == id && t.resolved_path.is_some())
    {
        return (Some(target.clone()), "auto");
    }
    if let Some(target) = platform::configured_launch_target(id, configured_path) {
        return (Some(target), "configured");
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

/// 仅自动发现（不读设置覆盖），供依赖体检使用。
pub fn ide_probe_auto() -> Vec<(String, bool, Option<String>)> {
    platform::ide_targets()
        .into_iter()
        .map(|target| {
            let available = target.resolved_path.is_some();
            (target.id, available, target.resolved_path)
        })
        .collect()
}

#[tauri::command]
pub fn ide_list_apps(app: AppHandle) -> Vec<IdeAppInfo> {
    let discovered = platform::ide_targets();
    discovered
        .iter()
        .map(|preset| {
            let configured = configured_path_for(&app, &preset.id);
            let enabled = enabled_for(&app, &preset.id);
            let (resolved, resolution) = resolve_ide(&preset.id, &configured, &discovered);
            IdeAppInfo {
                id: preset.id.clone(),
                name: preset.name.clone(),
                enabled,
                available: resolved.is_some(),
                resolved_path: resolved.and_then(|t| t.resolved_path),
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
    let discovered = platform::ide_targets();
    let Some(launcher) = resolve_ide(id, &configured, &discovered).0 else {
        return IdeLaunchResult {
            success: false,
            error: Some(format!("IDE not found: {id}")),
        };
    };

    match platform::launch_ide(&launcher, target) {
        Ok(()) => IdeLaunchResult {
            success: true,
            error: None,
        },
        Err(error) => IdeLaunchResult {
            success: false,
            error: Some(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn discovered() -> Vec<LaunchTarget> {
        platform::ide_targets()
    }

    #[test]
    fn lists_preset_ides_including_qoder() {
        let ids: Vec<_> = discovered().into_iter().map(|t| t.id).collect();
        assert_eq!(ids, vec!["vscode", "cursor", "trae", "qoder"]);
        assert_eq!(
            discovered().iter().find(|t| t.id == "qoder").unwrap().name,
            "Qoder CN"
        );
    }

    #[test]
    fn empty_configured_path_never_resolves() {
        let (resolved, resolution) = resolve_ide("___missing_ide___", "", &discovered());
        assert!(resolved.is_none());
        assert_eq!(resolution, "none");
    }

    #[test]
    fn configured_override_is_validated_by_the_adapter() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let file = std::env::temp_dir().join(format!("afm-ide-override-{stamp}"));
        fs::write(&file, b"x").unwrap();
        let (resolved, resolution) =
            resolve_ide("___missing_ide___", file.to_str().unwrap(), &discovered());
        assert_eq!(resolution, "configured");
        assert_eq!(resolved.unwrap().resolved_path.as_deref(), file.to_str());
        let _ = fs::remove_file(&file);
    }

    #[test]
    fn setting_key_helpers() {
        assert_eq!(
            ide_setting_enabled_key("cursor"),
            "core.openTool.cursor.enabled"
        );
        assert_eq!(ide_setting_path_key("vscode"), "core.openTool.vscode.path");
    }
}
