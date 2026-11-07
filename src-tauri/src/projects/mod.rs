// SPDX-License-Identifier: GPL-3.0-or-later
//! Project-mode — WG super-set: session on bound Work Group; assets on Project.
//!
//! C1 (AIA-207): thin APPDATA index (`projects-index.json`) holds Projects Root +
//! `activeProject` only. Legacy `projects.json` remains until C7 deprecates it.
//! C2 (AIA-208): Core Workspace light create under Projects Root (ADR 0008).
//! C3 (AIA-209): list / activate / bind WG on disk Workspaces.
//! C4 (AIA-210): `members/` junctions; open real target paths.
//! C5 (AIA-211): CONTEXT.md + board.json four-column board.
//! C6 (AIA-212): delete / rename / Promote WG→Workspace.

pub(crate) mod board;
pub(crate) mod index;
pub(crate) mod lifecycle;
pub(crate) mod members;
pub(crate) mod migrate;
mod ops;
pub(crate) mod store;
pub(crate) mod types;
pub(crate) mod workspace;

use crate::work_groups::store as wg_store;
use crate::work_groups::types::{UserWorkGroups, WorkGroupOpenState};
use index::{load_from_path as load_index_from_path, save_to_path as save_index_to_path, ProjectsIndex};
use ops::{
    activate_project, add_task, add_workspace_link, bind_work_group, create_project, delete_project,
    delete_task, move_task, open_all_links, promote_work_group_to_project, remove_workspace_link,
    rename_project, save_bound_members_from_open_state, set_active_project, set_context_notes,
    update_task, validate_bindable_work_group,
};
use store::{load_from_path, save_to_path};
use tauri::{AppHandle, Manager};
use types::{ActivateProjectResult, OpenAllLinksResult, ProjectTask, TaskPatch, UserProjects};
use board::{BoardTask, BoardTaskPatch, WorkspaceBoard};
use lifecycle::{DeleteWorkspaceResult, PromoteWorkspaceResult, RenameWorkspaceResult};
use members::WorkspaceMember;
use workspace::{
    ActivateWorkspaceResult, CreateWorkspaceResult, WorkspaceListItem, WorkspaceListResult,
};

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

/// Core light create: write `project.json` + `CONTEXT.md` + `members/` + `board.json` under Projects Root.
/// Does not write `AGENTS.md` / `skills/` (ADR 0008). Does not activate (C3) or touch legacy projects.json.
#[tauri::command]
pub fn projects_workspace_create(
    app: AppHandle,
    display_name: String,
    description: Option<String>,
    slug: Option<String>,
    work_group_id: Option<String>,
) -> Result<CreateWorkspaceResult, String> {
    let dir = config_dir(&app)?;
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);

    let bound = if let Some(ref wg_id) = work_group_id {
        let wgs = wg_store::load_from_path(&dir)?;
        validate_bindable_work_group(&wgs, wg_id)?;
        Some(wg_id.clone())
    } else {
        None
    };

    workspace::create_workspace_at(
        &root,
        &display_name,
        description.as_deref(),
        slug.as_deref(),
        bound,
    )
}

/// Scan Projects Root for Workspaces (`project.json` dirs).
#[tauri::command]
pub fn projects_workspace_list(app: AppHandle) -> Result<WorkspaceListResult, String> {
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    let workspaces = workspace::list_workspaces(&root)?;
    Ok(WorkspaceListResult {
        projects_root: index.projects_root,
        active_slug: index.active_project.map(|a| a.slug),
        workspaces,
    })
}

/// Bind or clear a non-default Work Group on a Workspace (writes `project.json`).
#[tauri::command]
pub fn projects_workspace_bind_work_group(
    app: AppHandle,
    slug: String,
    work_group_id: Option<String>,
) -> Result<WorkspaceListItem, String> {
    let dir = config_dir(&app)?;
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    let wgs = wg_store::load_from_path(&dir)?;
    workspace::bind_workspace_work_group(&root, &wgs, &slug, work_group_id)
}

/// Activate Workspace: thin-index activeProject + silent-switch bound WG (if any).
#[tauri::command]
pub fn projects_workspace_activate(
    app: AppHandle,
    slug: String,
    live_open_state: Option<WorkGroupOpenState>,
) -> Result<ActivateWorkspaceResult, String> {
    let dir = config_dir(&app)?;
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    let wgs = wg_store::load_from_path(&dir)?;
    let mut result = workspace::activate_workspace(
        &root,
        &index,
        &wgs,
        &slug,
        live_open_state.as_ref(),
    )?;
    save_index_to_path(&dir, &result.index)?;
    if let Some(switch) = result.silent_switch.take() {
        result.silent_switch = Some(persist_silent_switch(&dir, switch)?);
    }
    Ok(result)
}

/// Create a directory junction under Workspace `members/` → `target_path`.
#[tauri::command]
pub fn projects_workspace_member_add(
    app: AppHandle,
    slug: String,
    target_path: String,
    link_name: Option<String>,
) -> Result<WorkspaceMember, String> {
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    members::add_member_for_slug(&root, &slug, &target_path, link_name.as_deref())
}

/// List `members/` junctions; `openPath` is always the real target.
#[tauri::command]
pub fn projects_workspace_member_list(
    app: AppHandle,
    slug: String,
) -> Result<Vec<WorkspaceMember>, String> {
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    members::list_members_for_slug(&root, &slug)
}

/// Remove a member junction only (never deletes the real folder).
#[tauri::command]
pub fn projects_workspace_member_remove(
    app: AppHandle,
    slug: String,
    name: String,
) -> Result<(), String> {
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    members::remove_member_for_slug(&root, &slug, &name)
}

/// Resolve junction/symlink to the real path for Navigator / tab open (C4).
#[tauri::command]
pub fn projects_resolve_open_path(path: String) -> Result<String, String> {
    members::resolve_open_path(&path)
}

/// Read Workspace `CONTEXT.md`.
#[tauri::command]
pub fn projects_workspace_context_get(app: AppHandle, slug: String) -> Result<String, String> {
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    board::read_context_for_slug(&root, &slug)
}

/// Write Workspace `CONTEXT.md` (full replace).
#[tauri::command]
pub fn projects_workspace_context_set(
    app: AppHandle,
    slug: String,
    content: String,
) -> Result<(), String> {
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    board::write_context_for_slug(&root, &slug, &content)
}

/// Read Workspace `board.json` (four columns: backlog/doing/review/done).
#[tauri::command]
pub fn projects_workspace_board_get(
    app: AppHandle,
    slug: String,
) -> Result<WorkspaceBoard, String> {
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    board::read_board_for_slug(&root, &slug)
}

#[tauri::command]
pub fn projects_workspace_board_add_task(
    app: AppHandle,
    slug: String,
    title: String,
    status_id: Option<String>,
) -> Result<(WorkspaceBoard, BoardTask), String> {
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    let item = workspace::load_workspace(&root, &slug)?;
    let dir = std::path::Path::new(&item.path);
    let board = board::read_board(dir)?;
    let (next, task) = board::add_task(&board, &title, status_id.as_deref())?;
    board::write_board(dir, &next)?;
    Ok((next, task))
}

#[tauri::command]
pub fn projects_workspace_board_update_task(
    app: AppHandle,
    slug: String,
    task_id: String,
    patch: BoardTaskPatch,
) -> Result<WorkspaceBoard, String> {
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    board::mutate_board_for_slug(&root, &slug, |b| board::update_task(b, &task_id, patch))
}

#[tauri::command]
pub fn projects_workspace_board_move_task(
    app: AppHandle,
    slug: String,
    task_id: String,
    status_id: String,
    sort_key: i64,
) -> Result<WorkspaceBoard, String> {
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    board::mutate_board_for_slug(&root, &slug, |b| {
        board::move_task(b, &task_id, &status_id, sort_key)
    })
}

#[tauri::command]
pub fn projects_workspace_board_delete_task(
    app: AppHandle,
    slug: String,
    task_id: String,
) -> Result<WorkspaceBoard, String> {
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    board::mutate_board_for_slug(&root, &slug, |b| board::delete_task(b, &task_id))
}

/// Delete entire Workspace directory (junction targets preserved). Clears thin-index active if needed.
#[tauri::command]
pub fn projects_workspace_delete(
    app: AppHandle,
    slug: String,
) -> Result<DeleteWorkspaceResult, String> {
    let dir = config_dir(&app)?;
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    let result = lifecycle::delete_workspace(&root, &index, &slug)?;
    save_index_to_path(&dir, &result.index)?;
    Ok(result)
}

/// Rename displayName and/or slug (slug renames the directory; conflict → PROJECT_SLUG_CONFLICT).
#[tauri::command]
pub fn projects_workspace_rename(
    app: AppHandle,
    slug: String,
    display_name: Option<String>,
    new_slug: Option<String>,
) -> Result<RenameWorkspaceResult, String> {
    let dir = config_dir(&app)?;
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    let result = lifecycle::rename_workspace(
        &root,
        &index,
        &slug,
        display_name.as_deref(),
        new_slug.as_deref(),
    )?;
    save_index_to_path(&dir, &result.index)?;
    Ok(result)
}

/// Promote non-default WG → Workspace: create + bind + seed members/ junctions from memberPaths.
#[tauri::command]
pub fn projects_workspace_promote_from_work_group(
    app: AppHandle,
    work_group_id: String,
) -> Result<PromoteWorkspaceResult, String> {
    let dir = config_dir(&app)?;
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    let wgs = wg_store::load_from_path(&dir)?;
    lifecycle::promote_work_group_to_workspace(&root, &wgs, &work_group_id)
}

#[allow(dead_code)]
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

// --- Legacy projects.json IPC (no longer registered in lib.rs; AIA-213 / C7) ---
// Disk helpers below remain for sigma_import / cleanup. Do not re-expose as UI authority.

#[allow(dead_code)]
#[tauri::command]
pub fn projects_get(app: AppHandle) -> Result<UserProjects, String> {
    let dir = config_dir(&app)?;
    load_from_path(&dir)
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[tauri::command]
pub fn projects_rename(
    app: AppHandle,
    project_id: String,
    name: String,
    description: Option<String>,
) -> Result<UserProjects, String> {
    with_document(&app, |document| rename_project(document, &project_id, name, description))
}

#[allow(dead_code)]
#[tauri::command]
pub fn projects_delete(app: AppHandle, project_id: String) -> Result<UserProjects, String> {
    with_document(&app, |document| delete_project(document, &project_id))
}

#[allow(dead_code)]
#[tauri::command]
pub fn projects_set_active(
    app: AppHandle,
    project_id: Option<String>,
) -> Result<UserProjects, String> {
    with_document(&app, |document| set_active_project(document, project_id))
}

/// Legacy session activate against projects.json (superseded by projects_workspace_activate).
#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
#[tauri::command]
pub fn projects_update_task(
    app: AppHandle,
    project_id: String,
    task_id: String,
    patch: TaskPatch,
) -> Result<UserProjects, String> {
    with_document(&app, |document| update_task(document, &project_id, &task_id, patch))
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[tauri::command]
pub fn projects_delete_task(
    app: AppHandle,
    project_id: String,
    task_id: String,
) -> Result<UserProjects, String> {
    with_document(&app, |document| delete_task(document, &project_id, &task_id))
}

#[allow(dead_code)]
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
