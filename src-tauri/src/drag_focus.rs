// SPDX-License-Identifier: GPL-3.0-or-later
//! AIA-261/264/266 + FIL-37/39: outbound drag must not raise AFM over the drop target.
//!
//! Explorer parity for "drag from a background window onto the foreground one":
//! 1. `WM_MOUSEACTIVATE` → `MA_NOACTIVATE` when another top-level window is already
//!    foreground (mousedown must not cover the drop target).
//! 2. Also arm on `WM_PARENTNOTIFY`/`LBUTTONDOWN` if activate was missed.
//! 3. Remember that prior FG + press point; while the button is still down (or a
//!    drag gesture / OLE is active), undo any WebView/Chromium raise via
//!    `WM_ACTIVATE` + a short poll timer (`SM_CXDRAG` / `SM_CYDRAG`).
//! 4. Plain click (mouse-up without drag/OLE) still activates via `WM_PARENTNOTIFY`
//!    or the poll-timer fallback when WebView swallows parent notify.
//! 5. FIL-39: once past drag threshold and cursor is not over our root HWND tree
//!    (`WindowFromPoint`), emit `afm://dnd/ole-promote` so Hybrid can lift OLE
//!    before the external target only sees HTML5.
//!
//! Complements Navigator row `mousedown` (AIA-266/280: no `preventDefault` —
//! that cancels HTML5 dragstart) and the patched `drag` crate,
//! which prefers press-time prior FG when Hybrid leave/blur → `DoDragDrop`
//! runs after the press already raised AFM.
//!
//! ## FIL-37 升级闸（暂不启用）
//! 若 dogfood 仍抬窗盖住目标，再评估：
//! - **WebView 子 HWND subclass**：可拦截子进程路径上的二次抬窗，但增加宿主/子窗
//!   生命周期耦合与崩溃面；需 TZ。
//! - **`LockSetForegroundWindow`**：需已是 FG 才能锁，易误伤其它应用的前台切换；
//!   仅作有时限的最后手段，需 TZ。当前路径以 `MA_NOACTIVATE` + prior FG sticky
//!   `SetWindowPos`/`SetForegroundWindow` 为准。

#![cfg(windows)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use tauri::{AppHandle, Emitter, Manager, Runtime};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, GetSystemMetrics, IsChild, KillTimer, SetForegroundWindow,
    SetTimer, SetWindowPos, WindowFromPoint, HWND_TOP, MA_NOACTIVATE, SM_CXDRAG, SM_CYDRAG,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, WA_INACTIVE, WM_ACTIVATE,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_PARENTNOTIFY, WM_TIMER,
};

const SUBCLASS_ID: usize = 0xA_F1_26_01; // AFM + AIA-261
const POLL_TIMER_ID: usize = 0xA_F1_26_04;
const POLL_INTERVAL_MS: u32 = 16;

/// Hybrid 前端监听：拖势已成立且光标不在 AFM 根 HWND 树上时请求提升 OLE。
pub const OLE_PROMOTE_EVENT: &str = "afm://dnd/ole-promote";

static PROMOTE_EMIT: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();
static POLL_ARMED: AtomicBool = AtomicBool::new(false);
static PROMOTE_EMITTED: AtomicBool = AtomicBool::new(false);

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
    let mut pt = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut pt) }.is_err() {
        return false;
    }
    let cx = unsafe { GetSystemMetrics(SM_CXDRAG) }.abs().max(1);
    let cy = unsafe { GetSystemMetrics(SM_CYDRAG) }.abs().max(1);
    (pt.x - ox).abs() >= cx || (pt.y - oy).abs() >= cy
}

fn cursor_over_our_tree(our_root: HWND) -> bool {
    let mut pt = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut pt) }.is_err() {
        return true;
    }
    let hit = unsafe { WindowFromPoint(pt) };
    hwnd_is_ours(our_root, hit)
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
    // AIA-268: WebView may bump z-order without a stable FG steal — always
    // re-stack the press-time prior window, and re-foreground when we own FG.
    let _ = unsafe {
        SetWindowPos(
            prev,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
        )
    };
    if hwnd_is_ours(our_root, fg) {
        let _ = unsafe { SetForegroundWindow(prev) };
    }
}

fn request_ole_promote_if_needed(our_root: HWND) {
    if PROMOTE_EMITTED.load(Ordering::SeqCst) || drag::ole_drag_in_progress() {
        return;
    }
    if !drag::drag_gesture_active() {
        return;
    }
    if cursor_over_our_tree(our_root) {
        return;
    }
    PROMOTE_EMITTED.store(true, Ordering::SeqCst);
    if let Some(emit) = PROMOTE_EMIT.get() {
        emit();
    }
}

fn arm_poll_timer(hwnd: HWND) {
    POLL_ARMED.store(true, Ordering::SeqCst);
    let _ = unsafe { SetTimer(Some(hwnd), POLL_TIMER_ID, POLL_INTERVAL_MS, None) };
}

fn disarm_poll_timer(hwnd: HWND) {
    POLL_ARMED.store(false, Ordering::SeqCst);
    let _ = unsafe { KillTimer(Some(hwnd), POLL_TIMER_ID) };
}

/// 按下：后台则 MA_NOACTIVATE 推迟激活；前台也记录原点以便 Hybrid 提升时序。
fn note_press(hwnd: HWND) {
    let fg = unsafe { GetForegroundWindow() };
    let mut pt = POINT { x: 0, y: 0 };
    let _ = unsafe { GetCursorPos(&mut pt) };

    if drag::should_suppress_foreground_raise() {
        // Already armed for this press — do not reset origin mid-gesture.
        arm_poll_timer(hwnd);
        return;
    }

    if !hwnd_is_ours(hwnd, fg) {
        drag::note_deferred_activate(hwnd_to_isize(fg), pt.x, pt.y);
    } else {
        drag::note_press_origin(pt.x, pt.y);
    }
    PROMOTE_EMITTED.store(false, Ordering::SeqCst);
    arm_poll_timer(hwnd);
}

/// Mouse-up without drag/OLE: activate like a normal focus click.
fn finish_press_maybe_activate(hwnd: HWND) {
    disarm_poll_timer(hwnd);
    PROMOTE_EMITTED.store(false, Ordering::SeqCst);
    let should_activate = drag::take_pending_activate()
        && !drag::ole_drag_in_progress()
        && !drag::drag_gesture_active();
    drag::clear_press_state();
    if should_activate {
        let _ = unsafe { SetForegroundWindow(hwnd) };
    }
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
                note_press(hwnd);
                return LRESULT(MA_NOACTIVATE as isize);
            }
        }
        WM_ACTIVATE => {
            let active = (wparam.0 as u32) & 0xffff;
            if active != WA_INACTIVE
                && drag::should_suppress_foreground_raise()
                && (left_button_down()
                    || drag::drag_gesture_active()
                    || drag::ole_drag_in_progress())
            {
                // WebView/Chromium often raises us on the same press — put prior FG back.
                restore_press_prior_if_we_raised(hwnd);
            }
        }
        WM_TIMER => {
            if wparam.0 == POLL_TIMER_ID {
                if !POLL_ARMED.load(Ordering::SeqCst)
                    && !drag::should_suppress_foreground_raise()
                    && !drag::drag_gesture_active()
                {
                    disarm_poll_timer(hwnd);
                    return LRESULT(0);
                }
                if left_button_down() {
                    if !drag::drag_gesture_active() && moved_past_drag_threshold() {
                        drag::note_drag_gesture();
                    }
                    if drag::should_suppress_foreground_raise() {
                        restore_press_prior_if_we_raised(hwnd);
                    }
                    request_ole_promote_if_needed(hwnd);
                } else {
                    // WebView2 sometimes swallows WM_PARENTNOTIFY LBUTTONUP.
                    finish_press_maybe_activate(hwnd);
                }
                return LRESULT(0);
            }
        }
        WM_PARENTNOTIFY => {
            let child_msg = (wparam.0 as u32) & 0xffff;
            if child_msg == WM_LBUTTONDOWN {
                // Belt-and-suspenders when MOUSEACTIVATE did not arm the press.
                // Also arms FG presses for FIL-39 Hybrid promote timing.
                note_press(hwnd);
            } else if child_msg == WM_LBUTTONUP {
                finish_press_maybe_activate(hwnd);
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
    let win = window.clone();
    let _ = PROMOTE_EMIT.set(Box::new(move || {
        let _ = win.emit(OLE_PROMOTE_EVENT, ());
    }));
    let ok = unsafe { SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, 0) };
    if !ok.as_bool() {
        eprintln!("AIA-266: SetWindowSubclass for drag focus failed");
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

    #[test]
    fn ole_promote_event_name_stable() {
        assert_eq!(super::OLE_PROMOTE_EVENT, "afm://dnd/ole-promote");
    }
}
