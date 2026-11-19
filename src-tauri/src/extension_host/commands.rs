// SPDX-License-Identifier: GPL-3.0-or-later

use super::host::ExtensionHost;
use super::types::{
    AggregatedContributions, ExtensionHostStatus, InstallablePack, InstalledPackRecord,
    PackInstallResult,
};
use std::path::Path;
use std::sync::OnceLock;

fn host() -> &'static ExtensionHost {
    static HOST: OnceLock<ExtensionHost> = OnceLock::new();
    HOST.get_or_init(ExtensionHost::new)
}

#[tauri::command]
pub fn extension_host_status() -> Result<ExtensionHostStatus, String> {
    host().status()
}

/// 启动时从 APPDATA 加载本机 Feature Pack。
pub fn load_from_disk(app_data_dir: &Path) -> Result<(), String> {
    host().load_from_disk(app_data_dir)
}

/// zip 解压与目录拷贝不得占用 Tauri 同步命令线程。
#[tauri::command]
pub async fn extension_host_install_pack(
    source_path: String,
) -> Result<PackInstallResult, String> {
    tauri::async_runtime::spawn_blocking(move || host().install_pack(Path::new(&source_path)))
        .await
        .map_err(|err| format!("extension_host_install_pack worker failed: {err}"))
}

/// feed「可安装 Store」列表（含本机已装标记）。fetch 走 spawn_blocking（网络）。
#[tauri::command]
pub async fn extension_host_list_installable() -> Result<Vec<InstallablePack>, String> {
    tauri::async_runtime::spawn_blocking(|| host().list_installable_packs())
        .await
        .map_err(|err| format!("extension_host_list_installable worker failed: {err}"))?
}

/// 从 feed 下载并安装指定 pack（zip → SHA-256 校验 → 复用本地安装路径）。spawn_blocking。
#[tauri::command]
pub async fn extension_host_install_pack_from_feed(
    pack_id: String,
) -> Result<PackInstallResult, String> {
    tauri::async_runtime::spawn_blocking(move || host().install_pack_from_feed(&pack_id))
        .await
        .map_err(|err| format!("extension_host_install_pack_from_feed worker failed: {err}"))?
}

#[tauri::command]
pub fn extension_host_list_packs() -> Result<Vec<InstalledPackRecord>, String> {
    host().list_packs()
}

#[tauri::command]
pub fn extension_host_set_pack_enabled(id: String, enabled: bool) -> Result<(), String> {
    host().set_pack_enabled(&id, enabled)
}

/// 删除 Pack 目录不得占用 Tauri 同步命令线程。
#[tauri::command]
pub async fn extension_host_uninstall_pack(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || host().uninstall_pack(&id))
        .await
        .map_err(|err| format!("extension_host_uninstall_pack worker failed: {err}"))?
}

#[tauri::command]
pub fn extension_host_list_contributions() -> Result<AggregatedContributions, String> {
    host().list_contributions()
}
