// SPDX-License-Identifier: GPL-3.0-or-later
//! Thin Kernel Navigator — OS path listing only (S2).

mod list;
mod places;
mod stat;
pub(crate) mod path_guard;

#[tauri::command]
pub fn navigator_list(path: String) -> Result<list::NavigatorListing, String> {
    list::list_directory(&path)
}

#[tauri::command]
pub fn navigator_default_path() -> Result<String, String> {
    list::default_path()
}

#[tauri::command]
pub fn navigator_parent(path: String) -> Result<Option<String>, String> {
    list::parent_path(&path)
}

#[tauri::command]
pub fn navigator_stat(path: String) -> Result<stat::NavigatorEntryStat, String> {
    stat::stat_path(&path)
}

#[tauri::command]
pub fn navigator_list_drives() -> Result<Vec<places::DriveInfo>, String> {
    places::list_drives()
}

#[tauri::command]
pub fn navigator_user_folders() -> Result<places::UserFolders, String> {
    places::user_folders()
}
