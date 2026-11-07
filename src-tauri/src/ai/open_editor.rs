// SPDX-License-Identifier: GPL-3.0-or-later
//! Open a Project Workspace in an IDE/Agent (AIA-221 / A8).
//!
//! Cursor / VS Code / Trae → open Workspace directory.
//! Qoder → open `CONTEXT.md` inside the Workspace (falls back to dir if missing).
//! Reuses `ide_launch::ide_open_path`.

use crate::ide_launch::{ide_open_path, IdeLaunchResult};
use serde::Serialize;
use std::path::PathBuf;
use tauri::AppHandle;

pub const ERR_OPEN_EDITOR_WORKSPACE: &str = "AI_OPEN_EDITOR_WORKSPACE_INVALID";
pub const ERR_OPEN_EDITOR_IDE: &str = "AI_OPEN_EDITOR_IDE_INVALID";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenEditorReport {
    pub ide_id: String,
    pub opened_path: String,
    pub mode: String,
    pub success: bool,
    pub error: Option<String>,
}

fn normalize_ide(raw: &str) -> Result<String, String> {
    let id = raw.trim().to_ascii_lowercase();
    match id.as_str() {
        "cursor" | "vscode" | "trae" | "qoder" => Ok(id),
        "" => Ok("cursor".to_string()),
        _ => Err(ERR_OPEN_EDITOR_IDE.to_string()),
    }
}

/// Resolve which path to hand to the IDE for this Project.
pub fn resolve_open_path(workspace_path: &str, ide_id: &str) -> Result<(PathBuf, String), String> {
    let root = PathBuf::from(workspace_path.trim());
    if !root.is_dir() || !root.join("project.json").is_file() {
        return Err(ERR_OPEN_EDITOR_WORKSPACE.to_string());
    }

    if ide_id == "qoder" {
        let context = root.join("CONTEXT.md");
        if context.is_file() {
            return Ok((context, "context".to_string()));
        }
    }
    Ok((root, "workspace".to_string()))
}

pub fn open_project_in_editor(
    app: &AppHandle,
    workspace_path: &str,
    ide_id: Option<&str>,
) -> Result<OpenEditorReport, String> {
    let ide = normalize_ide(ide_id.unwrap_or("cursor"))?;
    let (path, mode) = resolve_open_path(workspace_path, &ide)?;
    let opened = path.to_string_lossy().replace('/', "\\");
    let launch: IdeLaunchResult = ide_open_path(app.clone(), ide.clone(), opened.clone());
    Ok(OpenEditorReport {
        ide_id: ide,
        opened_path: opened,
        mode,
        success: launch.success,
        error: launch.error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::workspace::create_workspace_at;
    use std::fs;

    #[test]
    fn cursor_opens_workspace_dir() {
        let root = std::env::temp_dir().join(format!("afm-open-ed-{}-{}", "ws", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let created = create_workspace_at(&root, "Demo", None, Some("demo"), None).unwrap();
        let (path, mode) = resolve_open_path(&created.path, "cursor").unwrap();
        assert_eq!(mode, "workspace");
        assert_eq!(path, PathBuf::from(&created.path));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn qoder_opens_context_md_when_present() {
        let root = std::env::temp_dir().join(format!("afm-open-ed-{}-{}", "ctx", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let created = create_workspace_at(&root, "Demo", None, Some("demo2"), None).unwrap();
        let (path, mode) = resolve_open_path(&created.path, "qoder").unwrap();
        assert_eq!(mode, "context");
        assert!(path.ends_with("CONTEXT.md"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_non_workspace() {
        let err = resolve_open_path(r"C:\Windows", "cursor").unwrap_err();
        assert_eq!(err, ERR_OPEN_EDITOR_WORKSPACE);
    }
}
