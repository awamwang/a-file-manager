// SPDX-License-Identifier: GPL-3.0-or-later
//! Work Groups — hard config ↔ runtime Open State + silent switch (ADR / design §5).

mod paths;
pub(crate) mod store;
pub(crate) mod sync;
pub(crate) mod types;

use store::{load_from_path, save_to_path};
use sync::{
    add_member_to_hard_config, copy_work_group_in_list, create_and_activate_work_group,
    delete_work_group_in_list, remove_member_from_hard_config, rename_work_group_in_list,
    save_members_from_open_state_to_hard, save_runtime_open_state_to_hard,
    silent_switch_work_group,
};
use tauri::{AppHandle, Manager};
use types::{
    SilentSwitchResult, UserWorkGroups, WorkGroup, WorkGroupOpenState, WorkGroupSoftSyncNotice,
};

fn config_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config dir: {err}"))
}

#[tauri::command]
pub fn work_groups_get(app: AppHandle) -> Result<UserWorkGroups, String> {
    let dir = config_dir(&app)?;
    load_from_path(&dir)
}

#[tauri::command]
pub fn work_groups_create(
    app: AppHandle,
    name: Option<String>,
    live_open_state: Option<WorkGroupOpenState>,
) -> Result<SilentSwitchResult, String> {
    let dir = config_dir(&app)?;
    let document = load_from_path(&dir)?;
    let (mut result, _created) =
        create_and_activate_work_group(&document, name, live_open_state.as_ref())?;
    // Persist flushed leaving group + new active id; stamp restored open state onto target.
    if let Some(restored) = result.open_state_to_restore.clone() {
        if let Some(group) = result
            .document
            .work_groups
            .iter_mut()
            .find(|group| group.id == result.document.active_work_group_id)
        {
            group.open_state = Some(restored);
        }
    }
    save_to_path(&dir, &result.document)?;
    Ok(result)
}

#[tauri::command]
pub fn work_groups_add_member(
    app: AppHandle,
    work_group_id: String,
    member_path: String,
    live_open_state: Option<WorkGroupOpenState>,
) -> Result<(UserWorkGroups, WorkGroupOpenState, Option<WorkGroupSoftSyncNotice>), String> {
    let dir = config_dir(&app)?;
    let mut document = load_from_path(&dir)?;
    let index = document
        .work_groups
        .iter()
        .position(|group| group.id == work_group_id)
        .ok_or_else(|| format!("Unknown Work Group: {work_group_id}"))?;
    let (next_group, next_open, notice) = add_member_to_hard_config(
        &document.work_groups[index],
        live_open_state.as_ref(),
        &member_path,
    );
    document.work_groups[index] = WorkGroup {
        open_state: Some(next_open.clone()),
        ..next_group
    };
    save_to_path(&dir, &document)?;
    Ok((document, next_open, notice))
}

#[tauri::command]
pub fn work_groups_remove_member(
    app: AppHandle,
    work_group_id: String,
    member_path: String,
) -> Result<UserWorkGroups, String> {
    let dir = config_dir(&app)?;
    let mut document = load_from_path(&dir)?;
    let index = document
        .work_groups
        .iter()
        .position(|group| group.id == work_group_id)
        .ok_or_else(|| format!("Unknown Work Group: {work_group_id}"))?;
    document.work_groups[index] =
        remove_member_from_hard_config(&document.work_groups[index], &member_path)?;
    save_to_path(&dir, &document)?;
    Ok(document)
}

#[tauri::command]
pub fn work_groups_rename(
    app: AppHandle,
    work_group_id: String,
    name: String,
) -> Result<UserWorkGroups, String> {
    let dir = config_dir(&app)?;
    let mut document = load_from_path(&dir)?;
    document.work_groups = rename_work_group_in_list(&document.work_groups, &work_group_id, &name)?;
    save_to_path(&dir, &document)?;
    Ok(document)
}

/// Copy a Work Group (members + openState with fresh tab ids). Does not change active.
#[tauri::command]
pub fn work_groups_copy(
    app: AppHandle,
    work_group_id: String,
) -> Result<(UserWorkGroups, WorkGroup), String> {
    let dir = config_dir(&app)?;
    let document = load_from_path(&dir)?;
    let (next, created) = copy_work_group_in_list(&document, &work_group_id)?;
    save_to_path(&dir, &next)?;
    Ok((next, created))
}

#[tauri::command]
pub fn work_groups_delete(
    app: AppHandle,
    work_group_id: String,
) -> Result<UserWorkGroups, String> {
    let dir = config_dir(&app)?;
    let document = load_from_path(&dir)?;
    let next = delete_work_group_in_list(&document, &work_group_id)?;
    save_to_path(&dir, &next)?;
    Ok(next)
}

#[tauri::command]
pub fn work_groups_silent_switch(
    app: AppHandle,
    target_work_group_id: String,
    live_open_state: Option<WorkGroupOpenState>,
) -> Result<SilentSwitchResult, String> {
    let dir = config_dir(&app)?;
    let document = load_from_path(&dir)?;
    let mut result =
        silent_switch_work_group(&document, &target_work_group_id, live_open_state.as_ref())?;
    // Persist flushed leaving group + new active id; also stamp restored open state onto target.
    if let Some(restored) = result.open_state_to_restore.clone() {
        if let Some(group) = result
            .document
            .work_groups
            .iter_mut()
            .find(|group| group.id == result.document.active_work_group_id)
        {
            group.open_state = Some(restored);
        }
    }
    save_to_path(&dir, &result.document)?;
    Ok(result)
}

#[tauri::command]
pub fn work_groups_save_open_state(
    app: AppHandle,
    work_group_id: String,
    live_open_state: Option<WorkGroupOpenState>,
) -> Result<UserWorkGroups, String> {
    let dir = config_dir(&app)?;
    let mut document = load_from_path(&dir)?;
    let index = document
        .work_groups
        .iter()
        .position(|group| group.id == work_group_id)
        .ok_or_else(|| format!("Unknown Work Group: {work_group_id}"))?;
    document.work_groups[index] =
        save_runtime_open_state_to_hard(&document.work_groups[index], live_open_state.as_ref());
    save_to_path(&dir, &document)?;
    Ok(document)
}

#[tauri::command]
pub fn work_groups_save_members_from_open_state(
    app: AppHandle,
    work_group_id: String,
    live_open_state: Option<WorkGroupOpenState>,
) -> Result<UserWorkGroups, String> {
    let dir = config_dir(&app)?;
    let mut document = load_from_path(&dir)?;
    let index = document
        .work_groups
        .iter()
        .position(|group| group.id == work_group_id)
        .ok_or_else(|| format!("Unknown Work Group: {work_group_id}"))?;
    document.work_groups[index] = save_members_from_open_state_to_hard(
        &document.work_groups[index],
        live_open_state.as_ref(),
    );
    save_to_path(&dir, &document)?;
    Ok(document)
}
