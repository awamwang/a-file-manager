// SPDX-License-Identifier: GPL-3.0-or-later
//! Core Project Workspace light create (AIA-208 / C2 · ADR 0008).
//! Writes only: `project.json`, `CONTEXT.md`, `members/`, `board.json`.
//! Does **not** write `AGENTS.md` / `skills/` (AI Harness init — A6).

use super::index::{
    ensure_projects_root_dir, normalize_path_string, validate_slug, workspace_path_for_slug,
};
use super::ops::now_millis;
use super::types::TASK_STATUSES;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const WORKSPACE_PROJECT_FILE: &str = "project.json";
pub const WORKSPACE_CONTEXT_FILE: &str = "CONTEXT.md";
pub const WORKSPACE_MEMBERS_DIR: &str = "members";
pub const WORKSPACE_BOARD_FILE: &str = "board.json";

pub const WORKSPACE_PROJECT_SCHEMA_VERSION: u32 = 1;
pub const WORKSPACE_BOARD_SCHEMA_VERSION: u32 = 1;

pub const ERR_PROJECT_WORKSPACE_EXISTS: &str = "PROJECT_WORKSPACE_EXISTS";
pub const ERR_PROJECT_DISPLAY_NAME_EMPTY: &str = "PROJECT_DISPLAY_NAME_EMPTY";

/// `project.json` — Core create metadata (ADR 0007 / 0008).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceProjectMeta {
    pub schema_version: u32,
    pub slug: String,
    pub display_name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_work_group_id: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Minimal empty board written at Core create; task shape refined in C5.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceBoard {
    pub schema_version: u32,
    /// Fixed four columns (backlog / doing / review / done).
    pub columns: Vec<String>,
    pub tasks: Vec<serde_json::Value>,
}

impl WorkspaceBoard {
    pub fn empty() -> Self {
        Self {
            schema_version: WORKSPACE_BOARD_SCHEMA_VERSION,
            columns: TASK_STATUSES.iter().map(|s| (*s).to_string()).collect(),
            tasks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceResult {
    pub project: WorkspaceProjectMeta,
    /// Absolute Workspace directory path.
    pub path: String,
}

/// Derive an ASCII slug candidate from a display name.
pub fn slugify_display_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if matches!(c, ' ' | '_' | '-') {
            if !out.is_empty() && !out.ends_with('-') {
                out.push('-');
            }
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    let mut slug = if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed
    };
    if slug.len() > 64 {
        slug.truncate(64);
        slug = slug.trim_end_matches('-').to_string();
        if slug.is_empty() {
            slug = "project".to_string();
        }
    }
    slug
}

fn allocate_unique_slug(projects_root: &Path, preferred: &str) -> Result<String, String> {
    let base = validate_slug(preferred)?.to_string();
    for n in 0..200u32 {
        let candidate = if n == 0 {
            base.clone()
        } else {
            let suffix = format!("-{n}");
            let max_base = 64usize.saturating_sub(suffix.len());
            let mut stem = base.clone();
            if stem.len() > max_base {
                stem.truncate(max_base);
                stem = stem.trim_end_matches('-').to_string();
            }
            if stem.is_empty() {
                stem = "project".to_string();
            }
            format!("{stem}{suffix}")
        };
        let path = workspace_path_for_slug(projects_root, &candidate)?;
        if !path.exists() {
            return Ok(candidate);
        }
        if n == 0 {
            // First collision: if caller wanted an exact slug, surface conflict only when
            // preferred was explicit — handled by caller. Here we always try suffixes.
            continue;
        }
    }
    Err(ERR_PROJECT_WORKSPACE_EXISTS.to_string())
}

/// Create a Core Workspace under `projects_root` (ADR 0008 light create).
///
/// When `exact_slug` is `true`, `slug_or_name` is treated as the slug and must be free.
/// When `false`, `slug_or_name` is a display-name seed (slugified + unique suffix).
pub fn create_workspace_at(
    projects_root: &Path,
    display_name: &str,
    description: Option<&str>,
    slug_hint: Option<&str>,
    bound_work_group_id: Option<String>,
) -> Result<CreateWorkspaceResult, String> {
    let display_name = display_name.trim();
    if display_name.is_empty() {
        return Err(ERR_PROJECT_DISPLAY_NAME_EMPTY.to_string());
    }

    ensure_projects_root_dir(projects_root)?;

    let slug = match slug_hint.map(str::trim).filter(|s| !s.is_empty()) {
        Some(exact) => {
            let slug = validate_slug(exact)?.to_string();
            let path = workspace_path_for_slug(projects_root, &slug)?;
            if path.exists() {
                return Err(ERR_PROJECT_WORKSPACE_EXISTS.to_string());
            }
            slug
        }
        None => {
            let preferred = slugify_display_name(display_name);
            allocate_unique_slug(projects_root, &preferred)?
        }
    };

    let workspace_dir = workspace_path_for_slug(projects_root, &slug)?;
    fs::create_dir_all(&workspace_dir)
        .map_err(|err| format!("Failed to create Workspace directory: {err}"))?;

    let stamp = now_millis();
    let project = WorkspaceProjectMeta {
        schema_version: WORKSPACE_PROJECT_SCHEMA_VERSION,
        slug: slug.clone(),
        display_name: display_name.to_string(),
        description: description.unwrap_or("").trim().to_string(),
        bound_work_group_id,
        created_at: stamp,
        updated_at: stamp,
    };

    write_light_workspace_files(&workspace_dir, &project)?;

    Ok(CreateWorkspaceResult {
        project,
        path: normalize_path_string(&workspace_dir),
    })
}

fn write_light_workspace_files(
    workspace_dir: &Path,
    project: &WorkspaceProjectMeta,
) -> Result<(), String> {
    let project_path = workspace_dir.join(WORKSPACE_PROJECT_FILE);
    let project_text = serde_json::to_string_pretty(project)
        .map_err(|err| format!("Failed to serialize project.json: {err}"))?;
    fs::write(&project_path, format!("{project_text}\n"))
        .map_err(|err| format!("Failed to write project.json: {err}"))?;

    let context_path = workspace_dir.join(WORKSPACE_CONTEXT_FILE);
    fs::write(&context_path, "")
        .map_err(|err| format!("Failed to write CONTEXT.md: {err}"))?;

    let members_dir = workspace_dir.join(WORKSPACE_MEMBERS_DIR);
    fs::create_dir_all(&members_dir)
        .map_err(|err| format!("Failed to create members/: {err}"))?;

    let board = WorkspaceBoard::empty();
    let board_path = workspace_dir.join(WORKSPACE_BOARD_FILE);
    let board_text = serde_json::to_string_pretty(&board)
        .map_err(|err| format!("Failed to serialize board.json: {err}"))?;
    fs::write(&board_path, format!("{board_text}\n"))
        .map_err(|err| format!("Failed to write board.json: {err}"))?;

    // Guardrail: Core create must not emit AI Harness files.
    debug_assert!(!workspace_dir.join("AGENTS.md").exists());
    debug_assert!(!workspace_dir.join("skills").exists());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn list_workspace_entries(workspace_dir: &Path) -> Result<Vec<String>, String> {
        let mut names = Vec::new();
        let entries = fs::read_dir(workspace_dir)
            .map_err(|err| format!("Failed to read Workspace: {err}"))?;
        for entry in entries {
            let entry = entry.map_err(|err| format!("Failed to read Workspace entry: {err}"))?;
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
        names.sort();
        Ok(names)
    }

    fn temp_root(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-workspace-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn slugify_ascii_and_fallback() {
        assert_eq!(slugify_display_name("My Demo App"), "my-demo-app");
        assert_eq!(slugify_display_name("  Hello__World  "), "hello-world");
        assert_eq!(slugify_display_name("中文项目"), "project");
        assert_eq!(slugify_display_name("---"), "project");
    }

    #[test]
    fn light_create_writes_only_core_files() {
        let root = temp_root("create");
        let result = create_workspace_at(
            &root,
            "Demo App",
            Some("notes"),
            None,
            None,
        )
        .unwrap();

        assert_eq!(result.project.slug, "demo-app");
        assert_eq!(result.project.display_name, "Demo App");
        assert_eq!(result.project.description, "notes");
        assert!(result.project.bound_work_group_id.is_none());

        let dir = PathBuf::from(&result.path);
        assert!(dir.is_dir());

        let entries = list_workspace_entries(&dir).unwrap();
        assert_eq!(
            entries,
            vec![
                "CONTEXT.md".to_string(),
                "board.json".to_string(),
                "members".to_string(),
                "project.json".to_string(),
            ]
        );
        assert!(!dir.join("AGENTS.md").exists());
        assert!(!dir.join("skills").exists());

        let meta: WorkspaceProjectMeta =
            serde_json::from_str(&fs::read_to_string(dir.join("project.json")).unwrap()).unwrap();
        assert_eq!(meta.slug, "demo-app");
        assert_eq!(meta.schema_version, WORKSPACE_PROJECT_SCHEMA_VERSION);

        let context = fs::read_to_string(dir.join("CONTEXT.md")).unwrap();
        assert_eq!(context, "");

        assert!(dir.join("members").is_dir());

        let board: WorkspaceBoard =
            serde_json::from_str(&fs::read_to_string(dir.join("board.json")).unwrap()).unwrap();
        assert_eq!(board.columns, vec!["backlog", "doing", "review", "done"]);
        assert!(board.tasks.is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn exact_slug_conflict_errors() {
        let root = temp_root("conflict");
        create_workspace_at(&root, "One", None, Some("taken"), None).unwrap();
        let err = create_workspace_at(&root, "Two", None, Some("taken"), None).unwrap_err();
        assert_eq!(err, ERR_PROJECT_WORKSPACE_EXISTS);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn auto_slug_suffix_on_collision() {
        let root = temp_root("suffix");
        let a = create_workspace_at(&root, "Demo", None, None, None).unwrap();
        let b = create_workspace_at(&root, "Demo", None, None, None).unwrap();
        assert_eq!(a.project.slug, "demo");
        assert_eq!(b.project.slug, "demo-1");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_display_name_rejected() {
        let root = temp_root("empty");
        let err = create_workspace_at(&root, "   ", None, None, None).unwrap_err();
        assert_eq!(err, ERR_PROJECT_DISPLAY_NAME_EMPTY);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn bound_work_group_persisted_in_project_json() {
        let root = temp_root("bound");
        let result = create_workspace_at(
            &root,
            "Bound",
            None,
            Some("bound"),
            Some("work-group-abc".into()),
        )
        .unwrap();
        assert_eq!(
            result.project.bound_work_group_id.as_deref(),
            Some("work-group-abc")
        );
        let meta: WorkspaceProjectMeta = serde_json::from_str(
            &fs::read_to_string(PathBuf::from(&result.path).join("project.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(meta.bound_work_group_id.as_deref(), Some("work-group-abc"));
        let _ = fs::remove_dir_all(&root);
    }
}
