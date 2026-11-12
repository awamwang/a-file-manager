// SPDX-License-Identifier: GPL-3.0-or-later
//! AIA-261/264/266 + FIL-37/39/43/44/45/46: outbound drag must not raise AFM over the drop target.
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
//! 5. FIL-45: 越过拖势阈值且已 `arm_outbound_paths` 时**立即**宿主直启 OLE；
//!    前端 `dragstart` `preventDefault` 取消 HTML5 文本出站。
//! 6. FIL-46/47: 失焦挂临时 `WS_EX_NOACTIVATE`；`WH_MOUSE_LL` 预武装 prior；
//!    `GetAncestor(GA_ROOT)` 识别 OOP WebView。FIL-47：**禁止**对目标窗
//!    `HWND_TOPMOST` / 无条件 `SetForegroundWindow`——那会把 PotPlayer 强拉前台并
//!    打断 `DoDragDrop`。仅当我们自己已抢到 FG 时才把 prior 还回去。
//!
//! ## FIL-45/47 焦点 / OLE
//! - 后台临时 `WS_EX_NOACTIVATE` + `MA_NOACTIVATE`；仅「我们是 FG」时 sticky 还 prior
//! - 阈值 OLE：`take_outbound_paths` → `start_files_drag_hwnd`；OLE 期间不轮询抢焦点

#![cfg(windows)]

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering};
use std::sync::{Mutex, OnceLock};

use tauri::{AppHandle, Emitter, Manager, Runtime};
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, EnumChildWindows, GetAncestor, GetClassNameW, GetCursorPos,
    GetForegroundWindow, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect,
    GetWindowThreadProcessId, IsChild, KillTimer, SetForegroundWindow, SetTimer,
    SetWindowLongPtrW, SetWindowsHookExW, UnhookWindowsHookEx, WindowFromPoint, GA_ROOT,
    GWL_EXSTYLE, HHOOK, MA_NOACTIVATE, MSLLHOOKSTRUCT, SM_CXDRAG, SM_CYDRAG, WA_INACTIVE,
    WH_MOUSE_LL, WM_ACTIVATE, WM_CREATE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE,
    WM_PARENTNOTIFY, WM_TIMER, WS_EX_NOACTIVATE,
};

const SUBCLASS_ID: usize = 0xA_F1_26_01; // AFM + AIA-261（主窗）
const WEBVIEW_SUBCLASS_ID: usize = 0xA_F1_43_02; // FIL-43 WebView 子窗
const POLL_TIMER_ID: usize = 0xA_F1_26_04;
const FIND_TIMER_ID: usize = 0xA_F1_43_05;
const POLL_INTERVAL_MS: u32 = 16;
const FIND_INTERVAL_MS: u32 = 250;
const FIND_MAX_TICKS: i32 = 40; // ~10s 内重扫 WebView 子窗

/// Hybrid 前端监听（FIL-45 宿主直启失败时的回退事件名，保持稳定）。
pub const OLE_PROMOTE_EVENT: &str = "afm://dnd/ole-promote";

static PROMOTE_EMIT: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();
static POLL_ARMED: AtomicBool = AtomicBool::new(false);
static PROMOTE_EMITTED: AtomicBool = AtomicBool::new(false);
static ROOT_HWND: AtomicIsize = AtomicIsize::new(0);
static FIND_TICKS: AtomicI32 = AtomicI32::new(0);
static TEMP_NOACTIVATE: AtomicBool = AtomicBool::new(false);
/// 最近一次「非我们」的前台窗（失焦时记录），最大化抢前台时作 prior 兜底。
static LAST_OTHER_FG: AtomicIsize = AtomicIsize::new(0);
static MOUSE_HOOK: AtomicIsize = AtomicIsize::new(0);
static WEBVIEW_SUBCLASSED: OnceLock<Mutex<HashSet<isize>>> = OnceLock::new();

fn webview_subclassed() -> &'static Mutex<HashSet<isize>> {
    WEBVIEW_SUBCLASSED.get_or_init(|| Mutex::new(HashSet::new()))
}

fn hwnd_is_ours(our_root: HWND, candidate: HWND) -> bool {
    if candidate.0.is_null() || our_root.0.is_null() {
        return false;
    }
    if candidate == our_root {
        return true;
    }
    unsafe {
        if IsChild(our_root, candidate).as_bool() {
            return true;
        }
        // FIL-46: WebView2 OOP 命中窗上 IsChild 常为假；同一顶层根仍算我们。
        let root = GetAncestor(candidate, GA_ROOT);
        root == our_root
    }
}

fn hwnd_to_isize(hwnd: HWND) -> isize {
    hwnd.0 as isize
}

fn isize_to_hwnd(raw: isize) -> HWND {
    HWND(raw as *mut _)
}

fn root_hwnd() -> HWND {
    isize_to_hwnd(ROOT_HWND.load(Ordering::SeqCst))
}

fn left_button_down() -> bool {
    unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 }
}

fn point_targets_us(our_root: HWND, pt: POINT) -> bool {
    if our_root.0.is_null() {
        return false;
    }
    let hit = unsafe { WindowFromPoint(pt) };
    if hwnd_is_ours(our_root, hit) {
        return true;
    }
    let mut rc = RECT::default();
    if unsafe { GetWindowRect(our_root, &mut rc) }.is_err() {
        return false;
    }
    if pt.x < rc.left || pt.x >= rc.right || pt.y < rc.top || pt.y >= rc.bottom {
        return false;
    }
    // 点在我们矩形内，且该点顶层根是我们（露出的最大化区域）。
    let top_root = unsafe { GetAncestor(hit, GA_ROOT) };
    top_root == our_root
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

fn restore_press_prior_if_we_raised(our_root: HWND) {
    // FIL-48: 其它应用已在前台 → 不动；仅本轮后台推迟激活且我们误抢了 FG 时才还 prior。
    // 禁止用 LAST_OTHER 凭空 note_deferred（会在前台拖拽时把旧目标强拉前台）。
    if !drag::had_deferred_activate() {
        return;
    }
    let fg = unsafe { GetForegroundWindow() };
    if !hwnd_is_ours(our_root, fg) {
        return;
    }
    if drag::press_prior_foreground() == 0 {
        return;
    }
    drag::restore_press_prior_foreground(hwnd_to_isize(our_root));
}

/// 后台期间挂临时 WS_EX_NOACTIVATE（失焦即挂；单击激活 / OLE 结束时清除）。
fn enable_temp_noactivate(root: HWND) {
    if root.0.is_null() {
        return;
    }
    let _ = TEMP_NOACTIVATE.swap(true, Ordering::SeqCst);
    unsafe {
        let style = GetWindowLongPtrW(root, GWL_EXSTYLE);
        let flag = WS_EX_NOACTIVATE.0 as isize;
        if style & flag == 0 {
            SetWindowLongPtrW(root, GWL_EXSTYLE, style | flag);
        }
    }
}

fn disable_temp_noactivate(root: HWND) {
    if !TEMP_NOACTIVATE.swap(false, Ordering::SeqCst) {
        return;
    }
    if root.0.is_null() {
        return;
    }
    unsafe {
        let style = GetWindowLongPtrW(root, GWL_EXSTYLE);
        let flag = WS_EX_NOACTIVATE.0 as isize;
        if style & flag != 0 {
            SetWindowLongPtrW(root, GWL_EXSTYLE, style & !flag);
        }
    }
}

fn remember_other_foreground(our_root: HWND) {
    let fg = unsafe { GetForegroundWindow() };
    if fg.0.is_null() || hwnd_is_ours(our_root, fg) {
        return;
    }
    LAST_OTHER_FG.store(hwnd_to_isize(fg), Ordering::SeqCst);
}

fn request_ole_promote_if_needed(our_root: HWND) {
    if PROMOTE_EMITTED.load(Ordering::SeqCst) || drag::ole_drag_in_progress() {
        return;
    }
    if !drag::drag_gesture_active() {
        return;
    }
    let paths = drag::take_outbound_paths();
    if paths.is_empty() {
        return;
    }
    PROMOTE_EMITTED.store(true, Ordering::SeqCst);
    // 仅当我们误抢了 FG 时还 prior；不要在进 OLE 前无条件抬目标窗。
    restore_press_prior_if_we_raised(our_root);

    match drag::start_files_drag_hwnd(hwnd_to_isize(our_root), paths.clone()) {
        Ok(()) => {
            disable_temp_noactivate(our_root);
        }
        Err(err) => {
            eprintln!("FIL-45: host-direct OLE start failed: {err}");
            drag::arm_outbound_paths(paths);
            PROMOTE_EMITTED.store(false, Ordering::SeqCst);
            if let Some(emit) = PROMOTE_EMIT.get() {
                emit();
            }
        }
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

/// 按下：后台则推迟激活；前台也记录原点以便阈值 OLE。
fn note_press(hwnd: HWND) {
    let root = {
        let r = root_hwnd();
        if r.0.is_null() {
            hwnd
        } else {
            r
        }
    };
    let fg = unsafe { GetForegroundWindow() };
    let mut pt = POINT { x: 0, y: 0 };
    let _ = unsafe { GetCursorPos(&mut pt) };

    if drag::should_suppress_foreground_raise() {
        arm_poll_timer(root);
        // 仅当我们已是 FG 时才尝试还 prior（否则无操作）。
        restore_press_prior_if_we_raised(root);
        return;
    }

    if !hwnd_is_ours(root, fg) {
        drag::note_deferred_activate(hwnd_to_isize(fg), pt.x, pt.y);
        LAST_OTHER_FG.store(hwnd_to_isize(fg), Ordering::SeqCst);
        enable_temp_noactivate(root);
        // 其它应用在前台：不 SetForegroundWindow。
    } else if TEMP_NOACTIVATE.load(Ordering::SeqCst)
        && left_button_down()
        && LAST_OTHER_FG.load(Ordering::SeqCst) != 0
        && LAST_OTHER_FG.load(Ordering::SeqCst) != hwnd_to_isize(root)
    {
        // 仍处「后台 noactivate」会话却已是 FG → 视为误抢，记下 prior 并还回。
        let last = LAST_OTHER_FG.load(Ordering::SeqCst);
        drag::note_deferred_activate(last, pt.x, pt.y);
        restore_press_prior_if_we_raised(root);
    } else {
        // 本就前台的正常按下：清空陈旧 prior，绝不拉其它应用。
        drag::note_press_origin(pt.x, pt.y);
    }
    PROMOTE_EMITTED.store(false, Ordering::SeqCst);
    arm_poll_timer(root);
}

/// Mouse-up without drag/OLE: activate like a normal focus click.
fn finish_press_maybe_activate(hwnd: HWND) {
    let root = {
        let r = root_hwnd();
        if r.0.is_null() {
            hwnd
        } else {
            r
        }
    };
    disarm_poll_timer(root);
    PROMOTE_EMITTED.store(false, Ordering::SeqCst);
    let should_activate = drag::take_pending_activate()
        && !drag::ole_drag_in_progress()
        && !drag::drag_gesture_active();
    if !drag::ole_drag_in_progress() && !drag::drag_gesture_active() {
        drag::clear_outbound_paths();
    }
    drag::clear_press_state();
    disable_temp_noactivate(root);
    if should_activate {
        let _ = unsafe { SetForegroundWindow(root) };
    } else if !hwnd_is_ours(root, unsafe { GetForegroundWindow() }) {
        // 仍在后台：保持 noactivate，便于下次按下。
        enable_temp_noactivate(root);
    }
}

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) };
    if n <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..n as usize])
}

fn is_webview_input_class(name: &str) -> bool {
    name.starts_with("Chrome_WidgetWin_")
        || name == "Intermediate D3D Window"
        || name == "Chrome_RenderWidgetHostHWND"
}

fn same_process(hwnd: HWND) -> bool {
    let mut pid = 0u32;
    let _ = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid != 0 && pid == std::process::id()
}

fn subclass_webview_child(child: HWND) {
    if child.0.is_null() || !same_process(child) {
        return;
    }
    let name = class_name(child);
    if !is_webview_input_class(&name) {
        return;
    }
    let key = hwnd_to_isize(child);
    let mut set = match webview_subclassed().lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    if set.contains(&key) {
        return;
    }
    let ok = unsafe { SetWindowSubclass(child, Some(webview_subclass_proc), WEBVIEW_SUBCLASS_ID, 0) };
    if ok.as_bool() {
        set.insert(key);
    } else {
        eprintln!("FIL-43: SetWindowSubclass for WebView child failed ({name})");
    }
}

unsafe extern "system" fn enum_child_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    subclass_webview_child(hwnd);
    BOOL(1)
}

fn scan_webview_children(root: HWND) {
    if root.0.is_null() {
        return;
    }
    let _ = unsafe { EnumChildWindows(Some(root), Some(enum_child_proc), LPARAM(0)) };
}

fn arm_find_timer(hwnd: HWND) {
    FIND_TICKS.store(0, Ordering::SeqCst);
    let _ = unsafe { SetTimer(Some(hwnd), FIND_TIMER_ID, FIND_INTERVAL_MS, None) };
}

fn disarm_find_timer(hwnd: HWND) {
    let _ = unsafe { KillTimer(Some(hwnd), FIND_TIMER_ID) };
}

/// FIL-46: 系统分发点击前武装——不依赖 WebView 是否把 MOUSEACTIVATE 交给我们。
unsafe extern "system" fn mouse_ll_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 && wparam.0 as u32 == WM_LBUTTONDOWN {
        let root = root_hwnd();
        if !root.0.is_null() {
            let info = *(lparam.0 as *const MSLLHOOKSTRUCT);
            let pt = info.pt;
            let fg = GetForegroundWindow();
            if point_targets_us(root, pt) && !hwnd_is_ours(root, fg) {
                note_press(root);
            }
        }
    }
    CallNextHookEx(
        Some(HHOOK(MOUSE_HOOK.load(Ordering::SeqCst) as *mut _)),
        code,
        wparam,
        lparam,
    )
}

fn install_mouse_ll_hook() {
    if MOUSE_HOOK.load(Ordering::SeqCst) != 0 {
        return;
    }
    let hook = unsafe {
        SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_ll_proc), None, 0)
    };
    match hook {
        Ok(h) => {
            MOUSE_HOOK.store(h.0 as isize, Ordering::SeqCst);
        }
        Err(err) => {
            eprintln!("FIL-46: SetWindowsHookExW WH_MOUSE_LL failed: {err}");
        }
    }
}

#[allow(dead_code)]
fn uninstall_mouse_ll_hook() {
    let raw = MOUSE_HOOK.swap(0, Ordering::SeqCst);
    if raw != 0 {
        let _ = unsafe { UnhookWindowsHookEx(HHOOK(raw as *mut _)) };
    }
}

/// FIL-43: WebView 子窗按下路径。
unsafe extern "system" fn webview_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    let root = root_hwnd();
    let our = if root.0.is_null() { hwnd } else { root };
    match msg {
        WM_MOUSEACTIVATE => {
            let fg = unsafe { GetForegroundWindow() };
            if !hwnd_is_ours(our, fg) {
                note_press(our);
                return LRESULT(MA_NOACTIVATE as isize);
            }
        }
        WM_LBUTTONDOWN => {
            note_press(our);
        }
        WM_LBUTTONUP => {
            finish_press_maybe_activate(our);
        }
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
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
                note_press(hwnd);
                return LRESULT(MA_NOACTIVATE as isize);
            }
        }
        WM_ACTIVATE => {
            let active = (wparam.0 as u32) & 0xffff;
            if active == WA_INACTIVE {
                remember_other_foreground(hwnd);
                enable_temp_noactivate(hwnd);
            } else if left_button_down()
                || drag::should_suppress_foreground_raise()
                || drag::drag_gesture_active()
            {
                // 左键未松就变成前台：这是误抢，还 prior（内部仅当我们是 FG 才动手）。
                if !drag::should_suppress_foreground_raise() {
                    note_press(hwnd);
                }
                restore_press_prior_if_we_raised(hwnd);
                enable_temp_noactivate(hwnd);
            } else if !drag::ole_drag_in_progress() {
                disable_temp_noactivate(hwnd);
            }
        }
        WM_TIMER => {
            if wparam.0 == FIND_TIMER_ID {
                scan_webview_children(hwnd);
                let ticks = FIND_TICKS.fetch_add(1, Ordering::SeqCst) + 1;
                let count = webview_subclassed()
                    .lock()
                    .map(|s| s.len())
                    .unwrap_or(0);
                if count > 0 || ticks >= FIND_MAX_TICKS {
                    disarm_find_timer(hwnd);
                }
                return LRESULT(0);
            }
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
                    // OLE 进行中由 DropSource 处理；此处只在我们误持 FG 时还 prior。
                    if !drag::ole_drag_in_progress()
                        && drag::should_suppress_foreground_raise()
                        && hwnd_is_ours(hwnd, unsafe { GetForegroundWindow() })
                    {
                        restore_press_prior_if_we_raised(hwnd);
                    }
                    request_ole_promote_if_needed(hwnd);
                } else {
                    finish_press_maybe_activate(hwnd);
                }
                return LRESULT(0);
            }
        }
        WM_PARENTNOTIFY => {
            let child_msg = (wparam.0 as u32) & 0xffff;
            if child_msg == WM_CREATE {
                scan_webview_children(hwnd);
            } else if child_msg == WM_LBUTTONDOWN {
                note_press(hwnd);
            } else if child_msg == WM_LBUTTONUP {
                finish_press_maybe_activate(hwnd);
            }
        }
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

/// FIL-44: 前端 arm 出站时登记路径，供宿主阈值直启 OLE。
#[tauri::command]
pub fn dnd_arm_outbound_paths(paths: Vec<String>) -> Result<(), String> {
    let cleaned: Vec<std::path::PathBuf> = paths
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .map(std::path::PathBuf::from)
        .collect();
    drag::arm_outbound_paths(cleaned);
    Ok(())
}

/// FIL-44: 取消拖势且未进入 OLE 时清空登记路径。
#[tauri::command]
pub fn dnd_clear_outbound_paths() -> Result<(), String> {
    drag::clear_outbound_paths();
    Ok(())
}

/// Install focus / OLE host hooks on the main window.
pub fn install_for_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    ROOT_HWND.store(hwnd_to_isize(hwnd), Ordering::SeqCst);
    let win = window.clone();
    let _ = PROMOTE_EMIT.set(Box::new(move || {
        let _ = win.emit(OLE_PROMOTE_EVENT, ());
    }));
    let ok = unsafe { SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, 0) };
    if !ok.as_bool() {
        eprintln!("AIA-266: SetWindowSubclass for drag focus failed");
        return;
    }
    scan_webview_children(hwnd);
    arm_find_timer(hwnd);
    install_mouse_ll_hook();
    // 启动时若已在后台，预先挂 noactivate，避免第一次点击才武装。
    let fg = unsafe { GetForegroundWindow() };
    if !hwnd_is_ours(hwnd, fg) {
        remember_other_foreground(hwnd);
        enable_temp_noactivate(hwnd);
    }
}

#[cfg(test)]
mod tests {
    use super::{is_webview_input_class, OLE_PROMOTE_EVENT};
    use std::sync::Mutex;

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
        assert_eq!(OLE_PROMOTE_EVENT, "afm://dnd/ole-promote");
    }

    #[test]
    fn webview_input_class_names_recognized() {
        assert!(is_webview_input_class("Chrome_WidgetWin_1"));
        assert!(is_webview_input_class("Chrome_WidgetWin_0"));
        assert!(is_webview_input_class("Intermediate D3D Window"));
        assert!(is_webview_input_class("Chrome_RenderWidgetHostHWND"));
        assert!(!is_webview_input_class("Button"));
        assert!(!is_webview_input_class(""));
    }
}
