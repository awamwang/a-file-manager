// SPDX-License-Identifier: GPL-3.0-or-later

use super::host::ExtensionHost;
use super::types::{
    ExtensionHostStatus, ExtensionManifestVNext, FsProviderContribution, InstallResult,
    InstalledExtensionRecord, MarketplaceCatalog, UiCommandRequest, UiCommandResult,
};
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
