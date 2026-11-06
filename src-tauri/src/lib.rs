// SPDX-License-Identifier: GPL-3.0-or-later

mod archive;
mod autostart;
mod discovery;
mod extension_host;
mod favorites;
mod visit_stats;
mod file_ops;
mod navigator;
mod preview;
mod preview_bridge;
mod shell_icons;
mod ide_launch;
mod open_with;
mod projects;
mod providers;
mod settings;
mod shell_menu;
mod system_tray;
mod terminal;
pub mod sigma_import;
mod snap_chrome;
mod win_process;
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

/// Quit the process (tray Quit + close-confirm Quit share this path — AIA-86).
#[tauri::command]
fn app_quit(app: tauri::AppHandle) {
    system_tray::destroy_system_tray(&app);
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager;
    use tauri_plugin_frame::{FramePluginBuilder, WebviewWindowExt};

    let raw_args: Vec<String> = std::env::args().collect();
    let from_autostart = launched_from_autostart(&raw_args);
    let add_favorite_path = favorites::take_add_favorite_path(&raw_args);

    // GUI single-instance: avoid stacked tray icons from a second process.
    // Shell-verb "add favorite" is allowed through so it can still mutate favorites.
    let primary_instance = system_tray::try_acquire_single_instance_lock();
    if !primary_instance && add_favorite_path.is_none() {
        eprintln!("A File Manager is already running (single-instance); exiting.");
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Native OLE/file drag-out (CF_HDROP) for dropping onto players / Explorer / etc.
        .plugin(tauri_plugin_drag::init())
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
            // Taskbar icon: set explicitly from bundled AFM PNG (PE resource can lag after icon swaps).
            let app_icon = system_tray::load_app_icon(app.handle());
            if let Some(window) = app.get_webview_window("main") {
                window.create_overlay_titlebar_with_height(snap_chrome::TITLEBAR_HEIGHT_PX)?;
                if let Some(icon) = app_icon.clone() {
                    if let Err(err) = window.set_icon(icon) {
                        eprintln!("window set_icon failed: {err}");
                    }
                }
            }

            // Only the primary instance owns the tray icon.
            if primary_instance {
                if let Err(err) = system_tray::setup_system_tray(app.handle()) {
                    eprintln!("system tray setup failed: {err}");
                }
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
                // Secondary process was only needed for the shell verb — exit after applying.
                if !primary_instance {
                    app.handle().exit(0);
                }
            }

            Ok(())
        })
        .on_menu_event(|app, event| {
            system_tray::handle_menu_event(app, event);
        })
        .invoke_handler(tauri::generate_handler![
            health_get,
            app_quit,
            autostart::autostart_status,
            autostart::autostart_set_enabled,
            navigator::navigator_list,
            navigator::navigator_search,
            navigator::navigator_default_path,
            navigator::navigator_parent,
            navigator::navigator_stat,
            navigator::navigator_list_drives,
            navigator::navigator_user_folders,
            ide_launch::ide_list_apps,
            ide_launch::ide_open_path,
            file_ops::file_ops_name_exists,
            file_ops::file_ops_copy,
            file_ops::file_ops_move,
            file_ops::file_ops_rename,
            file_ops::file_ops_create_dir,
            file_ops::file_ops_create_file,
            file_ops::file_ops_delete,
            file_ops::file_ops_delete_permanent,
            file_ops::file_ops_can_recycle,
            file_ops::clipboard::file_ops_clipboard_set,
            file_ops::clipboard::file_ops_clipboard_get,
            file_ops::clipboard::file_ops_clipboard_clear,
            shell_icons::shell_icon_get,
            shell_icons::shell_icon_get_many,
            settings::settings_schema_get,
            settings::settings_get,
            settings::settings_set,
            discovery::discovery_status,
            discovery::discovery_estimate_count,
            discovery::discovery_set_fallback_enabled,
            discovery::discovery_search,
            discovery::discovery_set_blacklist,
            providers::provider_backends,
            providers::provider_list,
            providers::provider_stat,
            providers::provider_resolve,
            providers::provider_preload,
            providers::provider_cache_invalidate,
            work_groups::work_groups_get,
            work_groups::work_groups_create,
            work_groups::work_groups_add_member,
            work_groups::work_groups_remove_member,
            work_groups::work_groups_rename,
            work_groups::work_groups_delete,
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
            terminal::terminal_list_apps,
            terminal::open_in_terminal,
            favorites::favorites_get,
            favorites::favorites_toggle,
            favorites::favorites_contains,
            favorites::favorites_os_hook_status,
            favorites::favorites_os_hook_set_enabled,
            visit_stats::visit_stats_get,
            visit_stats::visit_stats_record,
            visit_stats::visit_stats_quick_access,
            open_with::open_path_default,
            open_with::open_native_open_with_dialog,
            preview::preview_load,
            preview_bridge::preview_bridge_invoke,
            preview_bridge::preview_bridge_switch,
            archive::archive_compress_zip,
            archive::archive_extract_zip,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // Drop the NotifyIcon on graceful exit so `tauri dev` / HMR does not stack ghosts.
            if matches!(
                event,
                tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }
            ) {
                system_tray::destroy_system_tray(app_handle);
            }
        });
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
