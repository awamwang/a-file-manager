// SPDX-License-Identifier: GPL-3.0-or-later
//! One-time first-run presets: one example Work Group + two example Projects.
//!
//! The presets are ordinary, user-owned data: they are meant to be renamed and
//! kept in real use, so they are NOT re-seeded once deleted. A small marker file
//! records that seeding already happened. Seeding is idempotent: it only adds
//! entries that are still missing, so a partial failure can be retried on the
//! next launch without duplicating anything.

use crate::projects::index::{resolve_projects_root, workspace_path_for_slug, PROJECTS_ROOT_SETTING_KEY};
use crate::projects::workspace::{create_workspace_at, CreateWorkspaceResult, WorkspaceListItem};
use crate::work_groups::store as wg_store;
use crate::work_groups::types::{
    UserWorkGroups, WorkGroup, WorkGroupOpenState, WorkGroupOpenStateTab,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Stable id of the preset Work Group; not the default group, so it is
/// bindable and can be freely renamed / deleted by the user.
pub const EXAMPLE_WORK_GROUP_ID: &str = "work-group-example-1";
pub const EXAMPLE_WORK_GROUP_NAME: &str = "示例工作组一";
pub const EXAMPLE_PROJECT_NAME_1: &str = "我的项目一";
pub const EXAMPLE_PROJECT_NAME_2: &str = "我的项目二";

const SEED_FILE_NAME: &str = "examples-seeded.json";
const SEED_SCHEMA_VERSION: u32 = 1;
const EXAMPLE_TAB_ID: &str = "tab-example-home";

/// Preset projects: (display name, slug, optional bound work group).
/// Project 1 is bound to the example group for the full teaching flow; project 2
/// stays unbound so it can be activated independently of any work group.
const EXAMPLE_PROJECT_SPECS: &[(&str, &str, Option<&str>)] = &[
    (
        EXAMPLE_PROJECT_NAME_1,
        "example-project-1",
        Some(EXAMPLE_WORK_GROUP_ID),
    ),
    (EXAMPLE_PROJECT_NAME_2, "example-project-2", None),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct SeedMarker {
    schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExampleSeedResult {
    /// True when this call actually created anything.
    pub seeded: bool,
    pub work_group_created: bool,
    pub projects_created: Vec<String>,
}

fn seed_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join(SEED_FILE_NAME)
}

fn already_seeded(config_dir: &Path) -> bool {
    let Ok(text) = fs::read_to_string(seed_file_path(config_dir)) else {
        return false;
    };
    serde_json::from_str::<SeedMarker>(&text)
        .map(|marker| marker.schema_version == SEED_SCHEMA_VERSION)
        .unwrap_or(false)
}

fn write_seed_marker(config_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(config_dir)
        .map_err(|err| format!("Failed to create config dir for seed marker: {err}"))?;
    let marker = SeedMarker {
        schema_version: SEED_SCHEMA_VERSION,
    };
    let text = serde_json::to_string_pretty(&marker)
        .map_err(|err| format!("Failed to serialize seed marker: {err}"))?;
    fs::write(seed_file_path(config_dir), text)
        .map_err(|err| format!("Failed to write seed marker: {err}"))
}

/// A single directory tab pointing at the user's home directory, used as the
/// example Work Group's Open State so activating a preset opens a real, useful
/// location rather than an empty folder.
fn home_open_state(home: &Path) -> WorkGroupOpenState {
    let path = crate::paths::to_native_fs_path(&home.to_string_lossy());
    let name = home
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Home")
        .to_string();
    WorkGroupOpenState {
        tab_groups: vec![vec![WorkGroupOpenStateTab {
            id: EXAMPLE_TAB_ID.to_string(),
            name,
            path,
            tab_type: "directory".to_string(),
        }]],
        current_tab_group_index: 0,
        current_tab_index: 0,
    }
}

fn example_work_group(home: &Path) -> WorkGroup {
    let home_path = crate::paths::to_native_fs_path(&home.to_string_lossy());
    WorkGroup {
        id: EXAMPLE_WORK_GROUP_ID.to_string(),
        name: EXAMPLE_WORK_GROUP_NAME.to_string(),
        is_default: false,
        member_paths: vec![home_path],
        open_state: Some(home_open_state(home)),
    }
}

/// Ensure the example Work Group exists in the document. Returns true when it
/// was newly added. Uses the shared document lock so a startup seed cannot race
/// with a command that is mutating the same file.
fn ensure_example_work_group(config_dir: &Path, home: &Path) -> Result<bool, String> {
    let _guard = wg_store::lock_document()?;
    let mut document = wg_store::load_from_path(config_dir)?;
    if document
        .work_groups
        .iter()
        .any(|group| group.id == EXAMPLE_WORK_GROUP_ID)
    {
        return Ok(false);
    }
    document.work_groups.push(example_work_group(home));
    wg_store::save_to_path(config_dir, &document)?;
    Ok(true)
}

/// Ensure one example Project workspace exists. Returns the created slug, or
/// None when a workspace at that slug already exists (partial-run recovery).
fn ensure_example_project(
    projects_root: &Path,
    name: &str,
    slug: &str,
    bound_work_group_id: Option<String>,
) -> Result<Option<String>, String> {
    if workspace_path_for_slug(projects_root, slug)?.exists() {
        return Ok(None);
    }
    let created: CreateWorkspaceResult = create_workspace_at(
        projects_root,
        name,
        Some("用于教学的示例项目，可直接改名后作为你自己的项目使用。"),
        Some(slug),
        bound_work_group_id,
    )?;
    Ok(Some(created.project.slug))
}

/// Pure, testable seed routine. `config_dir` holds `work-groups.json` and the
/// marker; `projects_root` is the resolved Projects Root; `home` is the user's
/// home directory used as the preset Work Group's hard member + Open State tab.
pub fn seed_examples(
    config_dir: &Path,
    projects_root: &Path,
    home: &Path,
) -> Result<ExampleSeedResult, String> {
    if already_seeded(config_dir) {
        return Ok(ExampleSeedResult {
            seeded: false,
            work_group_created: false,
            projects_created: Vec::new(),
        });
    }

    let work_group_created = ensure_example_work_group(config_dir, home)?;

    let mut projects_created = Vec::new();
    for (name, slug, bound_work_group) in EXAMPLE_PROJECT_SPECS {
        if let Some(created) = ensure_example_project(
            projects_root,
            name,
            slug,
            bound_work_group.map(|id| id.to_string()),
        )? {
            projects_created.push(created);
        }
    }

    write_seed_marker(config_dir)?;

    Ok(ExampleSeedResult {
        seeded: work_group_created || !projects_created.is_empty(),
        work_group_created,
        projects_created,
    })
}

/// Resolve paths from the running App and run [`seed_examples`] once. Called
/// from startup; a missing home directory is not fatal — seeding is skipped and
/// retried on the next launch.
pub fn seed_examples_for_app(app: &tauri::AppHandle) -> Result<ExampleSeedResult, String> {
    use tauri::Manager;

    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config dir: {err}"))?;
    let configured = crate::settings::read_string_setting(app, PROJECTS_ROOT_SETTING_KEY);
    let projects_root = resolve_projects_root(&config_dir, &configured);
    let Some(home) = crate::platform::home_dir() else {
        return Ok(ExampleSeedResult {
            seeded: false,
            work_group_created: false,
            projects_created: Vec::new(),
        });
    };
    seed_examples(&config_dir, &projects_root, &home)
}

/// Number of preset Work Groups in a document (test helper / diagnostic).
#[allow(dead_code)]
pub fn count_example_work_groups(document: &UserWorkGroups) -> usize {
    document
        .work_groups
        .iter()
        .filter(|group| group.id == EXAMPLE_WORK_GROUP_ID)
        .count()
}

/// List preset Projects currently on disk (test helper / diagnostic).
#[allow(dead_code)]
pub fn list_example_projects(projects_root: &Path) -> Result<Vec<WorkspaceListItem>, String> {
    let all = crate::projects::workspace::list_workspaces(projects_root)?;
    let slugs: Vec<&str> = EXAMPLE_PROJECT_SPECS
        .iter()
        .map(|(_, slug, _)| *slug)
        .collect();
    Ok(all
        .into_iter()
        .filter(|item| slugs.contains(&item.project.slug.as_str()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::workspace::list_workspaces;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-examples-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn seeds_work_group_and_two_projects_with_home_tab() {
        let base = temp_dir("seed");
        let config = base.join("config");
        let root = base.join("projects");
        let home = base.join("home");
        fs::create_dir_all(&home).unwrap();

        let result = seed_examples(&config, &root, &home).unwrap();
        assert!(result.seeded);
        assert!(result.work_group_created);
        assert_eq!(result.projects_created.len(), 2);
        assert!(seed_file_path(&config).is_file());

        let wgs = wg_store::load_from_path(&config).unwrap();
        let example = wgs
            .work_groups
            .iter()
            .find(|group| group.id == EXAMPLE_WORK_GROUP_ID)
            .expect("example work group present");
        assert_eq!(example.name, EXAMPLE_WORK_GROUP_NAME);
        assert!(!example.is_default);
        let native_home = crate::paths::to_native_fs_path(&home.to_string_lossy());
        assert_eq!(example.member_paths, vec![native_home.clone()]);
        let tab = &example.open_state.as_ref().unwrap().tab_groups[0][0];
        assert_eq!(tab.path, native_home);
        assert_eq!(tab.tab_type, "directory");

        let projects = list_workspaces(&root).unwrap();
        assert_eq!(projects.len(), 2);
        let project1 = projects
            .iter()
            .find(|p| p.project.display_name == EXAMPLE_PROJECT_NAME_1)
            .expect("project 1 present");
        assert_eq!(
            project1.project.bound_work_group_id.as_deref(),
            Some(EXAMPLE_WORK_GROUP_ID)
        );
        let project2 = projects
            .iter()
            .find(|p| p.project.display_name == EXAMPLE_PROJECT_NAME_2)
            .expect("project 2 present");
        assert_eq!(project2.project.bound_work_group_id.as_deref(), None);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn marker_prevents_reseed_after_user_deletes_presets() {
        let base = temp_dir("marker");
        let config = base.join("config");
        let root = base.join("projects");
        let home = base.join("home");
        fs::create_dir_all(&home).unwrap();

        seed_examples(&config, &root, &home).unwrap();

        // Simulate the user converting the presets to their own use / deleting them.
        {
            let _guard = wg_store::lock_document().unwrap();
            let mut wgs = wg_store::load_from_path(&config).unwrap();
            wgs.work_groups
                .retain(|group| group.id != EXAMPLE_WORK_GROUP_ID);
            wg_store::save_to_path(&config, &wgs).unwrap();
        }
        for slug in ["example-project-1", "example-project-2"] {
            fs::remove_dir_all(workspace_path_for_slug(&root, slug).unwrap()).unwrap();
        }

        let again = seed_examples(&config, &root, &home).unwrap();
        assert!(!again.seeded);
        assert!(!again.work_group_created);
        assert!(again.projects_created.is_empty());
        let wgs = wg_store::load_from_path(&config).unwrap();
        assert_eq!(count_example_work_groups(&wgs), 0);
        assert!(list_workspaces(&root).unwrap().is_empty());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn partial_run_is_repaired_without_duplicating() {
        let base = temp_dir("partial");
        let config = base.join("config");
        let root = base.join("projects");
        let home = base.join("home");
        fs::create_dir_all(&home).unwrap();

        seed_examples(&config, &root, &home).unwrap();

        // Corrupt the marker so the seed re-runs, but leave the data intact.
        fs::write(seed_file_path(&config), "{ broken json").unwrap();

        let again = seed_examples(&config, &root, &home).unwrap();
        assert!(!again.seeded);
        assert!(!again.work_group_created);
        assert!(again.projects_created.is_empty());
        let wgs = wg_store::load_from_path(&config).unwrap();
        assert_eq!(count_example_work_groups(&wgs), 1);
        assert_eq!(list_example_projects(&root).unwrap().len(), 2);

        let _ = fs::remove_dir_all(&base);
    }
}
