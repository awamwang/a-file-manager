// SPDX-License-Identifier: GPL-3.0-or-later
//! Thin Kernel Navigator — OS path listing only (S2).

mod attrs;
mod disk_space;
mod drill;
mod env_path;
mod list;
mod places;
mod search;
mod stat;
pub(crate) mod path_guard;

/// FIL-54: FS / COM / `net use` 不得占用 Tauri 同步命令线程；慢路径（UNC / 映射盘 / OneDrive）走 blocking 池。
#[tauri::command]
pub async fn navigator_list(
    path: String,
    show_hidden: Option<bool>,
) -> Result<list::NavigatorListing, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let t0 = std::time::Instant::now();
        let result = list::list_directory(&path, show_hidden.unwrap_or(false));
        match &result {
            Ok(listing) => crate::perf_log::afm_perf_line(format!(
                "AFM_PERF navigator_list n={} {:.1}ms path={}",
                listing.entries.len(),
                t0.elapsed().as_secs_f64() * 1000.0,
                path
            )),
            Err(err) => crate::perf_log::afm_perf_line(format!(
                "AFM_PERF navigator_list ERR {:.1}ms path={} err={}",
                t0.elapsed().as_secs_f64() * 1000.0,
                path,
                err
            )),
        }
        result
    })
    .await
    .map_err(|err| format!("navigator_list worker failed: {err}"))?
}

#[tauri::command]
pub async fn navigator_search(
    path: String,
    query: String,
    show_hidden: Option<bool>,
) -> Result<search::NavigatorSearch, String> {
    tauri::async_runtime::spawn_blocking(move || {
        search::search_directory(&path, &query, show_hidden.unwrap_or(false))
    })
    .await
    .map_err(|err| format!("navigator_search worker failed: {err}"))?
}

/// ADR 0031: drill-through listing — flatten the tree under `path` up to
/// `depth` levels of subfolders. The drill root stays put while the user
/// walks inside the tree, so this is asked once per (root, depth) pair.
#[tauri::command]
pub async fn navigator_drill_list(
    path: String,
    depth: u32,
    show_hidden: Option<bool>,
    limit: Option<usize>,
) -> Result<drill::DrillListing, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let t0 = std::time::Instant::now();
        let result = drill::drill_directory(
            &path,
            depth,
            show_hidden.unwrap_or(false),
            limit.unwrap_or(drill::DEFAULT_DRILL_LIMIT),
        );
        match &result {
            Ok(listing) => crate::perf_log::afm_perf_line(format!(
                "AFM_PERF navigator_drill_list n={} {:.1}ms depth={} path={}",
                listing.entries.len(),
                t0.elapsed().as_secs_f64() * 1000.0,
                depth,
                path
            )),
            Err(err) => crate::perf_log::afm_perf_line(format!(
                "AFM_PERF navigator_drill_list ERR {:.1}ms depth={} path={} err={}",
                t0.elapsed().as_secs_f64() * 1000.0,
                depth,
                path,
                err
            )),
        }
        result
    })
    .await
    .map_err(|err| format!("navigator_drill_list worker failed: {err}"))?
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
pub async fn navigator_stat(path: String) -> Result<stat::NavigatorEntryStat, String> {
    tauri::async_runtime::spawn_blocking(move || stat::stat_path(&path))
        .await
        .map_err(|err| format!("navigator_stat worker failed: {err}"))?
}

#[tauri::command]
pub async fn navigator_get_attributes(path: String) -> Result<attrs::EntryAttributes, String> {
    tauri::async_runtime::spawn_blocking(move || attrs::read_attributes(&path))
        .await
        .map_err(|err| format!("navigator_get_attributes worker failed: {err}"))?
}

#[tauri::command]
pub async fn navigator_set_attributes(
    path: String,
    readonly: bool,
    hidden: bool,
) -> Result<attrs::EntryAttributes, String> {
    tauri::async_runtime::spawn_blocking(move || {
        attrs::set_attributes(
            &path,
            attrs::EntryAttributes { readonly, hidden },
        )
    })
    .await
    .map_err(|err| format!("navigator_set_attributes worker failed: {err}"))?
}

#[tauri::command]
pub async fn navigator_list_drives() -> Result<Vec<places::DriveInfo>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let t0 = std::time::Instant::now();
        let result = places::list_drives();
        match &result {
            Ok(drives) => crate::perf_log::afm_perf_line(format!(
                "AFM_PERF navigator_list_drives n={} {:.1}ms",
                drives.len(),
                t0.elapsed().as_secs_f64() * 1000.0
            )),
            Err(err) => crate::perf_log::afm_perf_line(format!(
                "AFM_PERF navigator_list_drives ERR {:.1}ms err={}",
                t0.elapsed().as_secs_f64() * 1000.0,
                err
            )),
        }
        result
    })
    .await
    .map_err(|err| format!("navigator_list_drives worker failed: {err}"))?
}

#[tauri::command]
pub async fn navigator_user_folders() -> Result<places::UserFolders, String> {
    tauri::async_runtime::spawn_blocking(places::user_folders)
        .await
        .map_err(|err| format!("navigator_user_folders worker failed: {err}"))?
}

/// ADR 0026: mapped + best-effort LAN-discovered network shares for the This PC
/// → Network tree node and Guide cards. Runs on the blocking pool — `net view`
/// / `mount` discovery touches the network, never the UI thread.
#[tauri::command]
pub async fn navigator_network_shares() -> Result<Vec<places::NetworkPlace>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let t0 = std::time::Instant::now();
        let places = places::network_shares();
        crate::perf_log::afm_perf_line(format!(
            "AFM_PERF navigator_network_shares n={} {:.1}ms",
            places.len(),
            t0.elapsed().as_secs_f64() * 1000.0
        ));
        Ok(places)
    })
    .await
    .map_err(|err| format!("navigator_network_shares worker failed: {err}"))?
}

#[tauri::command]
pub async fn navigator_shell_places() -> Result<places::ShellPlaces, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let t0 = std::time::Instant::now();
        let result = places::shell_places();
        match &result {
            Ok(places) => crate::perf_log::afm_perf_line(format!(
                "AFM_PERF navigator_shell_places libs={} net={} {:.1}ms",
                places.libraries.len(),
                places.network.len(),
                t0.elapsed().as_secs_f64() * 1000.0
            )),
            Err(err) => crate::perf_log::afm_perf_line(format!(
                "AFM_PERF navigator_shell_places ERR {:.1}ms err={}",
                t0.elapsed().as_secs_f64() * 1000.0,
                err
            )),
        }
        result
    })
    .await
    .map_err(|err| format!("navigator_shell_places worker failed: {err}"))?
}

#[tauri::command]
pub async fn navigator_disk_space(path: String) -> Result<disk_space::DiskSpaceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || disk_space::disk_space_for_path(&path))
        .await
        .map_err(|err| format!("navigator_disk_space worker failed: {err}"))?
}

#[tauri::command]
pub fn navigator_expand_path(path: String) -> String {
    env_path::expand_env_path(&path)
}
