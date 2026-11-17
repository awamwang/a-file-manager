// SPDX-License-Identifier: GPL-3.0-or-later
//! Open a Project Workspace in any project-capable IDE/Agent (AIA-221 / A8).
//!
//! The tool's behaviour comes from the *shared* open-tool registry
//! (`crate::ide_launch`), not a hardcoded id list: each tool declares whether it
//! supports opening a project and how (`workspace` dir vs `contextFile`). This
//! keeps the file "Open with" list and the Project "Open with" list in sync.
//! Reuses `ide_launch::ide_open_project_path`.

use crate::ide_launch::{self, ProjectOpenMode};
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

/// Resolve which concrete path to hand to the tool for this workspace, given
/// the tool's project mode. `contextFile` tools open `CONTEXT.md` when present,
/// otherwise they fall back to the workspace directory.
pub fn resolve_open_path(
    workspace_path: &str,
    mode: ProjectOpenMode,
) -> Result<(PathBuf, String), String> {
    let root = PathBuf::from(workspace_path.trim());
    if !root.is_dir() || !root.join("project.json").is_file() {
        return Err(ERR_OPEN_EDITOR_WORKSPACE.to_string());
    }

    if mode == ProjectOpenMode::ContextFile {
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
    let id = ide_id.unwrap_or("").trim();
    // `project_capability` is the gate: unknown / disabled / not-installed /
    // file-only tools all resolve to `None`.
    let caps = ide_launch::project_capability(app, id).ok_or_else(|| {
        format!("{}: {id}", ERR_OPEN_EDITOR_IDE)
    })?;
    let (path, mode) = resolve_open_path(workspace_path, caps.project_open_mode)?;
    // This is a real path handed to the launcher; on macOS a Windows-ified
    // `\Users\...` string is not a valid launch target (7.20).
    let opened = crate::paths::to_native_fs_path(&path.to_string_lossy());
    let launch = ide_launch::ide_open_project_path(app, id, &opened);
    Ok(OpenEditorReport {
        ide_id: id.to_string(),
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
    fn workspace_mode_opens_dir() {
        let root =
            std::env::temp_dir().join(format!("afm-open-ed-{}-{}", "ws", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let created = create_workspace_at(&root, "Demo", None, Some("demo"), None).unwrap();
        let (path, mode) = resolve_open_path(&created.path, ProjectOpenMode::Workspace).unwrap();
        assert_eq!(mode, "workspace");
        assert_eq!(path, PathBuf::from(&created.path));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn context_mode_opens_context_md_when_present() {
        let root =
            std::env::temp_dir().join(format!("afm-open-ed-{}-{}", "ctx", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let created = create_workspace_at(&root, "Demo", None, Some("demo2"), None).unwrap();
        // create_workspace_at scaffolds CONTEXT.md inside the workspace dir.
        let (path, mode) = resolve_open_path(&created.path, ProjectOpenMode::ContextFile).unwrap();
        assert_eq!(mode, "context");
        assert!(path.ends_with("CONTEXT.md"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn context_mode_falls_back_to_dir_without_context_md() {
        let root =
            std::env::temp_dir().join(format!("afm-open-ed-{}-{}", "ctx2", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let created = create_workspace_at(&root, "Demo", None, Some("demo3"), None).unwrap();
        // Delete the scaffolded CONTEXT.md to exercise the directory fallback.
        let _ = fs::remove_file(PathBuf::from(created.path.trim()).join("CONTEXT.md"));
        let (_path, mode) = resolve_open_path(&created.path, ProjectOpenMode::ContextFile).unwrap();
        assert_eq!(mode, "workspace");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_non_workspace() {
        let err = resolve_open_path(r"C:\Windows", ProjectOpenMode::Workspace).unwrap_err();
        assert_eq!(err, ERR_OPEN_EDITOR_WORKSPACE);
    }
}
