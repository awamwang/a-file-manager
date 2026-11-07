// SPDX-License-Identifier: GPL-3.0-or-later
//! AIA-261/264: outbound drag must not raise AFM over the drop target.
//!
//! Explorer parity for "drag from a background window onto the foreground one":
//! 1. `WM_MOUSEACTIVATE` → `MA_NOACTIVATE` when another top-level window is already
//!    foreground (mousedown must not cover the drop target).
//! 2. Remember that prior FG + press point; while the button is still down (or a
//!    drag gesture / OLE is active), undo any WebView/Chromium raise via
//!    `WM_ACTIVATE` + a short poll timer (`SM_CXDRAG` / `SM_CYDRAG`).
//! 3. Plain click (mouse-up without drag/OLE) still activates via `WM_PARENTNOTIFY`.
//!
//! Complements the patched `drag` crate, which prefers press-time prior FG when
//! Hybrid leave/blur → `DoDragDrop` runs after the press already raised AFM.

#![cfg(windows)]

use tauri::{AppHandle, Manager, Runtime};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, GetSystemMetrics, IsChild, KillTimer, SetForegroundWindow,
    SetTimer, MA_NOACTIVATE, SM_CXDRAG, SM_CYDRAG, WA_INACTIVE, WM_ACTIVATE, WM_LBUTTONUP,
    WM_MOUSEACTIVATE, WM_PARENTNOTIFY, WM_TIMER,
};

const SUBCLASS_ID: usize = 0xA_F1_26_01; // AFM + AIA-261
const POLL_TIMER_ID: usize = 0xA_F1_26_04;
const POLL_INTERVAL_MS: u32 = 16;

fn hwnd_is_ours(our_root: HWND, candidate: HWND) -> bool {
    if candidate.0.is_null() || our_root.0.is_null() {
        return false;
    }
    if candidate == our_root {
        return true;
    }
    unsafe { IsChild(our_root, candidate).as_bool() }
}

fn hwnd_to_isize(hwnd: HWND) -> isize {
    hwnd.0 as isize
}

fn isize_to_hwnd(raw: isize) -> HWND {
    HWND(raw as *mut _)
}

fn left_button_down() -> bool {
    unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 }
}

fn moved_past_drag_threshold() -> bool {
    let (ox, oy) = drag::press_origin();
    let mut pt = windows::Win32::Foundation::POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut pt) }.is_err() {
        return false;
    }
    let cx = unsafe { GetSystemMetrics(SM_CXDRAG) }.abs().max(1);
    let cy = unsafe { GetSystemMetrics(SM_CYDRAG) }.abs().max(1);
    (pt.x - ox).abs() >= cx || (pt.y - oy).abs() >= cy
}

fn restore_press_prior_if_we_raised(our_root: HWND) {
    let prior = drag::press_prior_foreground();
    if prior == 0 {
        return;
    }
    let prev = isize_to_hwnd(prior);
    if hwnd_is_ours(our_root, prev) {
        return;
    }
    let fg = unsafe { GetForegroundWindow() };
    if hwnd_is_ours(our_root, fg) {
        let _ = unsafe { SetForegroundWindow(prev) };
    }
}

fn arm_poll_timer(hwnd: HWND) {
    let _ = unsafe { SetTimer(Some(hwnd), POLL_TIMER_ID, POLL_INTERVAL_MS, None) };
}

fn disarm_poll_timer(hwnd: HWND) {
    let _ = unsafe { KillTimer(Some(hwnd), POLL_TIMER_ID) };
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
                let mut pt = windows::Win32::Foundation::POINT { x: 0, y: 0 };
                let _ = unsafe { GetCursorPos(&mut pt) };
                drag::note_deferred_activate(hwnd_to_isize(fg), pt.x, pt.y);
                arm_poll_timer(hwnd);
                return LRESULT(MA_NOACTIVATE as isize);
            }
        }
        WM_ACTIVATE => {
            let active = (wparam.0 as u32) & 0xffff;
            if active != WA_INACTIVE
                && drag::should_suppress_foreground_raise()
                && (left_button_down() || drag::drag_gesture_active() || drag::ole_drag_in_progress())
            {
                // WebView/Chromium often raises us on the same press — put prior FG back.
                restore_press_prior_if_we_raised(hwnd);
            }
        }
        WM_TIMER => {
            if wparam.0 == POLL_TIMER_ID {
                if !drag::should_suppress_foreground_raise() {
                    disarm_poll_timer(hwnd);
                    return LRESULT(0);
                }
                if left_button_down() {
                    if !drag::drag_gesture_active() && moved_past_drag_threshold() {
                        drag::note_drag_gesture();
                    }
                    restore_press_prior_if_we_raised(hwnd);
                }
                // Mouse-up is handled in WM_PARENTNOTIFY; keep timer until then / OLE clear.
                return LRESULT(0);
            }
        }
        WM_PARENTNOTIFY => {
            let child_msg = (wparam.0 as u32) & 0xffff;
            if child_msg == WM_LBUTTONUP {
                disarm_poll_timer(hwnd);
                let should_activate = drag::take_pending_activate()
                    && !drag::ole_drag_in_progress()
                    && !drag::drag_gesture_active();
                drag::clear_press_state();
                if should_activate {
                    // Plain click (no drag / OLE outbound): activate like a normal focus click.
                    let _ = unsafe { SetForegroundWindow(hwnd) };
                }
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
        eprintln!("AIA-264: SetWindowSubclass for drag focus failed");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    // Shared process atomics in `drag` — serialize host tests that touch them.
    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn deferred_activate_and_drag_gesture_helpers() {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        drag::clear_press_state();
        drag::note_deferred_activate(0x42, 1, 2);
        assert_eq!(drag::press_prior_foreground(), 0x42);
        assert_eq!(drag::press_origin(), (1, 2));
        assert!(drag::take_pending_activate());
        assert!(!drag::take_pending_activate());
        assert!(!drag::ole_drag_in_progress());

        drag::note_deferred_activate(0x42, 0, 0);
        drag::note_drag_gesture();
        assert!(!drag::take_pending_activate());
        assert!(drag::drag_gesture_active());
        assert_eq!(
            drag::resolve_preserve_foreground(0x99, 0x99),
            Some(0x42)
        );
        drag::clear_press_state();
    }
}
