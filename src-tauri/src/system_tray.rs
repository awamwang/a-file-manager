// SPDX-License-Identifier: GPL-3.0-or-later
//! System tray for restore-after-autostart-hide (AIA-60).
//! Single tray icon per process: replace-by-id on setup, destroy on exit (HMR-safe).
//! AIA-272: startup reveal gate — main window starts hidden; frontend shows after paint
//! unless launched autostart-hidden (tray restore only).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State,
};

const TRAY_ID: &str = "main";

/// Compile-time AFM icon (same asset as `tauri.conf.json` / `npx tauri icon` output).
/// Prefer this over `default_window_icon()` so tray/taskbar do not keep a stale PE embed.
const APP_ICON_PNG: &[u8] = include_bytes!("../icons/128x128.png");

/// Keeps the tray handle alive for the process lifetime (dropping it can remove the icon).
/// Pinned to the default Wry runtime so managed state matches `AppHandle` at `run` time.
pub struct TrayState(pub Mutex<Option<TrayIcon>>);

/// One-shot gate: frontend may reveal the main window after first paint (AIA-272).
/// `false` when OS login autostart + `core.launchAtStartupHidden` — stay in tray.
pub struct StartupReveal {
    should_reveal: AtomicBool,
}

impl StartupReveal {
    pub fn new(should_reveal: bool) -> Self {
        Self {
            should_reveal: AtomicBool::new(should_reveal),
        }
    }
}

/// Consume the startup reveal token. Returns true once; subsequent calls are false.
#[tauri::command]
pub fn afm_startup_should_reveal(state: State<'_, StartupReveal>) -> bool {
    state.should_reveal.swap(false, Ordering::SeqCst)
}

/// Load the bundled AFM app icon for tray + window (taskbar) use.
pub fn load_app_icon(app: &AppHandle) -> Option<Image<'_>> {
    match Image::from_bytes(APP_ICON_PNG) {
        Ok(icon) => Some(icon),
        Err(err) => {
            eprintln!("bundled app icon decode failed ({err}); falling back to default_window_icon");
            app.default_window_icon().cloned()
        }
    }
}

fn build_tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
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

/// Drop any existing `main` tray then create one. Prevents stacked icons on re-setup / HMR.
pub fn setup_system_tray(app: &AppHandle) -> tauri::Result<()> {
    destroy_system_tray(app);

    let version = app.package_info().version.to_string();
    let name = app.package_info().name.to_string();
    let tooltip = format!("{name} – v{version}");
    let menu = build_tray_menu(app)?;
    let Some(icon) = load_app_icon(app) else {
        eprintln!("system tray skipped: no app icon available");
        return Ok(());
    };

    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(&tooltip)
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            on_system_tray_event(tray.app_handle(), event);
        })
        .build(app)?;

    if let Some(state) = app.try_state::<TrayState>() {
        *state.0.lock().expect("tray state lock") = Some(tray);
    } else {
        app.manage(TrayState(Mutex::new(Some(tray))));
    }

    Ok(())
}

/// Remove the tray icon from the shell (call on process exit / before recreate).
pub fn destroy_system_tray(app: &AppHandle) {
    if let Some(state) = app.try_state::<TrayState>() {
        if let Ok(mut guard) = state.0.lock() {
            *guard = None;
        }
    }
    let _ = app.remove_tray_by_id(TRAY_ID);
}

pub fn on_system_tray_event(app: &AppHandle, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        focus_main_window(app);
    }
}

pub fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "show_main_window" => show_main_window(app),
        "quit" => {
            destroy_system_tray(app);
            app.exit(0);
        }
        _ => {}
    }
}

/// AIA-272 + FIL-72: show the main window and *activate* it from the host.
///
/// On a fresh install the process is launched by the NSIS installer and WebView2
/// cold-start takes seconds. By the time the frontend reveals the window the
/// process has lost Windows' startup foreground allowance, so a plain
/// `set_focus()` is refused: the maximized window appears but stays behind the
/// shell until the user clicks the taskbar icon again. Attaching to the current
/// foreground thread, with a transient topmost toggle as fallback, makes the
/// reveal land in the foreground. FIL-47 forbids raising *other* apps during
/// drag — this only ever raises our own main window.
#[cfg(windows)]
fn force_main_window_foreground(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId,
        SetForegroundWindow, SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE,
        SWP_NOSIZE, SWP_SHOWWINDOW,
    };

    unsafe {
        let fg = GetForegroundWindow();
        let fg_tid = GetWindowThreadProcessId(fg, None);
        let our_tid = GetCurrentThreadId();
        let attached = fg_tid != 0
            && fg_tid != our_tid
            && AttachThreadInput(our_tid, fg_tid, true).as_bool();

        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        if GetForegroundWindow() != hwnd {
            let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW;
            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, flags);
            let _ = SetWindowPos(hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags);
            let _ = SetForegroundWindow(hwnd);
        }
        if attached {
            let _ = AttachThreadInput(our_tid, fg_tid, false);
        }
    }
}

/// Startup reveal (FIL-72): one atomic host-side show + foreground raise.
#[tauri::command]
pub fn afm_reveal_main_window(app: AppHandle) {
    show_main_window(&app);
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(windows)]
        crate::drag_focus::begin_forced_activation();
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        #[cfg(windows)]
        if let Ok(hwnd) = window.hwnd() {
            force_main_window_foreground(hwnd);
        }
    }
}

pub fn focus_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_minimized().unwrap_or(false) {
            let _ = window.unminimize();
        }
        show_main_window(app);
    }
}

/// Hide main window (tray-only). Startup uses `visible: false` + reveal gate (AIA-272);
/// close-to-tray currently calls `Window.hide` from the frontend.
#[allow(dead_code)]
pub fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

/// Named mutex so a second GUI process does not stack another tray icon.
/// Returns false when another AFM instance already holds the lock.
#[cfg(windows)]
pub fn try_acquire_single_instance_lock() -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;

    // Keep the mutex handle for process lifetime (leaked on purpose).
    static mut HOLD: Option<windows::Win32::Foundation::HANDLE> = None;

    unsafe {
        match CreateMutexW(None, true, w!("Local\\com.a-file-manager.app.single")) {
            Ok(handle) => {
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    let _ = CloseHandle(handle);
                    false
                } else {
                    HOLD = Some(handle);
                    true
                }
            }
            Err(err) => {
                eprintln!("single-instance mutex failed ({err}); continuing");
                true
            }
        }
    }
}

#[cfg(not(windows))]
pub fn try_acquire_single_instance_lock() -> bool {
    true
}
