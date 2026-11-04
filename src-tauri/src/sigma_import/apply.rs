// SPDX-License-Identifier: GPL-3.0-or-later

use super::map::{build_plan, merge_documents, replace_documents, MappedImport};
use super::types::{ImportMode, ImportPlan};
use crate::projects::store as projects_store;
use crate::projects::types::UserProjects;
use crate::work_groups::store as work_groups_store;
use crate::work_groups::sync::default_document;
use crate::work_groups::types::{UserWorkGroups, DEFAULT_WORK_GROUP_ID};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ApplyRequest {
    pub mode: ImportMode,
    pub dry_run: bool,
    /// When true, refuse replace/merge write if target already has meaningful data
    /// and caller did not pass an explicit mode (CLI handles the gate before call).
    pub allow_destructive: bool,
}

#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub plan: ImportPlan,
    pub wrote: bool,
    pub work_groups_path: String,
    pub projects_path: String,
    pub blocked_reason: Option<String>,
}

pub fn target_has_meaningful_data(config_dir: &Path) -> Result<bool, String> {
    let wg = if work_groups_store::work_groups_file_path(config_dir).exists() {
        work_groups_store::load_from_path(config_dir)?
    } else {
        default_document()
    };
    let projects = if projects_store::projects_file_path(config_dir).exists() {
        projects_store::load_from_path(config_dir)?
    } else {
        crate::projects::migrate::empty_document()
    };
    Ok(work_groups_are_meaningful(&wg) || projects_are_meaningful(&projects))
}

fn work_groups_are_meaningful(doc: &UserWorkGroups) -> bool {
    if doc.work_groups.len() > 1 {
        return true;
    }
    doc.work_groups.iter().any(|g| {
        !g.member_paths.is_empty()
            || g.open_state
                .as_ref()
                .map(|s| !s.tab_groups.is_empty())
                .unwrap_or(false)
            || (g.id != DEFAULT_WORK_GROUP_ID)
    })
}

fn projects_are_meaningful(doc: &UserProjects) -> bool {
    !doc.projects.is_empty()
}

pub fn apply_import(
    config_dir: &Path,
    mapped: &MappedImport,
    request: &ApplyRequest,
) -> Result<ApplyResult, String> {
    let plan = build_plan(mapped, request.mode);
    let wg_path = work_groups_store::work_groups_file_path(config_dir);
    let projects_path = projects_store::projects_file_path(config_dir);

    let meaningful = target_has_meaningful_data(config_dir)?;
    if meaningful && !request.allow_destructive {
        return Ok(ApplyResult {
            plan,
            wrote: false,
            work_groups_path: wg_path.display().to_string(),
            projects_path: projects_path.display().to_string(),
            blocked_reason: Some(
                "Target already has Work Groups / Projects data. Pass --mode replace or --mode merge explicitly (destructive gate)."
                    .into(),
            ),
        });
    }

    if request.dry_run {
        return Ok(ApplyResult {
            plan,
            wrote: false,
            work_groups_path: wg_path.display().to_string(),
            projects_path: projects_path.display().to_string(),
            blocked_reason: None,
        });
    }

    let (wg_doc, projects_doc) = match request.mode {
        ImportMode::Replace => replace_documents(mapped),
        ImportMode::Merge => {
            let existing_wg = if wg_path.exists() {
                work_groups_store::load_from_path(config_dir)?
            } else {
                default_document()
            };
            let existing_projects = if projects_path.exists() {
                projects_store::load_from_path(config_dir)?
            } else {
                crate::projects::migrate::empty_document()
            };
            merge_documents(&existing_wg, &existing_projects, mapped)
        }
    };

    work_groups_store::save_to_path(config_dir, &wg_doc)?;
    projects_store::save_to_path(config_dir, &projects_doc)?;

    Ok(ApplyResult {
        plan,
        wrote: true,
        work_groups_path: wg_path.display().to_string(),
        projects_path: projects_path.display().to_string(),
        blocked_reason: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sigma_import::map::map_sigma_values;
    use serde_json::json;
    use std::fs;

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "afm-sigma-apply-{}-{}",
            label,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn dry_run_does_not_write() {
        let dir = temp_dir("dry");
        let mapped = map_sigma_values(
            json!({
                "workGroups": [{
                    "id": "default-work-group",
                    "name": "Default",
                    "isDefault": true,
                    "memberPaths": ["D:/X"],
                    "openState": null
                }],
                "activeWorkGroupId": "default-work-group"
            }),
            None,
            None,
        )
        .unwrap();
        let result = apply_import(
            &dir,
            &mapped,
            &ApplyRequest {
                mode: ImportMode::Replace,
                dry_run: true,
                allow_destructive: true,
            },
        )
        .unwrap();
        assert!(!result.wrote);
        assert!(!work_groups_store::work_groups_file_path(&dir).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_writes_when_empty_target() {
        let dir = temp_dir("replace");
        let mapped = map_sigma_values(
            json!({
                "workGroups": [{
                    "id": "default-work-group",
                    "name": "Default",
                    "isDefault": true,
                    "memberPaths": ["D:/X"],
                    "openState": {
                        "currentTabGroupIndex": 0,
                        "currentTabIndex": 0,
                        "tabGroups": [[{"id":"t","name":"X","path":"D:/X","type":"directory"}]]
                    }
                }],
                "activeWorkGroupId": "default-work-group"
            }),
            Some(json!({
                "schemaVersion": 2,
                "activeProjectId": "p1",
                "projects": [{
                    "id": "p1",
                    "name": "Demo",
                    "description": "",
                    "contextNotes": "",
                    "createdAt": 1,
                    "updatedAt": 1,
                    "tasks": [],
                    "workspaceLinks": [{
                        "id": "l1",
                        "path": "D:/X",
                        "label": "X",
                        "kind": "folder",
                        "addedAt": 1
                    }]
                }]
            })),
            None,
        )
        .unwrap();
        let result = apply_import(
            &dir,
            &mapped,
            &ApplyRequest {
                mode: ImportMode::Replace,
                dry_run: false,
                allow_destructive: true,
            },
        )
        .unwrap();
        assert!(result.wrote);
        let wg = work_groups_store::load_from_path(&dir).unwrap();
        assert_eq!(wg.work_groups[0].member_paths, vec!["D:/X".to_string()]);
        let projects = projects_store::load_from_path(&dir).unwrap();
        assert_eq!(projects.projects.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn blocks_without_allow_destructive_when_target_meaningful() {
        let dir = temp_dir("block");
        // Seed meaningful AFM data
        let mut wg = default_document();
        wg.work_groups[0].member_paths = vec!["D:/Existing".into()];
        work_groups_store::save_to_path(&dir, &wg).unwrap();

        let mapped = map_sigma_values(
            json!({
                "workGroups": [{
                    "id": "default-work-group",
                    "name": "Default",
                    "isDefault": true,
                    "memberPaths": ["D:/New"],
                    "openState": null
                }],
                "activeWorkGroupId": "default-work-group"
            }),
            None,
            None,
        )
        .unwrap();
        let result = apply_import(
            &dir,
            &mapped,
            &ApplyRequest {
                mode: ImportMode::Replace,
                dry_run: false,
                allow_destructive: false,
            },
        )
        .unwrap();
        assert!(!result.wrote);
        assert!(result.blocked_reason.is_some());
        let loaded = work_groups_store::load_from_path(&dir).unwrap();
        assert_eq!(
            loaded.work_groups[0].member_paths,
            vec!["D:/Existing".to_string()]
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
