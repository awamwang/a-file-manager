// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows taskbar / window icon wiring (AIA-271).
//!
//! Tauri's `WebviewWindow::set_icon` only maps to tao `set_window_icon`, which
//! sends `WM_SETICON` with `ICON_SMALL`. The taskbar button on Windows 10/11
//! prefers `ICON_BIG` (`set_taskbar_icon` in tao), which Tauri never sets —
//! so a packaged exe can show the correct PE icon in Explorer while the
//! running window's taskbar slot stays blank.
//!
//! Fix: set process AppUserModelID to match `tauri.conf.json` `identifier`,
//! then `ExtractIconExW` the running exe and apply both `WM_SETICON` sizes
//! plus class icons.

#![cfg(windows)]

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::OnceLock;

use tauri::{AppHandle, Manager, Runtime};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::UI::Shell::{ExtractIconExW, SetCurrentProcessExplicitAppUserModelID};
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageW, SetClassLongPtrW, GCLP_HICON, GCLP_HICONSM, HICON, ICON_BIG, ICON_SMALL,
    WM_SETICON,
};

/// Must match `tauri.conf.json` `identifier` / NSIS shortcut AUMID.
pub const APP_USER_MODEL_ID: &str = "com.a-file-manager.app";

/// Keep ExtractIconEx handles alive for the process lifetime (window refs them).
/// Stored as raw `isize` so the static is `Sync` (HICON is a raw pointer newtype).
static ICON_HOLD: OnceLock<(isize, isize)> = OnceLock::new();

fn wide_null(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// Call before creating any HWND so the process identity matches Start Menu /
/// taskbar shortcuts created by the NSIS installer.
pub fn set_process_app_user_model_id() {
    let wide = wide_null(APP_USER_MODEL_ID);
    let result = unsafe { SetCurrentProcessExplicitAppUserModelID(PCWSTR(wide.as_ptr())) };
    if let Err(err) = result {
        eprintln!("SetCurrentProcessExplicitAppUserModelID failed: {err}");
    }
}

fn current_exe_wide() -> Option<Vec<u16>> {
    let mut buf = vec![0u16; 1024];
    let len = unsafe { GetModuleFileNameW(None, &mut buf) } as usize;
    if len == 0 || len >= buf.len() {
        return None;
    }
    buf.truncate(len);
    buf.push(0);
    Some(buf)
}

fn extract_exe_icons() -> Option<(isize, isize)> {
    if let Some(held) = ICON_HOLD.get() {
        return Some(*held);
    }
    let path = current_exe_wide()?;
    let mut big = HICON::default();
    let mut small = HICON::default();
    let count = unsafe {
        ExtractIconExW(
            PCWSTR(path.as_ptr()),
            0,
            Some(&mut big),
            Some(&mut small),
            1,
        )
    };
    if count == 0 || big.0.is_null() || small.0.is_null() {
        eprintln!("ExtractIconExW failed (count={count}) for AFM exe icons");
        return None;
    }
    let pair = (big.0 as isize, small.0 as isize);
    let _ = ICON_HOLD.set(pair);
    Some(pair)
}

fn set_window_icons(hwnd: HWND, big: isize, small: isize) {
    unsafe {
        let _ = SendMessageW(
            hwnd,
            WM_SETICON,
            Some(WPARAM(ICON_BIG as usize)),
            Some(LPARAM(big)),
        );
        let _ = SendMessageW(
            hwnd,
            WM_SETICON,
            Some(WPARAM(ICON_SMALL as usize)),
            Some(LPARAM(small)),
        );
        // Class icons cover Alt+Tab / switcher when per-window icons are cleared.
        let _ = SetClassLongPtrW(hwnd, GCLP_HICON, big);
        let _ = SetClassLongPtrW(hwnd, GCLP_HICONSM, small);
    }
}

/// Apply PE-embedded app icons to the main window (ICON_BIG + ICON_SMALL).
pub fn apply_main_window_icons<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        eprintln!("window icon skipped: main window missing");
        return;
    };
    let Ok(hwnd) = window.hwnd() else {
        eprintln!("window icon skipped: hwnd unavailable");
        return;
    };
    let Some((big, small)) = extract_exe_icons() else {
        return;
    };
    set_window_icons(hwnd, big, small);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn aumid_matches_tauri_identifier() {
        assert_eq!(APP_USER_MODEL_ID, "com.a-file-manager.app");
    }

    #[test]
    fn icon_ico_has_taskbar_sizes() {
        let ico = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/icon.ico");
        let data = std::fs::read(&ico).expect("icon.ico readable");
        assert!(data.len() > 22, "ico too small");
        let count = u16::from_le_bytes([data[4], data[5]]) as usize;
        assert!(count >= 4, "expected multi-size ico, got {count} entries");

        let mut sizes = Vec::new();
        for i in 0..count {
            let off = 6 + i * 16;
            let w = data[off];
            let h = data[off + 1];
            sizes.push((if w == 0 { 256 } else { w as u32 }, if h == 0 { 256 } else { h as u32 }));
        }
        for need in [16u32, 32, 48, 256] {
            assert!(
                sizes.iter().any(|(w, h)| *w == need && *h == need),
                "icon.ico missing {need}x{need}; have {sizes:?}"
            );
        }
    }

    #[test]
    fn extract_icons_from_built_exe() {
        use std::os::windows::ffi::OsStrExt;

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let candidates = [
            manifest.join("target/debug/a-file-manager.exe"),
            manifest.join("target/release/a-file-manager.exe"),
        ];
        let Some(exe) = candidates.into_iter().find(|p| p.is_file()) else {
            eprintln!("skip extract_icons_from_built_exe: no a-file-manager.exe built");
            return;
        };
        let wide: Vec<u16> = exe
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut big = HICON::default();
        let mut small = HICON::default();
        let count = unsafe {
            ExtractIconExW(
                PCWSTR(wide.as_ptr()),
                0,
                Some(&mut big),
                Some(&mut small),
                1,
            )
        };
        assert!(count > 0, "ExtractIconExW returned 0 for {}", exe.display());
        assert!(!big.0.is_null(), "large icon null");
        assert!(!small.0.is_null(), "small icon null");
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyIcon(big);
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyIcon(small);
        }
    }
}
