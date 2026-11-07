// SPDX-License-Identifier: GPL-3.0-or-later
//! AIA-261: outbound drag must not raise AFM over the drop target.
//!
//! Explorer parity for "drag from a background window onto the foreground one":
//! return `MA_NOACTIVATE` from `WM_MOUSEACTIVATE` when another top-level window is
//! already foreground, so the mouse-down that starts a file drag does not cover
//! the drop target. A normal click (no OLE drag) still activates on mouse-up via
//! `WM_PARENTNOTIFY`.
//!
//! Complements the patched `drag` crate, which restores the prior foreground
//! window during Hybrid leave/blur → `DoDragDrop`.

#![cfg(windows)]

use tauri::{AppHandle, Manager, Runtime};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, IsChild, SetForegroundWindow, MA_NOACTIVATE, WM_LBUTTONUP,
    WM_MOUSEACTIVATE, WM_PARENTNOTIFY,
};

const SUBCLASS_ID: usize = 0xA_F1_26_01; // AFM + AIA-261

fn hwnd_is_ours(our_root: HWND, candidate: HWND) -> bool {
    if candidate.0.is_null() || our_root.0.is_null() {
        return false;
    }
    if candidate == our_root {
        return true;
    }
    unsafe { IsChild(our_root, candidate).as_bool() }
}

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    match msg {
        WM_MOUSEACTIVATE => {
            let fg = unsafe { GetForegroundWindow() };
            if !hwnd_is_ours(hwnd, fg) {
                // Defer activation: keep the drop-target (or other) window in front
                // while the user may be starting an outbound file drag.
                drag::note_deferred_activate();
                return LRESULT(MA_NOACTIVATE as isize);
            }
        }
        WM_PARENTNOTIFY => {
            let child_msg = (wparam.0 as u32) & 0xffff;
            if child_msg == WM_LBUTTONUP
                && drag::take_pending_activate()
                && !drag::ole_drag_in_progress()
            {
                // Plain click (no OLE outbound): activate like a normal focus click.
                let _ = unsafe { SetForegroundWindow(hwnd) };
            }
        }
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

/// Install the no-activate-on-background-drag subclass on the main window.
pub fn install_for_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let ok = unsafe { SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, 0) };
    if !ok.as_bool() {
        eprintln!("AIA-261: SetWindowSubclass for drag focus failed");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn deferred_activate_helpers_roundtrip() {
        drag::note_deferred_activate();
        assert!(drag::take_pending_activate());
        assert!(!drag::take_pending_activate());
        assert!(!drag::ole_drag_in_progress());
    }
}
