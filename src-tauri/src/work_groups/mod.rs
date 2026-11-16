// SPDX-License-Identifier: GPL-3.0-or-later
//! Work Groups — hard config ↔ runtime Open State + silent switch (ADR / design §5).

mod paths;
pub(crate) mod store;
pub(crate) mod sync;
pub(crate) mod types;

use store::{load_from_path, lock_document, save_to_path};
use sync::{
    add_member_to_hard_config, copy_work_group_in_list, create_and_activate_work_group,
    delete_work_group_in_list, remove_member_from_hard_config, rename_work_group_in_list,
    reorder_work_groups_in_list, save_members_from_open_state_to_hard,
    save_runtime_open_state_to_hard, silent_switch_work_group,
};
use tauri::{AppHandle, Manager};
use types::{
    PinnedTab, SilentSwitchResult, UserWorkGroups, WorkGroup, WorkGroupOpenState,
    WorkGroupSoftSyncNotice,
};

fn config_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config dir: {err}"))
}

/// 锁内 load → mutate → save；mutate 返回写入前的快照值供调用方回传。
fn with_locked_doc<T>(
    app: &AppHandle,
    mutate: impl FnOnce(&mut UserWorkGroups) -> Result<T, String>,
) -> Result<T, String> {
    let _guard = lock_document()?;
    let dir = config_dir(app)?;
    let mut document = load_from_path(&dir)?;
    let out = mutate(&mut document)?;
    save_to_path(&dir, &document)?;
    Ok(out)
}

#[tauri::command]
pub fn work_groups_get(app: AppHandle) -> Result<UserWorkGroups, String> {
    let _guard = lock_document()?;
    let dir = config_dir(&app)?;
    load_from_path(&dir)
}

/// FIL-68：固定标签快照存于 work-groups.json 顶层（与工作组打开标签同存储级别），
/// 不归属任何工作组；由前端在固定/取消固定/切换/落盘时写入。
#[tauri::command]
pub fn work_groups_set_pinned_tab(
    app: AppHandle,
    pinned: Option<PinnedTab>,
) -> Result<UserWorkGroups, String> {
    with_locked_doc(&app, |document| {
        document.pinned_tab = pinned;
        Ok(document.clone())
    })
}

#[tauri::command]
pub fn work_groups_create(
    app: AppHandle,
    name: Option<String>,
    live_open_state: Option<WorkGroupOpenState>,
) -> Result<SilentSwitchResult, String> {
    with_locked_doc(&app, |document| {
        let (mut result, _created) =
            create_and_activate_work_group(document, name, live_open_state.as_ref())?;
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
        *document = result.document.clone();
        Ok(result)
    })
}

#[tauri::command]
pub fn work_groups_add_member(
    app: AppHandle,
    work_group_id: String,
    member_path: String,
    live_open_state: Option<WorkGroupOpenState>,
) -> Result<(UserWorkGroups, WorkGroupOpenState, Option<WorkGroupSoftSyncNotice>), String> {
    with_locked_doc(&app, |document| {
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
        Ok((document.clone(), next_open, notice))
    })
}

#[tauri::command]
pub fn work_groups_remove_member(
    app: AppHandle,
    work_group_id: String,
    member_path: String,
) -> Result<UserWorkGroups, String> {
    with_locked_doc(&app, |document| {
        let index = document
            .work_groups
            .iter()
            .position(|group| group.id == work_group_id)
            .ok_or_else(|| format!("Unknown Work Group: {work_group_id}"))?;
        document.work_groups[index] =
            remove_member_from_hard_config(&document.work_groups[index], &member_path)?;
        Ok(document.clone())
    })
}

#[tauri::command]
pub fn work_groups_rename(
    app: AppHandle,
    work_group_id: String,
    name: String,
) -> Result<UserWorkGroups, String> {
    with_locked_doc(&app, |document| {
        document.work_groups =
            rename_work_group_in_list(&document.work_groups, &work_group_id, &name)?;
        Ok(document.clone())
    })
}

/// Copy a Work Group (members + openState with fresh tab ids). Does not change active.
#[tauri::command]
pub fn work_groups_copy(
    app: AppHandle,
    work_group_id: String,
) -> Result<(UserWorkGroups, WorkGroup), String> {
    with_locked_doc(&app, |document| {
        let (next, created) = copy_work_group_in_list(document, &work_group_id)?;
        *document = next;
        Ok((document.clone(), created))
    })
}

#[tauri::command]
pub fn work_groups_delete(
    app: AppHandle,
    work_group_id: String,
) -> Result<UserWorkGroups, String> {
    with_locked_doc(&app, |document| {
        let next = delete_work_group_in_list(document, &work_group_id)?;
        *document = next;
        Ok(document.clone())
    })
}

/// 拖动调整工作组列表顺序并写入 work-groups.json。
#[tauri::command]
pub fn work_groups_reorder(
    app: AppHandle,
    from_id: String,
    to_id: String,
) -> Result<UserWorkGroups, String> {
    with_locked_doc(&app, |document| {
        let next = reorder_work_groups_in_list(document, &from_id, &to_id)?;
        *document = next;
        Ok(document.clone())
    })
}

#[tauri::command]
pub fn work_groups_silent_switch(
    app: AppHandle,
    target_work_group_id: String,
    live_open_state: Option<WorkGroupOpenState>,
) -> Result<SilentSwitchResult, String> {
    with_locked_doc(&app, |document| {
        let mut result =
            silent_switch_work_group(document, &target_work_group_id, live_open_state.as_ref())?;
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
        *document = result.document.clone();
        Ok(result)
    })
}

#[tauri::command]
pub fn work_groups_save_open_state(
    app: AppHandle,
    work_group_id: String,
    live_open_state: Option<WorkGroupOpenState>,
) -> Result<UserWorkGroups, String> {
    with_locked_doc(&app, |document| {
        let index = document
            .work_groups
            .iter()
            .position(|group| group.id == work_group_id)
            .ok_or_else(|| format!("Unknown Work Group: {work_group_id}"))?;
        document.work_groups[index] =
            save_runtime_open_state_to_hard(&document.work_groups[index], live_open_state.as_ref());
        Ok(document.clone())
    })
}

#[tauri::command]
pub fn work_groups_save_members_from_open_state(
    app: AppHandle,
    work_group_id: String,
    live_open_state: Option<WorkGroupOpenState>,
) -> Result<UserWorkGroups, String> {
    with_locked_doc(&app, |document| {
        let index = document
            .work_groups
            .iter()
            .position(|group| group.id == work_group_id)
            .ok_or_else(|| format!("Unknown Work Group: {work_group_id}"))?;
        document.work_groups[index] = save_members_from_open_state_to_hard(
            &document.work_groups[index],
            live_open_state.as_ref(),
        );
        Ok(document.clone())
    })
}
