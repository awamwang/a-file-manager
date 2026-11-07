// SPDX-License-Identifier: GPL-3.0-or-later
//! Workspace delete / rename / Promote WG→Project (AIA-212 / C6 · ADR 0007).

use super::index::{
    normalize_path_string, set_active_project as set_index_active, validate_slug,
    workspace_path_for_slug, ProjectsIndex,
};
use super::members::{add_member_junction, WorkspaceMember};
use super::ops::{now_millis, validate_bindable_work_group};
use super::workspace::{
    create_workspace_at, load_workspace, write_project_meta, CreateWorkspaceResult,
    WorkspaceListItem, ERR_PROJECT_DISPLAY_NAME_EMPTY, ERR_PROJECT_WORKSPACE_NOT_FOUND,
};
use crate::work_groups::types::UserWorkGroups;
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
}
