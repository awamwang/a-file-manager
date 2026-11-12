// SPDX-License-Identifier: GPL-3.0-or-later

use super::host::ExtensionHost;
use super::types::{
    AggregatedContributions, ExtensionHostStatus, ExtensionManifestVNext, FsProviderContribution,
    InstallResult, InstalledExtensionRecord, InstalledPackRecord, MarketplaceCatalog,
    PackInstallResult, UiCommandRequest, UiCommandResult,
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

#[tauri::command]
pub fn extension_host_marketplace() -> Result<MarketplaceCatalog, String> {
    Ok(host().marketplace_catalog())
}

#[tauri::command]
pub fn extension_host_list_installed() -> Result<Vec<InstalledExtensionRecord>, String> {
    host().list_installed()
}

#[tauri::command]
pub fn extension_host_install(manifest: ExtensionManifestVNext) -> Result<InstallResult, String> {
    Ok(host().install_from_manifest(manifest))
}

#[tauri::command]
pub fn extension_host_set_enabled(
    extension_id: String,
    enabled: bool,
) -> Result<InstalledExtensionRecord, String> {
    host().set_enabled(&extension_id, enabled)
}

#[tauri::command]
pub fn extension_host_uninstall(extension_id: String) -> Result<bool, String> {
    host().uninstall(&extension_id)
}

#[tauri::command]
pub fn extension_host_dispatch_ui_command(
    request: UiCommandRequest,
) -> Result<UiCommandResult, String> {
    host().dispatch_ui_command(request)
}

#[tauri::command]
pub fn extension_host_list_fs_provider_hooks() -> Result<Vec<FsProviderContribution>, String> {
    host().list_fs_provider_hooks()
}

#[tauri::command]
pub fn extension_host_register_fs_provider_hook(
    extension_id: String,
    backend_id: String,
) -> Result<FsProviderContribution, String> {
    host().register_fs_provider_hook(&extension_id, &backend_id)
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
