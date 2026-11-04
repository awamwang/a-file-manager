// SPDX-License-Identifier: GPL-3.0-or-later

mod autostart;
mod discovery;
mod extension_host;
mod favorites;
mod file_ops;
mod navigator;
mod shell_icons;
mod open_with;
mod projects;
mod providers;
mod settings;
mod shell_menu;
mod system_tray;
mod terminal;
pub mod sigma_import;
mod snap_chrome;
mod work_groups;

fn launched_from_autostart(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == autostart::AFM_AUTOSTART_CLI_FLAG)
}

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

    let raw_args: Vec<String> = std::env::args().collect();
    let from_autostart = launched_from_autostart(&raw_args);
    let add_favorite_path = favorites::take_add_favorite_path(&raw_args);

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
        .setup(move |app| {
            if let Some(window) = app.get_webview_window("main") {
                window.create_overlay_titlebar_with_height(snap_chrome::TITLEBAR_HEIGHT_PX)?;
            }

            if let Err(err) = system_tray::setup_system_tray(app.handle()) {
                eprintln!("system tray setup failed: {err}");
            }

            // Hide to tray only when launched via OS login autostart AND user opted in.
            let hide_on_autostart =
                from_autostart && settings::read_bool_setting(app.handle(), "core.launchAtStartupHidden");
            if hide_on_autostart {
                system_tray::hide_main_window(app.handle());
            }

            discovery::sync_blacklist_from_settings(app.handle());

            if let Some(path) = add_favorite_path.clone() {
                match favorites::apply_add_favorite_cli(app.handle(), &path) {
                    Ok(added) => eprintln!(
                        "AFM favorites shell verb: {} ({path})",
                        if added { "added" } else { "already present" }
                    ),
                    Err(err) => eprintln!("AFM favorites shell verb failed: {err}"),
                }
            }

            Ok(())
        })
        .on_menu_event(|app, event| {
            system_tray::handle_menu_event(app, event);
        })
        .invoke_handler(tauri::generate_handler![
            health_get,
            autostart::autostart_status,
            autostart::autostart_set_enabled,
            navigator::navigator_list,
            navigator::navigator_default_path,
            navigator::navigator_parent,
            file_ops::file_ops_copy,
            file_ops::file_ops_move,
            file_ops::file_ops_rename,
            file_ops::file_ops_create_dir,
            file_ops::file_ops_delete,
            file_ops::file_ops_can_recycle,
            shell_icons::shell_icon_get,
            shell_icons::shell_icon_get_many,
            settings::settings_schema_get,
            settings::settings_get,
            settings::settings_set,
            discovery::discovery_status,
            discovery::discovery_set_fallback_enabled,
            discovery::discovery_search,
            discovery::discovery_set_blacklist,
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
            favorites::favorites_os_hook_set_enabled,
            open_with::open_native_open_with_dialog,
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
