// SPDX-License-Identifier: GPL-3.0-or-later
//! System tray for restore-after-autostart-hide (AIA-60).

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

fn build_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let version = app.package_info().version.to_string();
    let name = app.package_info().name.to_string();
    let title = format!("{name} – v{version}");

    let title_item = MenuItem::with_id(app, "title", &title, false, None::<&str>)?;
    let show_item = MenuItem::with_id(app, "show_main_window", "Open window", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &title_item,
            &PredefinedMenuItem::separator(app)?,
            &show_item,
            &PredefinedMenuItem::separator(app)?,
            &quit_item,
        ],
    )
}

pub fn setup_system_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let version = app.package_info().version.to_string();
    let name = app.package_info().name.to_string();
    let tooltip = format!("{name} – v{version}");
    let menu = build_tray_menu(app)?;
    let Some(icon) = app.default_window_icon().cloned() else {
        eprintln!("system tray skipped: no default window icon");
        return Ok(());
    };

    let _tray = TrayIconBuilder::with_id("main")
        .tooltip(&tooltip)
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            on_system_tray_event(tray.app_handle(), event);
        })
        .build(app)?;

    Ok(())
}

pub fn on_system_tray_event<R: Runtime>(app: &AppHandle<R>, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        focus_main_window(app);
    }
}

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "show_main_window" => show_main_window(app),
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn focus_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_minimized().unwrap_or(false) {
            let _ = window.unminimize();
        }
        show_main_window(app);
    }
}

pub fn hide_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}
