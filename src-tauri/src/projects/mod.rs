// SPDX-License-Identifier: GPL-3.0-or-later
//! Project-mode 鈥?WG super-set: session on bound Work Group; assets on Project.
//!
//! C1 (AIA-207): thin APPDATA index (`projects-index.json`) holds Projects Root +
//! `activeProject` only. C7 removed `projects.json` UI authority.
//! C2 (AIA-208): Core Workspace light create under Projects Root (ADR 0008).
//! C3 (AIA-209): list / activate / bind WG on disk Workspaces.
//! C4 (AIA-210): `members/` junctions; open real target paths.
//! C5 (AIA-211): CONTEXT.md + board.json four-column board.
//! C6 (AIA-212): delete / rename / Promote WG鈫扺orkspace.
//! AIA-202: one-shot migrate leftover APPDATA `projects.json` 鈫?Workspace layout.

pub(crate) mod board;
pub(crate) mod helpers;
pub(crate) mod index;
pub(crate) mod legacy_migrate;
pub(crate) mod lifecycle;
pub(crate) mod members;
pub(crate) mod migrate;
/// Legacy projects.json 突变仅供单测；活路径用 Workspace / helpers。
#[cfg(test)]
mod ops;
pub(crate) mod store;
pub(crate) mod types;
pub(crate) mod workspace;

use crate::work_groups::store as wg_store;
use crate::work_groups::types::WorkGroupOpenState;
use board::{BoardTask, BoardTaskPatch, WorkspaceBoard};
use helpers::validate_bindable_work_group;
use index::{
    load_from_path as load_index_from_path, reorder_workspace_slugs,
    save_to_path as save_index_to_path, ProjectsIndex,
};
use lifecycle::{
    CopyWorkspaceResult, DeleteWorkspaceResult, PromoteWorkspaceResult, RenameWorkspaceResult,
};
use members::WorkspaceMember;
use tauri::{AppHandle, Manager};
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
    // AIA-202: one-shot AFM legacy projects.json 鈫?Workspace (no Sigma read).
    legacy_migrate::maybe_migrate_legacy_projects(&dir, &configured)?;
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

/// Resolve effective Projects Root from settings (empty 鈫?default), ensure directory, sync index.
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

    let created = workspace::create_workspace_at(
        &root,
        &display_name,
        description.as_deref(),
        slug.as_deref(),
        bound,
    )?;
    let prior_order_len = index.workspace_order.len();
    let next_index = index.append_workspace_order(&created.project.slug);
    if next_index.workspace_order.len() != prior_order_len {
        save_index_to_path(&dir, &next_index)?;
    }
    Ok(created)
}

/// Scan Projects Root for Workspaces (`project.json` dirs).
#[tauri::command]
pub fn projects_workspace_list(app: AppHandle) -> Result<WorkspaceListResult, String> {
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    let workspaces =
        workspace::apply_workspace_order(workspace::list_workspaces(&root)?, &index.workspace_order);
    Ok(WorkspaceListResult {
        projects_root: index.projects_root,
        active_slug: index.active_project.map(|a| a.slug),
        workspaces,
    })
}

/// 拖动调整项目列表顺序，写入 thin-index `workspaceOrder`。
#[tauri::command]
pub fn projects_workspace_reorder(
    app: AppHandle,
    from_slug: String,
    to_slug: String,
) -> Result<WorkspaceListResult, String> {
    let dir = config_dir(&app)?;
    let mut index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    let listed = workspace::apply_workspace_order(
        workspace::list_workspaces(&root)?,
        &index.workspace_order,
    );
    let fallback: Vec<String> = listed
        .iter()
        .map(|item| item.project.slug.clone())
        .collect();
    index.workspace_order =
        reorder_workspace_slugs(&index.workspace_order, &fallback, &from_slug, &to_slug)?;
    save_index_to_path(&dir, &index)?;
    let workspaces = workspace::apply_workspace_order(listed, &index.workspace_order);
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
/// `mklink /J` is process-spawn heavy; run off the UI thread.
#[tauri::command]
pub async fn projects_workspace_member_add(
    app: AppHandle,
    slug: String,
    target_path: String,
    link_name: Option<String>,
) -> Result<WorkspaceMember, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let index = load_index(&app)?;
        let root = std::path::PathBuf::from(&index.projects_root);
        members::add_member_for_slug(&root, &slug, &target_path, link_name.as_deref())
    })
    .await
    .map_err(|err| format!("member_add worker failed: {err}"))?
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
    let mut result = lifecycle::delete_workspace(&root, &index, &slug)?;
    result.index = result.index.remove_workspace_order(&slug);
    save_index_to_path(&dir, &result.index)?;
    Ok(result)
}

/// Rename displayName and/or slug (slug renames the directory; conflict 鈫?PROJECT_SLUG_CONFLICT).
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
    let mut result = lifecycle::rename_workspace(
        &root,
        &index,
        &slug,
        display_name.as_deref(),
        new_slug.as_deref(),
    )?;
    result.index = result
        .index
        .rename_workspace_order(&slug, &result.workspace.project.slug);
    save_index_to_path(&dir, &result.index)?;
    Ok(result)
}

/// Promote non-default WG → Workspace: create + bind + seed members/ junctions.
/// Runs off the UI thread (junction mklink can be slow). Does not touch WG OpenState
/// or silent-switch the explorer active Work Group — only sets thin-index activeProject.
#[tauri::command]
pub async fn projects_workspace_promote_from_work_group(
    app: AppHandle,
    work_group_id: String,
) -> Result<PromoteWorkspaceResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = config_dir(&app)?;
        let index = load_index(&app)?;
        let root = std::path::PathBuf::from(&index.projects_root);
        let wgs = wg_store::load_from_path(&dir)?;
        let mut result =
            lifecycle::promote_work_group_to_workspace(&root, &wgs, &work_group_id, &index)?;
        result.index = result
            .index
            .append_workspace_order(&result.workspace.project.slug);
        save_index_to_path(&dir, &result.index)?;
        Ok(result)
    })
    .await
    .map_err(|err| format!("promote worker failed: {err}"))?
}

/// Copy a Workspace (CONTEXT / board / members). Bound WG is copied with fresh openState tab ids.
#[tauri::command]
pub fn projects_workspace_copy(
    app: AppHandle,
    slug: String,
) -> Result<CopyWorkspaceResult, String> {
    let dir = config_dir(&app)?;
    let index = load_index(&app)?;
    let root = std::path::PathBuf::from(&index.projects_root);
    let wgs = wg_store::load_from_path(&dir)?;
    let (result, wgs_next) = lifecycle::copy_workspace(&root, &wgs, &slug)?;
    if result.copied_work_group.is_some() {
        wg_store::save_to_path(&dir, &wgs_next)?;
    }
    // copy_workspace 不改 thin-index；有自定义顺序时把副本追加到末尾。
    let prior_order_len = index.workspace_order.len();
    let next_index = index.append_workspace_order(&result.workspace.project.slug);
    if next_index.workspace_order.len() != prior_order_len {
        save_index_to_path(&dir, &next_index)?;
    }
    Ok(result)
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

