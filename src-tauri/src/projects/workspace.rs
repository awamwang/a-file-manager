// SPDX-License-Identifier: GPL-3.0-or-later
//! Core Project Workspace disk ops (C2 create · C3 list/bind).
//! Light create writes only: `project.json`, `CONTEXT.md`, `members/`, `board.json`.
//! Does **not** write `AGENTS.md` / `skills/` (AI Harness init — A6).

use super::board::WorkspaceBoard;
use super::index::{
    ensure_projects_root_dir, is_windows_invalid_folder_char, is_windows_reserved_folder_name,
    normalize_path_string, set_active_project as set_index_active, validate_slug,
    workspace_path_for_slug, ProjectsIndex, MAX_SLUG_LEN,
};
use super::helpers::{now_millis, validate_bindable_work_group};
use crate::work_groups::sync::silent_switch_work_group;
use crate::work_groups::types::{SilentSwitchResult, UserWorkGroups, WorkGroupOpenState};
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
pub const ERR_PROJECT_WORKSPACE_NOT_FOUND: &str = "PROJECT_WORKSPACE_NOT_FOUND";

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceResult {
    pub project: WorkspaceProjectMeta,
    /// Absolute Workspace directory path.
    pub path: String,
}

/// One scanned Workspace under Projects Root (C3).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceListItem {
    pub project: WorkspaceProjectMeta,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceListResult {
    pub projects_root: String,
    pub workspaces: Vec<WorkspaceListItem>,
    pub active_slug: Option<String>,
}

/// Activate a disk Workspace (C3): thin-index activeProject + optional WG silent-switch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivateWorkspaceResult {
    pub workspace: WorkspaceListItem,
    pub index: ProjectsIndex,
    pub silent_switch: Option<SilentSwitchResult>,
}

/// Derive an on-disk folder name from a display name (Unicode allowed).
/// Sanitizes only Windows-invalid path characters; does not ASCII-slugify.
pub fn slugify_display_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        if is_windows_invalid_folder_char(c) {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
        } else {
            out.push(c);
        }
    }

    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }

    let mut slug = out;
    if slug.is_empty() || slug == "." || slug == ".." {
        slug = "project".to_string();
    }

    if slug.chars().count() > MAX_SLUG_LEN {
        slug = slug.chars().take(MAX_SLUG_LEN).collect();
        while slug.ends_with('.') || slug.ends_with(' ') {
            slug.pop();
        }
        if slug.is_empty() || slug == "." || slug == ".." {
            slug = "project".to_string();
        }
    }

    if is_windows_reserved_folder_name(&slug) {
        slug.push('_');
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
            let max_base = MAX_SLUG_LEN.saturating_sub(suffix.chars().count());
            let mut stem: String = base.chars().take(max_base).collect();
            while stem.ends_with('.') || stem.ends_with(' ') {
                stem.pop();
            }
            if stem.is_empty() || is_windows_reserved_folder_name(&stem) {
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
/// When `slug_hint` is `Some`, it is the exact folder name/slug and must be free.
/// When `None`, the folder name is derived from `display_name` (Windows-sanitized) with a unique suffix on collision.
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

pub(crate) fn write_project_meta(
    workspace_dir: &Path,
    project: &WorkspaceProjectMeta,
) -> Result<(), String> {
    let project_path = workspace_dir.join(WORKSPACE_PROJECT_FILE);
    let project_text = serde_json::to_string_pretty(project)
        .map_err(|err| format!("Failed to serialize project.json: {err}"))?;
    fs::write(&project_path, format!("{project_text}\n"))
        .map_err(|err| format!("Failed to write project.json: {err}"))?;
    Ok(())
}

fn write_light_workspace_files(
    workspace_dir: &Path,
    project: &WorkspaceProjectMeta,
) -> Result<(), String> {
    write_project_meta(workspace_dir, project)?;

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

/// Load `project.json` from a Workspace directory.
pub fn load_project_meta(workspace_dir: &Path) -> Result<WorkspaceProjectMeta, String> {
    let path = workspace_dir.join(WORKSPACE_PROJECT_FILE);
    if !path.is_file() {
        return Err(ERR_PROJECT_WORKSPACE_NOT_FOUND.to_string());
    }
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read project.json: {err}"))?;
    serde_json::from_str(&text).map_err(|err| format!("Failed to parse project.json: {err}"))
}

/// Resolve Workspace dir for slug under Root and load meta.
pub fn load_workspace(
    projects_root: &Path,
    slug: &str,
) -> Result<WorkspaceListItem, String> {
    let slug = validate_slug(slug)?;
    let dir = workspace_path_for_slug(projects_root, slug)?;
    if !dir.is_dir() {
        return Err(ERR_PROJECT_WORKSPACE_NOT_FOUND.to_string());
    }
    let mut project = load_project_meta(&dir)?;
    // Directory name is the authority for slug on disk.
    project.slug = slug.to_string();
    Ok(WorkspaceListItem {
        project,
        path: normalize_path_string(&dir),
    })
}

/// Scan Projects Root for directories containing `project.json` (C3).
pub fn list_workspaces(projects_root: &Path) -> Result<Vec<WorkspaceListItem>, String> {
    ensure_projects_root_dir(projects_root)?;
    if !projects_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    let entries = fs::read_dir(projects_root)
        .map_err(|err| format!("Failed to scan Projects Root: {err}"))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("Failed to read Projects Root entry: {err}"))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if validate_slug(name).is_err() {
            continue;
        }
        if !path.join(WORKSPACE_PROJECT_FILE).is_file() {
            continue;
        }
        match load_project_meta(&path) {
            Ok(mut project) => {
                project.slug = name.to_string();
                items.push(WorkspaceListItem {
                    project,
                    path: normalize_path_string(&path),
                });
            }
            Err(_) => continue,
        }
    }

    items.sort_by(|a, b| {
        a.project
            .display_name
            .to_ascii_lowercase()
            .cmp(&b.project.display_name.to_ascii_lowercase())
            .then_with(|| a.project.slug.cmp(&b.project.slug))
    });
    Ok(items)
}

/// 按 thin-index `workspaceOrder` 重排列表；未收录的 slug 保持相对顺序排在末尾。
pub fn apply_workspace_order(
    mut items: Vec<WorkspaceListItem>,
    order: &[String],
) -> Vec<WorkspaceListItem> {
    if order.is_empty() || items.len() < 2 {
        return items;
    }
    let rank: std::collections::HashMap<String, usize> = order
        .iter()
        .enumerate()
        .map(|(i, slug)| (slug.to_ascii_lowercase(), i))
        .collect();
    items.sort_by(|a, b| {
        let ra = rank.get(&a.project.slug.to_ascii_lowercase());
        let rb = rank.get(&b.project.slug.to_ascii_lowercase());
        match (ra, rb) {
            (Some(i), Some(j)) => i.cmp(j),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a
                .project
                .display_name
                .to_ascii_lowercase()
                .cmp(&b.project.display_name.to_ascii_lowercase())
                .then_with(|| a.project.slug.cmp(&b.project.slug)),
        }
    });
    items
}

/// Bind or unbind a non-default Work Group on a Workspace `project.json` (C3).
pub fn bind_workspace_work_group(
    projects_root: &Path,
    work_groups: &UserWorkGroups,
    slug: &str,
    work_group_id: Option<String>,
) -> Result<WorkspaceListItem, String> {
    if let Some(ref id) = work_group_id {
        validate_bindable_work_group(work_groups, id)?;
    }
    let mut item = load_workspace(projects_root, slug)?;
    item.project.bound_work_group_id = work_group_id;
    item.project.updated_at = now_millis();
    write_project_meta(Path::new(&item.path), &item.project)?;
    Ok(item)
}

/// Set thin-index active Project and optionally silent-switch the bound WG (C3).
/// Caller persists `index` and `silent_switch.document` to disk.
pub fn activate_workspace(
    projects_root: &Path,
    index: &ProjectsIndex,
    work_groups: &UserWorkGroups,
    slug: &str,
    live_open_state: Option<&WorkGroupOpenState>,
) -> Result<ActivateWorkspaceResult, String> {
    let item = load_workspace(projects_root, slug)?;
    let index_next = set_index_active(index, Some(&item.project.slug))?;

    let silent_switch = match item.project.bound_work_group_id.as_deref() {
        None => None,
        Some(wg_id) => {
            validate_bindable_work_group(work_groups, wg_id)?;
            Some(silent_switch_work_group(
                work_groups,
                wg_id,
                live_open_state,
            )?)
        }
    };

    Ok(ActivateWorkspaceResult {
        workspace: item,
        index: index_next,
        silent_switch,
    })
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
    fn slugify_preserves_display_name_with_windows_sanitize() {
        assert_eq!(slugify_display_name("My Demo App"), "My Demo App");
        assert_eq!(slugify_display_name("  Hello__World  "), "Hello__World");
        assert_eq!(slugify_display_name("中文项目"), "中文项目");
        assert_eq!(slugify_display_name("视频制作"), "视频制作");
        assert_eq!(slugify_display_name("---"), "---");
        assert_eq!(slugify_display_name("a<>b|c"), "a_b_c");
        assert_eq!(slugify_display_name("foo. "), "foo");
        assert_eq!(slugify_display_name("CON"), "CON_");
        assert_eq!(slugify_display_name("<<<"), "project");
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

        assert_eq!(result.project.slug, "Demo App");
        assert_eq!(result.project.display_name, "Demo App");
        assert_eq!(result.project.description, "notes");
        assert!(result.project.bound_work_group_id.is_none());

        let dir = PathBuf::from(&result.path);
        assert!(dir.is_dir());
        assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some("Demo App"));

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
        assert_eq!(meta.slug, "Demo App");
        assert_eq!(meta.schema_version, WORKSPACE_PROJECT_SCHEMA_VERSION);

        let context = fs::read_to_string(dir.join("CONTEXT.md")).unwrap();
        assert_eq!(context, "");

        assert!(dir.join("members").is_dir());

        let board: crate::projects::board::WorkspaceBoard =
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
        assert_eq!(a.project.slug, "Demo");
        assert_eq!(b.project.slug, "Demo-1");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn chinese_display_name_becomes_folder_name() {
        let root = temp_root("chinese");
        let result = create_workspace_at(&root, "视频制作", None, None, None).unwrap();
        assert_eq!(result.project.slug, "视频制作");
        assert_eq!(result.project.display_name, "视频制作");
        assert!(root.join("视频制作").is_dir());
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

    #[test]
    fn list_scans_root_skips_non_workspace_dirs() {
        let root = temp_root("list");
        create_workspace_at(&root, "Beta", None, Some("beta"), None).unwrap();
        create_workspace_at(&root, "Alpha", None, Some("alpha"), None).unwrap();
        fs::create_dir_all(root.join("not-a-project")).unwrap();
        fs::write(root.join("loose.txt"), "x").unwrap();
        // Oversized folder name with project.json is skipped by validate_slug.
        let too_long = "a".repeat(MAX_SLUG_LEN + 1);
        fs::create_dir_all(root.join(&too_long)).unwrap();
        fs::write(
            root.join(&too_long).join(WORKSPACE_PROJECT_FILE),
            "{\n  \"schemaVersion\": 1,\n  \"slug\": \"x\",\n  \"displayName\": \"x\",\n  \"description\": \"\",\n  \"createdAt\": 1,\n  \"updatedAt\": 1\n}\n",
        )
        .unwrap();

        let listed = list_workspaces(&root).unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].project.slug, "alpha");
        assert_eq!(listed[1].project.slug, "beta");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn bind_rejects_default_work_group() {
        use crate::work_groups::sync::default_document;
        use crate::work_groups::types::DEFAULT_WORK_GROUP_ID;
        use super::super::types::ERR_DEFAULT_WORK_GROUP_FORBIDDEN;

        let root = temp_root("bind-default");
        create_workspace_at(&root, "P", None, Some("p1"), None).unwrap();
        let wgs = default_document();
        let err = bind_workspace_work_group(
            &root,
            &wgs,
            "p1",
            Some(DEFAULT_WORK_GROUP_ID.to_string()),
        )
        .unwrap_err();
        assert_eq!(err, ERR_DEFAULT_WORK_GROUP_FORBIDDEN);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn bind_and_unbind_round_trip() {
        use crate::work_groups::sync::default_document;
        use crate::work_groups::types::WorkGroup;

        let root = temp_root("bind-ok");
        create_workspace_at(&root, "P", None, Some("p1"), None).unwrap();

        let mut wgs = default_document();
        wgs.work_groups.push(WorkGroup {
            id: "work-group-x".into(),
            name: "X".into(),
            is_default: false,
            member_paths: vec![r"C:\tmp".into()],
            open_state: None,
        });

        let bound =
            bind_workspace_work_group(&root, &wgs, "p1", Some("work-group-x".into())).unwrap();
        assert_eq!(
            bound.project.bound_work_group_id.as_deref(),
            Some("work-group-x")
        );
        let reloaded = load_workspace(&root, "p1").unwrap();
        assert_eq!(
            reloaded.project.bound_work_group_id.as_deref(),
            Some("work-group-x")
        );

        let unbound = bind_workspace_work_group(&root, &wgs, "p1", None).unwrap();
        assert!(unbound.project.bound_work_group_id.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn load_missing_workspace_errors() {
        let root = temp_root("missing");
        let err = load_workspace(&root, "nope").unwrap_err();
        assert_eq!(err, ERR_PROJECT_WORKSPACE_NOT_FOUND);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn activate_unbound_sets_index_without_switch() {
        use crate::work_groups::sync::default_document;

        let root = temp_root("act-unbound");
        create_workspace_at(&root, "Solo", None, Some("solo"), None).unwrap();
        let index = ProjectsIndex::empty(normalize_path_string(&root));
        let wgs = default_document();
        let result = activate_workspace(&root, &index, &wgs, "solo", None).unwrap();
        assert_eq!(
            result.index.active_project.as_ref().map(|a| a.slug.as_str()),
            Some("solo")
        );
        assert!(result.silent_switch.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn activate_bound_silent_switches_wg() {
        use crate::work_groups::sync::default_document;
        use crate::work_groups::types::{WorkGroup, WorkGroupOpenState, WorkGroupOpenStateTab};

        let root = temp_root("act-bound");
        create_workspace_at(
            &root,
            "Bound",
            None,
            Some("bound"),
            Some("work-group-target".into()),
        )
        .unwrap();

        let mut wgs = default_document();
        let open = WorkGroupOpenState {
            tab_groups: vec![vec![WorkGroupOpenStateTab {
                id: "tab-1".into(),
                name: "tmp".into(),
                path: r"C:\tmp".into(),
                tab_type: "directory".into(),
            }]],
            current_tab_group_index: 0,
            current_tab_index: 0,
        };
        wgs.work_groups.push(WorkGroup {
            id: "work-group-target".into(),
            name: "Target".into(),
            is_default: false,
            member_paths: vec![r"C:\tmp".into()],
            open_state: Some(open.clone()),
        });

        let index = ProjectsIndex::empty(normalize_path_string(&root));
        let result = activate_workspace(&root, &index, &wgs, "bound", None).unwrap();
        let switch = result.silent_switch.expect("switch");
        assert_eq!(switch.document.active_work_group_id, "work-group-target");
        assert!(switch.open_state_to_restore.is_some());
        let _ = fs::remove_dir_all(&root);
    }
}
