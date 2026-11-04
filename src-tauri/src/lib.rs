// SPDX-License-Identifier: GPL-3.0-or-later

mod discovery;
mod extension_host;
mod favorites;
mod navigator;
mod projects;
mod providers;
mod settings;
mod shell_menu;
mod terminal;
pub mod sigma_import;
mod snap_chrome;
mod work_groups;

use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthPayload {
    pub ok: bool,
    pub name: String,
    pub version: String,
    pub identifier: String,
}

#[tauri::command]
fn health_get() -> HealthPayload {
    HealthPayload {
        ok: true,
        name: "A File Manager".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        identifier: "com.a-file-manager.app".to_string(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager;
    use tauri_plugin_frame::{FramePluginBuilder, WebviewWindowExt};

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            FramePluginBuilder::new()
                // Keep Fluent self-drawn caption; plugin only installs HTMAXBUTTON overlay.
                .auto_titlebar(false)
                .snap_overlay(true)
                .titlebar_height(snap_chrome::TITLEBAR_HEIGHT_PX)
                .button_width(snap_chrome::BUTTON_WIDTH_PX)
                .build(),
        )
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.create_overlay_titlebar_with_height(snap_chrome::TITLEBAR_HEIGHT_PX)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health_get,
            navigator::navigator_list,
            navigator::navigator_default_path,
            navigator::navigator_parent,
            settings::settings_schema_get,
            settings::settings_get,
            settings::settings_set,
            discovery::discovery_status,
            discovery::discovery_set_fallback_enabled,
            discovery::discovery_search,
            providers::provider_backends,
            providers::provider_list,
            providers::provider_stat,
            providers::provider_resolve,
            work_groups::work_groups_get,
            work_groups::work_groups_create,
            work_groups::work_groups_add_member,
            work_groups::work_groups_silent_switch,
            work_groups::work_groups_save_open_state,
            work_groups::work_groups_save_members_from_open_state,
            projects::projects_get,
            projects::projects_create,
            projects::projects_rename,
            projects::projects_delete,
            projects::projects_set_active,
            projects::projects_activate,
            projects::projects_bind_work_group,
            projects::projects_promote_from_work_group,
            projects::projects_save_members_from_open_state,
            projects::projects_add_workspace_link,
            projects::projects_remove_workspace_link,
            projects::projects_set_context_notes,
            projects::projects_add_task,
            projects::projects_update_task,
            projects::projects_move_task,
            projects::projects_delete_task,
            projects::projects_open_all_links,
            extension_host::extension_host_status,
            extension_host::extension_host_marketplace,
            extension_host::extension_host_list_installed,
            extension_host::extension_host_install,
            extension_host::extension_host_set_enabled,
            extension_host::extension_host_uninstall,
            extension_host::extension_host_dispatch_ui_command,
            extension_host::extension_host_list_fs_provider_hooks,
            extension_host::extension_host_register_fs_provider_hook,
            shell_menu::shell_show_context_menu,
            terminal::open_in_terminal,
            favorites::favorites_get,
            favorites::favorites_toggle,
            favorites::favorites_contains,
            favorites::favorites_os_hook_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_get_returns_ok_payload() {
        let health = health_get();
        assert!(health.ok);
        assert_eq!(health.name, "A File Manager");
        assert_eq!(health.identifier, "com.a-file-manager.app");
        assert!(!health.version.is_empty());
    }
}
