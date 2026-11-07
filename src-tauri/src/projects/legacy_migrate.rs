// SPDX-License-Identifier: GPL-3.0-or-later
//! One-shot AFM legacy `projects.json` → disk Workspace + thin index (AIA-202).
//!
//! Preserves already-on-disk dogfood data after C7 removed `projects.json` UI authority.
//! Does **not** read live Sigma AppData. Idempotent: skips when `projects.json` is absent.

use super::board::{write_board, write_context, BoardTask, WorkspaceBoard};
use super::index::{
    ensure_projects_root_dir, load_from_path as load_index, normalize_path_string,
    resolve_projects_root, save_to_path as save_index, set_active_project,
};
use super::members::add_member_junction;
use super::migrate::migrate_value;
use super::ops::validate_bindable_work_group;
use super::store::projects_file_path;
use super::types::{ProjectTask, UserProject, UserProjects};
use super::workspace::{
    create_workspace_at, CreateWorkspaceResult, WORKSPACE_BOARD_SCHEMA_VERSION,
};
use crate::work_groups::store as wg_store;
use crate::work_groups::types::UserWorkGroups;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const LEGACY_PROJECTS_BACKUP_NAME: &str = "projects.json.pre-workspace.bak";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LegacyMigrateReport {
    pub skipped: bool,
    pub reason: Option<String>,
    pub migrated_count: usize,
    pub workspace_slugs: Vec<String>,
    pub active_slug: Option<String>,
    pub backup_path: Option<String>,
    pub member_junctions: usize,
    pub member_skipped: usize,
}

fn backup_path(config_dir: &Path) -> PathBuf {
    config_dir.join(LEGACY_PROJECTS_BACKUP_NAME)
}

fn read_legacy_document(config_dir: &Path) -> Result<Option<UserProjects>, String> {
    let path = projects_file_path(config_dir);
    if !path.exists() {
        return Ok(None);
    }
    let text =
        fs::read_to_string(&path).map_err(|err| format!("Failed to read projects.json: {err}"))?;
    let raw: serde_json::Value = serde_json::from_str(&text)
        .map_err(|err| format!("Failed to parse projects.json: {err}"))?;
    Ok(Some(migrate_value(raw)?))
}

fn resolve_bound_wg(
    work_groups: &UserWorkGroups,
    project: &UserProject,
) -> Option<String> {
    let id = project.work_group_id.as_deref()?.trim();
    if id.is_empty() {
        return None;
    }
    match validate_bindable_work_group(work_groups, id) {
        Ok(()) => Some(id.to_string()),
        Err(_) => None,
    }
}

fn board_from_legacy_tasks(tasks: &[ProjectTask]) -> WorkspaceBoard {
    let mut board = WorkspaceBoard {
        schema_version: WORKSPACE_BOARD_SCHEMA_VERSION,
        columns: super::types::TASK_STATUSES
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        tasks: Vec::new(),
    };
    for task in tasks {
        let status = if super::types::TASK_STATUSES.contains(&task.status_id.as_str()) {
            task.status_id.clone()
        } else {
            "backlog".to_string()
        };
        board.tasks.push(BoardTask {
            id: task.id.clone(),
            title: task.title.clone(),
            body: task.body.clone(),
            status_id: status,
            sort_key: task.sort_key,
            created_at: task.created_at,
            updated_at: task.updated_at,
            completed_at: task.completed_at,
        });
    }
    board
}

fn seed_members(
    workspace_dir: &Path,
    project: &UserProject,
) -> (usize, usize) {
    let mut ok = 0usize;
    let mut skipped = 0usize;
    for link in &project.workspace_links {
        let path = link.path.trim();
        if path.is_empty() || !Path::new(path).is_dir() {
            skipped += 1;
            continue;
        }
        let preferred = link.label.trim();
        let named = if preferred.is_empty() {
            None
        } else {
            Some(preferred)
        };
        match add_member_junction(workspace_dir, path, named) {
            Ok(_) => ok += 1,
            Err(_) if named.is_some() => {
                // Label may be invalid as a junction name — retry with basename.
                match add_member_junction(workspace_dir, path, None) {
                    Ok(_) => ok += 1,
                    Err(_) => skipped += 1,
                }
            }
            Err(_) => skipped += 1,
        }
    }
    (ok, skipped)
}

fn migrate_one_project(
    projects_root: &Path,
    work_groups: &UserWorkGroups,
    project: &UserProject,
) -> Result<(CreateWorkspaceResult, usize, usize), String> {
    let bound = resolve_bound_wg(work_groups, project);
    let created = create_workspace_at(
        projects_root,
        &project.name,
        Some(&project.description),
        None,
        bound,
    )?;
    let ws_dir = Path::new(&created.path);

    // Preserve timestamps from legacy project where possible.
    let mut meta = created.project.clone();
    meta.created_at = project.created_at;
    meta.updated_at = project.updated_at;
    super::workspace::write_project_meta(ws_dir, &meta)?;

    write_context(ws_dir, &project.context_notes)?;
    write_board(ws_dir, &board_from_legacy_tasks(&project.tasks))?;
    let (members_ok, members_skipped) = seed_members(ws_dir, project);

    Ok((
        CreateWorkspaceResult {
            project: meta,
            path: created.path,
        },
        members_ok,
        members_skipped,
    ))
}

fn archive_legacy_file(config_dir: &Path) -> Result<PathBuf, String> {
    let src = projects_file_path(config_dir);
    let dest = backup_path(config_dir);
    if dest.exists() {
        let stamped = config_dir.join(format!(
            "{}.{}",
            LEGACY_PROJECTS_BACKUP_NAME,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ));
        fs::rename(&src, &stamped)
            .map_err(|err| format!("Failed to archive projects.json: {err}"))?;
        return Ok(stamped);
    }
    fs::rename(&src, &dest).map_err(|err| format!("Failed to archive projects.json: {err}"))?;
    Ok(dest)
}

/// If APPDATA still has authority-era `projects.json`, materialize Workspaces under
/// Projects Root, set thin-index active, then archive the JSON (not deleted).
pub fn maybe_migrate_legacy_projects(
    config_dir: &Path,
    configured_root: &str,
) -> Result<LegacyMigrateReport, String> {
    let legacy = match read_legacy_document(config_dir)? {
        None => {
            return Ok(LegacyMigrateReport {
                skipped: true,
                reason: Some("no projects.json".into()),
                ..Default::default()
            });
        }
        Some(doc) => doc,
    };

    let resolved = resolve_projects_root(config_dir, configured_root);
    let root_str = ensure_projects_root_dir(&resolved)?;
    let projects_root = PathBuf::from(&root_str);

    if legacy.projects.is_empty() {
        let bak = archive_legacy_file(config_dir)?;
        let _ = load_index(config_dir, configured_root)?;
        return Ok(LegacyMigrateReport {
            skipped: false,
            reason: Some("empty projects.json archived".into()),
            backup_path: Some(normalize_path_string(&bak)),
            ..Default::default()
        });
    }

    let work_groups = wg_store::load_from_path(config_dir).unwrap_or_else(|_| {
        crate::work_groups::sync::default_document()
    });

    let mut report = LegacyMigrateReport {
        skipped: false,
        ..Default::default()
    };
    let mut id_to_slug: Vec<(String, String)> = Vec::new();

    for project in &legacy.projects {
        let (created, ok, skipped) =
            migrate_one_project(&projects_root, &work_groups, project)?;
        report.migrated_count += 1;
        report.workspace_slugs.push(created.project.slug.clone());
        report.member_junctions += ok;
        report.member_skipped += skipped;
        id_to_slug.push((project.id.clone(), created.project.slug));
    }

    let mut index = load_index(config_dir, configured_root)?;
    let active_slug = legacy
        .active_project_id
        .as_ref()
        .and_then(|id| {
            id_to_slug
                .iter()
                .find(|(legacy_id, _)| legacy_id == id)
                .map(|(_, slug)| slug.clone())
        })
        .or_else(|| report.workspace_slugs.first().cloned());

    if let Some(ref slug) = active_slug {
        index = set_active_project(&index, Some(slug))?;
        save_index(config_dir, &index)?;
        report.active_slug = Some(slug.clone());
    } else {
        let _ = index;
    }

    let bak = archive_legacy_file(config_dir)?;
    report.backup_path = Some(normalize_path_string(&bak));
    Ok(report)
}

/// Convenience for tests / live one-shot against an absolute config dir.
#[allow(dead_code)]
pub fn migrate_config_dir(config_dir: &Path) -> Result<LegacyMigrateReport, String> {
    maybe_migrate_legacy_projects(config_dir, "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::index::ProjectsIndex;
    use crate::projects::store::PROJECTS_FILE_NAME;
    use crate::work_groups::types::{WorkGroup, WorkGroupOpenState, DEFAULT_WORK_GROUP_ID};
    use serde_json::json;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-legacy-migrate-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_wg(config: &Path, extra: WorkGroup) {
        let mut doc = crate::work_groups::sync::default_document();
        doc.work_groups.push(extra);
        wg_store::save_to_path(config, &doc).unwrap();
    }

    #[test]
    fn skips_when_no_legacy_file() {
        let dir = temp_dir("skip");
        let report = maybe_migrate_legacy_projects(&dir, "").unwrap();
        assert!(report.skipped);
        assert_eq!(report.reason.as_deref(), Some("no projects.json"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrates_projects_json_to_workspaces_and_archives() {
        let dir = temp_dir("migrate");
        let member_a = dir.join("member-a");
        let member_b = dir.join("member-b");
        fs::create_dir_all(&member_a).unwrap();
        fs::create_dir_all(&member_b).unwrap();

        write_wg(
            &dir,
            WorkGroup {
                id: "work-group-video".into(),
                name: "视频组".into(),
                is_default: false,
                member_paths: vec![normalize_path_string(&member_a)],
                open_state: Some(WorkGroupOpenState {
                    tab_groups: vec![vec![]],
                    current_tab_group_index: 0,
                    current_tab_index: 0,
                }),
            },
        );

        let legacy = json!({
            "schemaVersion": 2,
            "activeProjectId": "project-2",
            "projects": [
                {
                    "id": "project-1",
                    "name": "OpenMontage",
                    "description": "demo",
                    "workspaceLinks": [
                        {
                            "id": "l1",
                            "path": member_a.to_string_lossy(),
                            "label": "OpenMontage",
                            "kind": "folder",
                            "addedAt": 1
                        }
                    ],
                    "tasks": [],
                    "contextNotes": "",
                    "createdAt": 10,
                    "updatedAt": 11
                },
                {
                    "id": "project-2",
                    "name": "视频制作",
                    "description": "",
                    "workGroupId": "work-group-video",
                    "workspaceLinks": [
                        {
                            "id": "l2",
                            "path": member_b.to_string_lossy(),
                            "label": "Videos",
                            "kind": "folder",
                            "addedAt": 2
                        },
                        {
                            "id": "l-missing",
                            "path": dir.join("missing-dir").to_string_lossy(),
                            "label": "gone",
                            "kind": "folder",
                            "addedAt": 3
                        }
                    ],
                    "tasks": [
                        {
                            "id": "task-1",
                            "title": "开发日记",
                            "body": "notes",
                            "statusId": "doing",
                            "sortKey": 0,
                            "workspaceLinkId": null,
                            "outcomeUrl": null,
                            "createdAt": 20,
                            "updatedAt": 21,
                            "completedAt": null
                        }
                    ],
                    "contextNotes": "合集说明\n",
                    "createdAt": 12,
                    "updatedAt": 22
                }
            ]
        });
        fs::write(
            dir.join(PROJECTS_FILE_NAME),
            serde_json::to_string_pretty(&legacy).unwrap(),
        )
        .unwrap();

        let report = maybe_migrate_legacy_projects(&dir, "").unwrap();
        assert!(!report.skipped);
        assert_eq!(report.migrated_count, 2);
        assert_eq!(report.workspace_slugs.len(), 2);
        assert_eq!(report.active_slug.as_deref(), Some("project"));
        assert!(report.member_junctions >= 2);
        assert!(report.member_skipped >= 1);
        assert!(!projects_file_path(&dir).exists());
        assert!(backup_path(&dir).is_file());

        let root = dir.join("projects");
        let openmontage = root.join("openmontage");
        assert!(openmontage.join("project.json").is_file());
        assert!(openmontage.join("CONTEXT.md").is_file());
        assert!(openmontage.join("board.json").is_file());
        assert!(openmontage.join("members").join("OpenMontage").exists());

        let video = root.join("project");
        let meta_text = fs::read_to_string(video.join("project.json")).unwrap();
        assert!(meta_text.contains("视频制作"));
        assert!(meta_text.contains("work-group-video"));
        let context = fs::read_to_string(video.join("CONTEXT.md")).unwrap();
        assert!(context.contains("合集说明"));
        let board_text = fs::read_to_string(video.join("board.json")).unwrap();
        assert!(board_text.contains("开发日记"));
        assert!(board_text.contains("doing"));
        assert!(video.join("members").join("Videos").exists());

        let index: ProjectsIndex = serde_json::from_str(
            &fs::read_to_string(dir.join("projects-index.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            index.active_project.as_ref().map(|a| a.slug.as_str()),
            Some("project")
        );

        // Idempotent second pass.
        let again = maybe_migrate_legacy_projects(&dir, "").unwrap();
        assert!(again.skipped);

        let _ = fs::remove_dir_all(&dir);
    }

    /// Live one-shot against this machine's AFM AppData (dogfood). Run explicitly:
    /// `cargo test --manifest-path src-tauri/Cargo.toml migrate_live_afm_appdata -- --ignored --nocapture`
    #[test]
    #[ignore = "mutates real APPDATA; run intentionally for AIA-202"]
    fn migrate_live_afm_appdata() {
        let config = PathBuf::from(std::env::var("APPDATA").expect("APPDATA"))
            .join("com.a-file-manager.app");
        assert!(
            config.is_dir(),
            "AFM AppData missing at {}",
            config.display()
        );
        let report = migrate_config_dir(&config).expect("live migrate");
        eprintln!("live migrate report: {report:?}");
        assert!(
            report.skipped || report.backup_path.is_some(),
            "expected skip or backup"
        );
        assert!(!projects_file_path(&config).exists());
        let index_path = config.join("projects-index.json");
        assert!(index_path.is_file(), "thin index should exist after migrate");
    }

    #[test]
    fn drops_invalid_default_wg_binding() {
        let dir = temp_dir("default-wg");
        let legacy = json!({
            "schemaVersion": 2,
            "activeProjectId": "p1",
            "projects": [{
                "id": "p1",
                "name": "Bound-default",
                "description": "",
                "workGroupId": DEFAULT_WORK_GROUP_ID,
                "workspaceLinks": [],
                "tasks": [],
                "contextNotes": "",
                "createdAt": 1,
                "updatedAt": 1
            }]
        });
        fs::write(
            dir.join(PROJECTS_FILE_NAME),
            serde_json::to_string_pretty(&legacy).unwrap(),
        )
        .unwrap();
        wg_store::save_to_path(&dir, &crate::work_groups::sync::default_document()).unwrap();

        let report = maybe_migrate_legacy_projects(&dir, "").unwrap();
        assert_eq!(report.migrated_count, 1);
        let meta = fs::read_to_string(dir.join("projects").join("bound-default").join("project.json"))
            .unwrap();
        assert!(!meta.contains("boundWorkGroupId"));
        let _ = fs::remove_dir_all(&dir);
    }
}
