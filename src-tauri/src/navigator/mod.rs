// SPDX-License-Identifier: GPL-3.0-or-later
//! Thin Kernel Navigator — OS path listing only (S2).

mod disk_space;
mod env_path;
mod list;
mod places;
mod search;
mod stat;
pub(crate) mod path_guard;

#[tauri::command]
pub fn navigator_list(path: String, show_hidden: Option<bool>) -> Result<list::NavigatorListing, String> {
    list::list_directory(&path, show_hidden.unwrap_or(false))
}

#[tauri::command]
pub fn navigator_search(
    path: String,
    query: String,
    show_hidden: Option<bool>,
) -> Result<search::NavigatorSearch, String> {
    search::search_directory(&path, &query, show_hidden.unwrap_or(false))
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

#[tauri::command]
pub fn navigator_shell_places() -> Result<places::ShellPlaces, String> {
    places::shell_places()
}

#[tauri::command]
pub fn navigator_disk_space(path: String) -> Result<disk_space::DiskSpaceInfo, String> {
    disk_space::disk_space_for_path(&path)
}

#[tauri::command]
pub fn navigator_expand_path(path: String) -> String {
    env_path::expand_env_path(&path)
}
