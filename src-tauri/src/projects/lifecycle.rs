// SPDX-License-Identifier: GPL-3.0-or-later
//! Workspace delete / rename / Promote WG→Project / Copy (AIA-212 / C6 · ADR 0007 · AIA-273).

use super::board::{read_board, read_context, write_board, write_context, BoardTask, WorkspaceBoard};
use super::index::{
    normalize_path_string, set_active_project as set_index_active, validate_slug,
    workspace_path_for_slug, ProjectsIndex,
};
use super::members::{add_member_junction, list_member_junctions, WorkspaceMember};
use super::ops::{next_id, now_millis, validate_bindable_work_group};
use super::workspace::{
    create_workspace_at, list_workspaces, load_workspace, write_project_meta, CreateWorkspaceResult,
    WorkspaceListItem, ERR_PROJECT_DISPLAY_NAME_EMPTY, ERR_PROJECT_WORKSPACE_NOT_FOUND,
};
use crate::work_groups::sync::{copy_work_group_in_list, unique_copy_name};
use crate::work_groups::types::{UserWorkGroups, WorkGroup};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const ERR_PROJECT_SLUG_CONFLICT: &str = "PROJECT_SLUG_CONFLICT";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RenameWorkspaceResult {
    pub workspace: WorkspaceListItem,
    pub index: ProjectsIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteWorkspaceResult {
    pub deleted_slug: String,
    pub index: ProjectsIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoteWorkspaceResult {
    pub workspace: CreateWorkspaceResult,
    pub members: Vec<WorkspaceMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CopyWorkspaceResult {
    pub workspace: CreateWorkspaceResult,
    pub members: Vec<WorkspaceMember>,
    /// Updated Work Groups document when a bound WG was copied; otherwise None.
    pub work_groups: Option<UserWorkGroups>,
    pub copied_work_group: Option<WorkGroup>,
}

/// Delete the entire Workspace directory. Junction targets are not deleted
/// (`remove_dir_all` unlinks junctions only on Windows).
pub fn delete_workspace(
    projects_root: &Path,
    index: &ProjectsIndex,
    slug: &str,
) -> Result<DeleteWorkspaceResult, String> {
    let item = load_workspace(projects_root, slug)?;
    let dir = PathBuf::from(&item.path);
    if !dir.is_dir() {
        return Err(ERR_PROJECT_WORKSPACE_NOT_FOUND.to_string());
    }

    fs::remove_dir_all(&dir)
        .map_err(|err| format!("Failed to delete Workspace directory: {err}"))?;

    let mut index_next = index.clone();
    if index_next
        .active_project
        .as_ref()
        .is_some_and(|a| a.slug.eq_ignore_ascii_case(&item.project.slug))
    {
        index_next = set_index_active(&index_next, None)?;
    }

    Ok(DeleteWorkspaceResult {
        deleted_slug: item.project.slug,
        index: index_next,
    })
}

/// Rename displayName and/or slug. Slug change renames the Workspace directory;
/// conflict with an existing slug directory is rejected.
pub fn rename_workspace(
    projects_root: &Path,
    index: &ProjectsIndex,
    slug: &str,
    display_name: Option<&str>,
    new_slug: Option<&str>,
) -> Result<RenameWorkspaceResult, String> {
    let item = load_workspace(projects_root, slug)?;
    let mut meta = item.project.clone();
    let old_slug = meta.slug.clone();
    let old_path = PathBuf::from(&item.path);

    if let Some(name) = display_name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(ERR_PROJECT_DISPLAY_NAME_EMPTY.to_string());
        }
        meta.display_name = trimmed.to_string();
    }

    let slug_changed = match new_slug.map(str::trim).filter(|s| !s.is_empty()) {
        None => false,
        Some(candidate) => {
            let next_slug = validate_slug(candidate)?.to_string();
            if next_slug.eq_ignore_ascii_case(&old_slug) {
                meta.slug = next_slug;
                false
            } else {
                let dest = workspace_path_for_slug(projects_root, &next_slug)?;
                if dest.exists() {
                    return Err(ERR_PROJECT_SLUG_CONFLICT.to_string());
                }
                meta.slug = next_slug;
                true
            }
        }
    };

    meta.updated_at = now_millis();

    let final_path = if slug_changed {
        let dest = workspace_path_for_slug(projects_root, &meta.slug)?;
        fs::rename(&old_path, &dest)
            .map_err(|err| format!("Failed to rename Workspace directory: {err}"))?;
        // Update project.json inside the renamed directory.
        write_project_meta(&dest, &meta)?;
        dest
    } else {
        write_project_meta(&old_path, &meta)?;
        old_path
    };

    let mut index_next = index.clone();
    if index_next
        .active_project
        .as_ref()
        .is_some_and(|a| a.slug.eq_ignore_ascii_case(&old_slug))
    {
        index_next = set_index_active(&index_next, Some(&meta.slug))?;
        // Ensure path matches renamed directory.
        if let Some(active) = index_next.active_project.as_mut() {
            active.path = normalize_path_string(&final_path);
        }
    }

    Ok(RenameWorkspaceResult {
        workspace: WorkspaceListItem {
            project: meta,
            path: normalize_path_string(&final_path),
        },
        index: index_next,
    })
}

/// Promote a non-default WG → new Workspace bound to that WG; seed `members/` junctions
/// from WG `memberPaths`. Does not delete or modify the WG itself.
pub fn promote_work_group_to_workspace(
    projects_root: &Path,
    work_groups: &UserWorkGroups,
    work_group_id: &str,
) -> Result<PromoteWorkspaceResult, String> {
    validate_bindable_work_group(work_groups, work_group_id)?;
    let group = work_groups
        .work_groups
        .iter()
        .find(|g| g.id == work_group_id)
        .ok_or_else(|| crate::projects::types::ERR_UNKNOWN_WORK_GROUP.to_string())?;

    let created = create_workspace_at(
        projects_root,
        &group.name,
        Some(&format!("Promoted from Work Group {}", group.name)),
        None,
        Some(group.id.clone()),
    )?;

    let ws_dir = Path::new(&created.path);
    let mut members = Vec::new();
    for path in &group.member_paths {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Skip missing targets rather than failing the whole promote.
        if !Path::new(trimmed).is_dir() {
            continue;
        }
        match add_member_junction(ws_dir, trimmed, None) {
            Ok(member) => members.push(member),
            Err(err) if err == super::members::ERR_MEMBER_EXISTS => continue,
            Err(_) => continue,
        }
    }

    Ok(PromoteWorkspaceResult {
        workspace: created,
        members,
    })
}

fn remap_board_task_ids(board: &WorkspaceBoard) -> WorkspaceBoard {
    WorkspaceBoard {
        schema_version: board.schema_version,
        columns: board.columns.clone(),
        tasks: board
            .tasks
            .iter()
            .map(|task| BoardTask {
                id: next_id("task"),
                title: task.title.clone(),
                body: task.body.clone(),
                status_id: task.status_id.clone(),
                sort_key: task.sort_key,
                created_at: task.created_at,
                updated_at: now_millis(),
                completed_at: task.completed_at,
            })
            .collect(),
    }
}

/// Copy a Workspace (CONTEXT / board / members). When bound to a WG, also copy that WG
/// with a fresh openState (no shared tab ids) and bind the new Project to the copy.
pub fn copy_workspace(
    projects_root: &Path,
    work_groups: &UserWorkGroups,
    slug: &str,
) -> Result<(CopyWorkspaceResult, UserWorkGroups), String> {
    let source = load_workspace(projects_root, slug)?;
    let source_dir = PathBuf::from(&source.path);

    let existing_names: Vec<String> = list_workspaces(projects_root)?
        .into_iter()
        .map(|item| item.project.display_name)
        .collect();
    let display_name = unique_copy_name(&source.project.display_name, &existing_names);

    let mut wgs_next = work_groups.clone();
    let mut copied_wg: Option<WorkGroup> = None;
    let bound_id = source
        .project
        .bound_work_group_id
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let new_bound = if let Some(wg_id) = bound_id {
        // Unknown / default bound ids: skip WG copy and leave unbound rather than fail.
        match copy_work_group_in_list(&wgs_next, &wg_id) {
            Ok((next, created)) => {
                wgs_next = next;
                copied_wg = Some(created.clone());
                Some(created.id)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    let description = if source.project.description.trim().is_empty() {
        None
    } else {
        Some(source.project.description.as_str())
    };

    let created = create_workspace_at(
        projects_root,
        &display_name,
        description,
        None,
        new_bound,
    )?;

    let dest_dir = Path::new(&created.path);
    let context = read_context(&source_dir).unwrap_or_default();
    write_context(dest_dir, &context)?;

    let board = read_board(&source_dir).unwrap_or_else(|_| WorkspaceBoard::empty());
    write_board(dest_dir, &remap_board_task_ids(&board))?;

    let mut members = Vec::new();
    let source_members = list_member_junctions(&source_dir).unwrap_or_default();
    for member in source_members {
        let target = member.target_path.trim();
        if target.is_empty() || !Path::new(target).is_dir() {
            continue;
        }
        match add_member_junction(dest_dir, target, Some(&member.name)) {
            Ok(m) => members.push(m),
            Err(err) if err == super::members::ERR_MEMBER_EXISTS => {
                if let Ok(m) = add_member_junction(dest_dir, target, None) {
                    members.push(m);
                }
            }
            Err(_) => continue,
        }
    }

    Ok((
        CopyWorkspaceResult {
            workspace: created,
            members,
            work_groups: copied_wg.as_ref().map(|_| wgs_next.clone()),
            copied_work_group: copied_wg,
        },
        wgs_next,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::members::add_member_junction;
    use crate::projects::workspace::create_workspace_at;
    use crate::work_groups::sync::default_document;
    use crate::work_groups::types::{WorkGroup, DEFAULT_WORK_GROUP_ID};

    fn temp_root(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-lifecycle-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn index_for(root: &Path) -> ProjectsIndex {
        ProjectsIndex::empty(normalize_path_string(root))
    }

    #[test]
    fn delete_removes_workspace_keeps_junction_target() {
        let root = temp_root("delete");
        let ws = create_workspace_at(&root, "App", None, Some("app"), None).unwrap();
        let target = root.join("real");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("keep.txt"), "x").unwrap();
        add_member_junction(Path::new(&ws.path), &target.to_string_lossy(), Some("alias"))
            .unwrap();

        let mut index = index_for(&root);
        index = set_index_active(&index, Some("app")).unwrap();
        let result = delete_workspace(&root, &index, "app").unwrap();
        assert_eq!(result.deleted_slug, "app");
        assert!(!Path::new(&ws.path).exists());
        assert!(target.join("keep.txt").is_file());
        assert!(result.index.active_project.is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rename_display_name_only() {
        let root = temp_root("rename-name");
        create_workspace_at(&root, "Old", None, Some("old"), None).unwrap();
        let index = index_for(&root);
        let result =
            rename_workspace(&root, &index, "old", Some("New Name"), None).unwrap();
        assert_eq!(result.workspace.project.display_name, "New Name");
        assert_eq!(result.workspace.project.slug, "old");
        assert!(Path::new(&result.workspace.path).join("project.json").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rename_slug_moves_directory_and_updates_active() {
        let root = temp_root("rename-slug");
        create_workspace_at(&root, "App", None, Some("old-slug"), None).unwrap();
        let mut index = index_for(&root);
        index = set_index_active(&index, Some("old-slug")).unwrap();

        let result =
            rename_workspace(&root, &index, "old-slug", None, Some("new-slug")).unwrap();
        assert_eq!(result.workspace.project.slug, "new-slug");
        assert!(!root.join("old-slug").exists());
        assert!(root.join("new-slug").is_dir());
        assert_eq!(
            result.index.active_project.as_ref().map(|a| a.slug.as_str()),
            Some("new-slug")
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rename_slug_conflict_rejected() {
        let root = temp_root("rename-conflict");
        create_workspace_at(&root, "A", None, Some("a"), None).unwrap();
        create_workspace_at(&root, "B", None, Some("b"), None).unwrap();
        let index = index_for(&root);
        let err = rename_workspace(&root, &index, "a", None, Some("b")).unwrap_err();
        assert_eq!(err, ERR_PROJECT_SLUG_CONFLICT);
        assert!(root.join("a").is_dir());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn promote_seeds_junctions_and_binds_wg() {
        let root = temp_root("promote");
        let m1 = root.join("m1");
        let m2 = root.join("m2");
        fs::create_dir_all(&m1).unwrap();
        fs::create_dir_all(&m2).unwrap();

        let mut wgs = default_document();
        wgs.work_groups.push(WorkGroup {
            id: "work-group-promo".into(),
            name: "Promo Group".into(),
            is_default: false,
            member_paths: vec![
                m1.to_string_lossy().to_string(),
                m2.to_string_lossy().to_string(),
                root.join("missing").to_string_lossy().to_string(),
            ],
            open_state: None,
        });

        let result =
            promote_work_group_to_workspace(&root, &wgs, "work-group-promo").unwrap();
        assert_eq!(result.workspace.project.display_name, "Promo Group");
        assert_eq!(
            result.workspace.project.bound_work_group_id.as_deref(),
            Some("work-group-promo")
        );
        assert_eq!(result.members.len(), 2);
        assert!(Path::new(&result.workspace.path).join("members").is_dir());
        // WG still present / unchanged conceptually (we didn't mutate wgs).
        assert!(wgs.work_groups.iter().any(|g| g.id == "work-group-promo"));
        assert!(wgs
            .work_groups
            .iter()
            .any(|g| g.id == DEFAULT_WORK_GROUP_ID));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn promote_rejects_default_wg() {
        let root = temp_root("promote-default");
        let wgs = default_document();
        let err =
            promote_work_group_to_workspace(&root, &wgs, DEFAULT_WORK_GROUP_ID).unwrap_err();
        assert_eq!(
            err,
            crate::projects::types::ERR_DEFAULT_WORK_GROUP_FORBIDDEN
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn copy_workspace_copies_bound_wg_with_fresh_open_state() {
        use crate::projects::board::{write_board, write_context, BoardTask, WorkspaceBoard};
        use crate::work_groups::types::WorkGroupOpenStateTab;

        let root = temp_root("copy-bound");
        let m1 = root.join("m1");
        fs::create_dir_all(&m1).unwrap();

        let source_tab_id = "tab-shared-collision".to_string();
        let mut wgs = default_document();
        wgs.work_groups.push(WorkGroup {
            id: "work-group-src".into(),
            name: "Session".into(),
            is_default: false,
            member_paths: vec![m1.to_string_lossy().to_string()],
            open_state: Some(crate::work_groups::types::WorkGroupOpenState {
                tab_groups: vec![vec![WorkGroupOpenStateTab {
                    id: source_tab_id.clone(),
                    name: "m1".into(),
                    path: m1.to_string_lossy().to_string(),
                    tab_type: "directory".into(),
                }]],
                current_tab_group_index: 0,
                current_tab_index: 0,
            }),
        });

        let ws = create_workspace_at(
            &root,
            "Alpha",
            Some("notes"),
            Some("alpha"),
            Some("work-group-src".into()),
        )
        .unwrap();
        let ws_dir = Path::new(&ws.path);
        write_context(ws_dir, "# hello").unwrap();
        let board = WorkspaceBoard {
            schema_version: 1,
            columns: vec![
                "backlog".into(),
                "doing".into(),
                "review".into(),
                "done".into(),
            ],
            tasks: vec![BoardTask {
                id: "task-keep".into(),
                title: "Ship".into(),
                body: "".into(),
                status_id: "backlog".into(),
                sort_key: 1,
                created_at: 1,
                updated_at: 1,
                completed_at: None,
            }],
        };
        write_board(ws_dir, &board).unwrap();
        add_member_junction(ws_dir, &m1.to_string_lossy(), Some("m1")).unwrap();

        let (result, wgs_next) = copy_workspace(&root, &wgs, "alpha").unwrap();
        assert_eq!(result.workspace.project.display_name, "Alpha (copy)");
        assert_ne!(result.workspace.project.slug, "alpha");
        let copied_wg = result.copied_work_group.expect("copied WG");
        assert_eq!(copied_wg.name, "Session (copy)");
        assert_ne!(copied_wg.id, "work-group-src");
        assert_eq!(
            result.workspace.project.bound_work_group_id.as_deref(),
            Some(copied_wg.id.as_str())
        );
        let copied_tab_id = copied_wg
            .open_state
            .as_ref()
            .unwrap()
            .tab_groups[0][0]
            .id
            .clone();
        assert_ne!(copied_tab_id, source_tab_id);
        assert!(wgs_next.work_groups.iter().any(|g| g.id == copied_wg.id));
        assert!(wgs_next.work_groups.iter().any(|g| g.id == "work-group-src"));
        assert_eq!(result.members.len(), 1);
        let dest = Path::new(&result.workspace.path);
        assert_eq!(
            fs::read_to_string(dest.join("CONTEXT.md")).unwrap().trim(),
            "# hello"
        );
        let copied_board: WorkspaceBoard =
            serde_json::from_str(&fs::read_to_string(dest.join("board.json")).unwrap()).unwrap();
        assert_eq!(copied_board.tasks.len(), 1);
        assert_eq!(copied_board.tasks[0].title, "Ship");
        assert_ne!(copied_board.tasks[0].id, "task-keep");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn copy_unbound_workspace_skips_wg() {
        let root = temp_root("copy-unbound");
        create_workspace_at(&root, "Solo", None, Some("solo"), None).unwrap();
        let wgs = default_document();
        let (result, wgs_next) = copy_workspace(&root, &wgs, "solo").unwrap();
        assert_eq!(result.workspace.project.display_name, "Solo (copy)");
        assert!(result.workspace.project.bound_work_group_id.is_none());
        assert!(result.copied_work_group.is_none());
        assert!(result.work_groups.is_none());
        assert_eq!(wgs_next.work_groups.len(), wgs.work_groups.len());
        let _ = fs::remove_dir_all(&root);
    }
}
