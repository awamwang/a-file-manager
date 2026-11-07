// SPDX-License-Identifier: GPL-3.0-or-later
//! Project-mode — WG super-set: session on bound Work Group; assets on Project.
//!
//! C1 (AIA-207): thin APPDATA index (`projects-index.json`) holds Projects Root +
//! `activeProject` only. Legacy `projects.json` remains until C7 deprecates it.

pub(crate) mod index;
pub(crate) mod migrate;
mod ops;
pub(crate) mod store;
pub(crate) mod types;

use index::{load_from_path as load_index_from_path, save_to_path as save_index_to_path, ProjectsIndex};
use ops::{
    activate_project, add_task, add_workspace_link, bind_work_group, create_project, delete_project,
    delete_task, move_task, open_all_links, promote_work_group_to_project,
    remove_workspace_link, rename_project, save_bound_members_from_open_state, set_active_project,
    set_context_notes, update_task, validate_bindable_work_group,
};
use store::{load_from_path, save_to_path};
use tauri::{AppHandle, Manager};
use types::{ActivateProjectResult, OpenAllLinksResult, ProjectTask, TaskPatch, UserProjects};
use crate::work_groups::store as wg_store;
use crate::work_groups::types::{UserWorkGroups, WorkGroupOpenState};

fn config_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config dir: {err}"))
}

fn configured_projects_root(app: &AppHandle) -> String {
    crate::settings::read_string_setting(app, index::PROJECTS_ROOT_SETTING_KEY)
}

fn load_index(app: &AppHandle) -> Result<ProjectsIndex, String> {
    let dir = config_dir(app)?;
    let configured = configured_projects_root(app);
    load_index_from_path(&dir, &configured)
}

/// Sync thin-index `projectsRoot` after Settings changes `core.projectsRoot`.
pub fn sync_index_root_from_settings(app: &AppHandle) -> Result<ProjectsIndex, String> {
    load_index(app)
}

/// Thin index: Projects Root + activeProject (ADR 0004 / 0007).
#[tauri::command]
pub fn projects_index_get(app: AppHandle) -> Result<ProjectsIndex, String> {
    load_index(&app)
}

/// Set or clear the thin-index active Project by ASCII slug (path = Root/slug).
#[tauri::command]
pub fn projects_index_set_active(
    app: AppHandle,
    slug: Option<String>,
) -> Result<ProjectsIndex, String> {
    let dir = config_dir(&app)?;
    let current = load_index(&app)?;
    let next = index::set_active_project(&current, slug.as_deref())?;
    save_index_to_path(&dir, &next)?;
    Ok(next)
}

/// Resolve effective Projects Root from settings (empty → default), ensure directory, sync index.
#[tauri::command]
pub fn projects_root_ensure(app: AppHandle) -> Result<ProjectsIndex, String> {
    load_index(&app)
}

fn with_document<F>(app: &AppHandle, f: F) -> Result<UserProjects, String>
where
    F: FnOnce(&UserProjects) -> Result<UserProjects, String>,
{
    let dir = config_dir(app)?;
    let document = load_from_path(&dir)?;
    let next = f(&document)?;
    save_to_path(&dir, &next)?;
    Ok(next)
}

fn persist_silent_switch(
    dir: &std::path::Path,
    mut switch: crate::work_groups::types::SilentSwitchResult,
) -> Result<crate::work_groups::types::SilentSwitchResult, String> {
    if let Some(restored) = switch.open_state_to_restore.clone() {
        if let Some(group) = switch
            .document
            .work_groups
            .iter_mut()
            .find(|group| group.id == switch.document.active_work_group_id)
        {
            group.open_state = Some(restored);
        }
    }
    wg_store::save_to_path(dir, &switch.document)?;
    Ok(switch)
}

#[tauri::command]
pub fn projects_get(app: AppHandle) -> Result<UserProjects, String> {
    let dir = config_dir(&app)?;
    load_from_path(&dir)
}

#[tauri::command]
pub fn projects_create(
    app: AppHandle,
    name: Option<String>,
    description: Option<String>,
    work_group_id: Option<String>,
) -> Result<UserProjects, String> {
    let dir = config_dir(&app)?;
    let document = load_from_path(&dir)?;
    if let Some(ref wg_id) = work_group_id {
        let wgs = wg_store::load_from_path(&dir)?;
        validate_bindable_work_group(&wgs, wg_id)?;
    }
    let (next, _) = create_project(&document, name, description, work_group_id);
    save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn projects_rename(
    app: AppHandle,
    project_id: String,
    name: String,
    description: Option<String>,
) -> Result<UserProjects, String> {
    with_document(&app, |document| rename_project(document, &project_id, name, description))
}

#[tauri::command]
pub fn projects_delete(app: AppHandle, project_id: String) -> Result<UserProjects, String> {
    with_document(&app, |document| delete_project(document, &project_id))
}

#[tauri::command]
pub fn projects_set_active(
    app: AppHandle,
    project_id: Option<String>,
) -> Result<UserProjects, String> {
    with_document(&app, |document| set_active_project(document, project_id))
}

/// Session-authoritative activate: set active Project and silent-switch bound WG into the strip.
#[tauri::command]
pub fn projects_activate(
    app: AppHandle,
    project_id: String,
    live_open_state: Option<WorkGroupOpenState>,
) -> Result<ActivateProjectResult, String> {
    let dir = config_dir(&app)?;
    let projects = load_from_path(&dir)?;
    let work_groups = wg_store::load_from_path(&dir)?;
    let (projects_next, switch) =
        activate_project(&projects, &work_groups, &project_id, live_open_state.as_ref())?;
    save_to_path(&dir, &projects_next)?;
    let silent_switch = match switch {
        Some(result) => Some(persist_silent_switch(&dir, result)?),
        None => None,
    };
    Ok(ActivateProjectResult {
        projects: projects_next,
        silent_switch,
    })
}

#[tauri::command]
pub fn projects_bind_work_group(
    app: AppHandle,
    project_id: String,
    work_group_id: Option<String>,
) -> Result<UserProjects, String> {
    let dir = config_dir(&app)?;
    let document = load_from_path(&dir)?;
    let wgs = wg_store::load_from_path(&dir)?;
    let next = bind_work_group(&document, &wgs, &project_id, work_group_id)?;
    save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn projects_promote_from_work_group(
    app: AppHandle,
    work_group_id: String,
) -> Result<UserProjects, String> {
    let dir = config_dir(&app)?;
    let document = load_from_path(&dir)?;
    let wgs = wg_store::load_from_path(&dir)?;
    let (next, _) = promote_work_group_to_project(&document, &wgs, &work_group_id)?;
    save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn projects_save_members_from_open_state(
    app: AppHandle,
    project_id: String,
    live_open_state: Option<WorkGroupOpenState>,
) -> Result<UserWorkGroups, String> {
    let dir = config_dir(&app)?;
    let projects = load_from_path(&dir)?;
    let wgs = wg_store::load_from_path(&dir)?;
    let next =
        save_bound_members_from_open_state(&projects, &wgs, &project_id, live_open_state.as_ref())?;
    wg_store::save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn projects_add_workspace_link(
    app: AppHandle,
    project_id: String,
    path: String,
    label: Option<String>,
) -> Result<UserProjects, String> {
    with_document(&app, |document| {
        add_workspace_link(document, &project_id, &path, label)
    })
}

#[tauri::command]
pub fn projects_remove_workspace_link(
    app: AppHandle,
    project_id: String,
    link_id: String,
) -> Result<UserProjects, String> {
    with_document(&app, |document| {
        remove_workspace_link(document, &project_id, &link_id)
    })
}

#[tauri::command]
pub fn projects_set_context_notes(
    app: AppHandle,
    project_id: String,
    context_notes: String,
) -> Result<UserProjects, String> {
    with_document(&app, |document| {
        set_context_notes(document, &project_id, context_notes)
    })
}

#[tauri::command]
pub fn projects_add_task(
    app: AppHandle,
    project_id: String,
    title: String,
    status_id: Option<String>,
) -> Result<(UserProjects, ProjectTask), String> {
    let dir = config_dir(&app)?;
    let document = load_from_path(&dir)?;
    let (next, task) = add_task(&document, &project_id, title, status_id)?;
    save_to_path(&dir, &next)?;
    Ok((next, task))
}

#[tauri::command]
pub fn projects_update_task(
    app: AppHandle,
    project_id: String,
    task_id: String,
    patch: TaskPatch,
) -> Result<UserProjects, String> {
    with_document(&app, |document| update_task(document, &project_id, &task_id, patch))
}

#[tauri::command]
pub fn projects_move_task(
    app: AppHandle,
    project_id: String,
    task_id: String,
    status_id: String,
    sort_key: i64,
) -> Result<UserProjects, String> {
    with_document(&app, |document| {
        move_task(document, &project_id, &task_id, status_id, sort_key)
    })
}

#[tauri::command]
pub fn projects_delete_task(
    app: AppHandle,
    project_id: String,
    task_id: String,
) -> Result<UserProjects, String> {
    with_document(&app, |document| delete_task(document, &project_id, &task_id))
}

#[tauri::command]
pub fn projects_open_all_links(
    app: AppHandle,
    project_id: String,
) -> Result<OpenAllLinksResult, String> {
    let dir = config_dir(&app)?;
    let document = load_from_path(&dir)?;
    // Open-all still only returns paths — binding / WG hard config unchanged.
    open_all_links(&document, &project_id)
}
