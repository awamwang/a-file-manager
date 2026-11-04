// SPDX-License-Identifier: GPL-3.0-or-later
//! Project-mode — Cursor-like domain container (independent of Work Groups).

pub(crate) mod migrate;
mod ops;
pub(crate) mod store;
pub(crate) mod types;

use ops::{
    add_task, add_workspace_link, create_project, delete_project, delete_task, move_task,
    open_all_links, remove_workspace_link, rename_project, set_active_project, set_context_notes,
    update_task,
};
use store::{load_from_path, save_to_path};
use tauri::{AppHandle, Manager};
use types::{OpenAllLinksResult, ProjectTask, TaskPatch, UserProjects};

fn config_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config dir: {err}"))
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
) -> Result<UserProjects, String> {
    with_document(&app, |document| {
        let (next, _) = create_project(document, name, description);
        Ok(next)
    })
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
    // Intentionally does not load or write work_groups — Navigator tabs only.
    open_all_links(&document, &project_id)
}
