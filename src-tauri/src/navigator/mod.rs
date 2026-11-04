// SPDX-License-Identifier: GPL-3.0-or-later
//! Thin Kernel Navigator — OS path listing only (S2).

mod list;
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
