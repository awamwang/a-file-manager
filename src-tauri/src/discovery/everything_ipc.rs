// SPDX-License-Identifier: GPL-3.0-or-later
//! Everything IPC client (WM_COPYDATA) — product path per ADR 0002.
//! Does **not** shell `es.exe`. Adapted for greenfield a-file-manager (S4).

use super::backend::EverythingBackend;
use super::types::{DiscoveryHit, EverythingProbe, EverythingQueryResult};

#[cfg(windows)]
mod win {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::DataExchange::COPYDATASTRUCT;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowW,
        GetWindowLongPtrW, PeekMessageW, PostQuitMessage, RegisterClassExW, SendMessageW,
        SetWindowLongPtrW, TranslateMessage, UnregisterClassW, CREATESTRUCTW, CS_HREDRAW,
        CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, HWND_MESSAGE, MSG, PM_REMOVE, WINDOW_EX_STYLE,
        WM_COPYDATA, WM_CREATE, WM_DESTROY, WM_QUIT, WNDCLASSEXW, WS_OVERLAPPED,
    };

    const EVERYTHING_IPC_WNDCLASS_MODERN: &str = "EVERYTHING_TASKBAR_NOTIFICATION";
    const EVERYTHING_IPC_WNDCLASS_LEGACY: &str = "EVERYTHING";
    const EVERYTHING_IPC_COPYDATAQUERYW: usize = 2;
    const EVERYTHING_IPC_FOLDER: u32 = 0x0000_0001;
    const COPYDATA_QUERYCOMPLETE: u32 = 1;
    const QUERY_TIMEOUT: Duration = Duration::from_secs(5);

    #[repr(C)]
    struct EverythingIpcQuery {
        reply_hwnd: u32,
        reply_copydata_message: u32,
        search_flags: u32,
        offset: u32,
        max_results: u32,
    }

    struct PendingReply {
        results: Option<Result<EverythingQueryResult, String>>,
    }

    unsafe extern "system" fn reply_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_CREATE => {
                let create = &*(lparam.0 as *const CREATESTRUCTW);
                let boxed = Box::from_raw(create.lpCreateParams as *mut Arc<Mutex<PendingReply>>);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(boxed) as isize);
                LRESULT(0)
            }
            WM_COPYDATA => {
                let cds = &*(lparam.0 as *const COPYDATASTRUCT);
                if cds.dwData == COPYDATA_QUERYCOMPLETE as usize {
                    if let Some(state) = get_window_state(hwnd) {
                        let parsed =
                            parse_list_v1(cds.lpData as *const u8, cds.cbData as usize);
                        if let Ok(mut guard) = state.lock() {
                            guard.results = Some(parsed);
                        }
                    }
                    return LRESULT(1);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                if ptr != 0 {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    drop(Box::from_raw(ptr as *mut Arc<Mutex<PendingReply>>));
                }
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    fn get_window_state(hwnd: HWND) -> Option<Arc<Mutex<PendingReply>>> {
        unsafe {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr == 0 {
                None
            } else {
                let arc = &*(ptr as *const Arc<Mutex<PendingReply>>);
                Some(arc.clone())
            }
        }
    }

    fn to_wide(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn find_everything_hwnd() -> Option<HWND> {
        for class in [EVERYTHING_IPC_WNDCLASS_MODERN, EVERYTHING_IPC_WNDCLASS_LEGACY] {
            let wide = to_wide(class);
            unsafe {
                if let Ok(hwnd) = FindWindowW(PCWSTR(wide.as_ptr()), PCWSTR::null()) {
                    if !hwnd.0.is_null() {
                        return Some(hwnd);
                    }
                }
            }
        }
        None
    }

    fn parse_list_v1(mem: *const u8, size: usize) -> Result<EverythingQueryResult, String> {
        if mem.is_null() || size < 7 * 4 {
            return Err("Everything IPC reply too small".to_string());
        }
        unsafe {
            let base = mem as *const u32;
            let total_items = *base.add(2) as u64;
            let num_items = *base.add(5) as usize;
            let offset = *base.add(6) as u64;
            let header_bytes = 7 * 4;
            let items_bytes = num_items
                .checked_mul(3 * 4)
                .ok_or_else(|| "Everything IPC item count overflow".to_string())?;
            if size < header_bytes + items_bytes {
                return Err("Everything IPC reply truncated".to_string());
            }

            let mut hits = Vec::with_capacity(num_items);
            for i in 0..num_items {
                let flags = *base.add(7 + i * 3);
                let filename_offset = *base.add(7 + i * 3 + 1) as usize;
                let path_offset = *base.add(7 + i * 3 + 2) as usize;
                if filename_offset >= size || path_offset >= size {
                    continue;
                }
                let name = read_wide_z(mem.add(filename_offset), size - filename_offset);
                let path_dir = read_wide_z(mem.add(path_offset), size - path_offset);
                if name.is_empty() {
                    continue;
                }
                let full_path = if path_dir.is_empty() {
                    name.clone()
                } else if path_dir.ends_with('\\') || path_dir.ends_with('/') {
                    format!("{path_dir}{name}")
                } else {
                    format!("{path_dir}\\{name}")
                };
                hits.push(DiscoveryHit {
                    name,
                    path: full_path.replace('\\', "/"),
                    is_dir: (flags & EVERYTHING_IPC_FOLDER) != 0,
                });
            }

            Ok(EverythingQueryResult {
                total_count: total_items,
                offset,
                hits,
            })
        }
    }

    unsafe fn read_wide_z(ptr: *const u8, max_bytes: usize) -> String {
        if ptr.is_null() || max_bytes < 2 {
            return String::new();
        }
        let u16_ptr = ptr as *const u16;
        let max_chars = max_bytes / 2;
        let mut len = 0usize;
        while len < max_chars && *u16_ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(u16_ptr, len))
    }

    pub struct WindowsEverythingIpc;

    impl EverythingBackend for WindowsEverythingIpc {
        fn probe(&self) -> EverythingProbe {
            match find_everything_hwnd() {
                Some(_) => EverythingProbe {
                    available: true,
                    detail: "Everything IPC window found".to_string(),
                },
                None => EverythingProbe {
                    available: false,
                    detail: "Everything IPC window not found — install/start Everything (voidtools)"
                        .to_string(),
                },
            }
        }

        fn query(
            &self,
            search: &str,
            max_results: u32,
            offset: u32,
        ) -> Result<EverythingQueryResult, String> {
            let everything_hwnd = find_everything_hwnd().ok_or_else(|| {
                "Everything IPC unavailable — install/start Everything".to_string()
            })?;

            // Empty search is valid: Everything still replies with totitems (used for
            // DiscoveryStatus.estimated_item_count). Do not short-circuit to zero.
            unsafe { query_via_copydata(everything_hwnd, search, max_results, offset) }
        }
    }

    unsafe fn query_via_copydata(
        everything_hwnd: HWND,
        search: &str,
        max_results: u32,
        offset: u32,
    ) -> Result<EverythingQueryResult, String> {
        let class_name = to_wide("AFileManagerDiscoveryEverythingReply");
        let window_name = to_wide("AFileManagerDiscoveryEverythingReply");

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(reply_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: windows::Win32::Foundation::HINSTANCE(ptr::null_mut()),
            hIcon: windows::Win32::UI::WindowsAndMessaging::HICON(ptr::null_mut()),
            hCursor: windows::Win32::UI::WindowsAndMessaging::HCURSOR(ptr::null_mut()),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(ptr::null_mut()),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hIconSm: windows::Win32::UI::WindowsAndMessaging::HICON(ptr::null_mut()),
        };

        let _ = RegisterClassExW(&wnd_class);

        let state = Arc::new(Mutex::new(PendingReply { results: None }));
        let state_ptr = Box::into_raw(Box::new(state.clone()));

        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(window_name.as_ptr()),
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            Some(HWND_MESSAGE),
            None,
            None,
            Some(state_ptr as *const core::ffi::c_void),
        ) {
            Ok(hwnd) if !hwnd.0.is_null() => hwnd,
            Ok(_) | Err(_) => {
                drop(Box::from_raw(state_ptr));
                return Err("Failed to create Everything reply window".to_string());
            }
        };

        let search_wide: Vec<u16> = OsStr::new(search)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let header_size = std::mem::size_of::<EverythingIpcQuery>();
        let payload_size = header_size + search_wide.len() * 2;
        let mut payload = vec![0u8; payload_size];
        let header = EverythingIpcQuery {
            reply_hwnd: hwnd.0 as usize as u32,
            reply_copydata_message: COPYDATA_QUERYCOMPLETE,
            search_flags: 0,
            offset,
            max_results,
        };
        ptr::copy_nonoverlapping(
            &header as *const EverythingIpcQuery as *const u8,
            payload.as_mut_ptr(),
            header_size,
        );
        ptr::copy_nonoverlapping(
            search_wide.as_ptr() as *const u8,
            payload.as_mut_ptr().add(header_size),
            search_wide.len() * 2,
        );

        let cds = COPYDATASTRUCT {
            dwData: EVERYTHING_IPC_COPYDATAQUERYW,
            cbData: payload.len() as u32,
            lpData: payload.as_mut_ptr() as *mut _,
        };

        let accepted = SendMessageW(
            everything_hwnd,
            WM_COPYDATA,
            Some(WPARAM(hwnd.0 as usize)),
            Some(LPARAM(&cds as *const COPYDATASTRUCT as isize)),
        );

        if accepted.0 == 0 {
            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(PCWSTR(class_name.as_ptr()), None);
            return Err("Everything IPC rejected query (is IPC enabled?)".to_string());
        }

        let deadline = Instant::now() + QUERY_TIMEOUT;
        let mut msg = MSG::default();
        let mut got_result = false;

        while Instant::now() < deadline {
            if let Ok(guard) = state.lock() {
                if guard.results.is_some() {
                    got_result = true;
                    break;
                }
            }

            let has_msg =
                PeekMessageW(&mut msg, Some(HWND(ptr::null_mut())), 0, 0, PM_REMOVE).as_bool();
            if has_msg {
                if msg.message == WM_QUIT {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            } else {
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        let result = {
            let mut guard = state.lock().map_err(|err| err.to_string())?;
            guard.results.take()
        };

        let _ = DestroyWindow(hwnd);
        let drain_deadline = Instant::now() + Duration::from_millis(200);
        while Instant::now() < drain_deadline {
            if !PeekMessageW(&mut msg, Some(HWND(ptr::null_mut())), 0, 0, PM_REMOVE).as_bool() {
                break;
            }
            if msg.message == WM_QUIT {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = UnregisterClassW(PCWSTR(class_name.as_ptr()), None);

        if !got_result {
            return Err("Timed out waiting for Everything IPC reply".to_string());
        }

        result.ok_or_else(|| "Everything IPC reply missing".to_string())?
    }

    #[cfg(test)]
    mod live_tests {
        use super::*;

        /// Optional live probe/query. Run with:
        /// `cargo test discovery::everything_ipc -- --ignored`
        /// when Everything is installed and running.
        #[test]
        #[ignore = "requires live Everything IPC; set AFM_EVERYTHING_LIVE=1 and run --ignored"]
        fn live_everything_ipc_probe_and_query() {
            if std::env::var("AFM_EVERYTHING_LIVE").ok().as_deref() != Some("1") {
                eprintln!("skip: set AFM_EVERYTHING_LIVE=1 to exercise live IPC");
                return;
            }
            let backend = WindowsEverythingIpc;
            let probe = backend.probe();
            assert!(
                probe.available,
                "Everything IPC window not found: {}",
                probe.detail
            );
            let result = backend
                .query("*.txt", 5, 0)
                .expect("live Everything query");
            assert!(result.total_count >= result.hits.len() as u64);
        }
    }
}

#[cfg(windows)]
pub use win::WindowsEverythingIpc;

#[cfg(not(windows))]
pub struct WindowsEverythingIpc;

#[cfg(not(windows))]
impl EverythingBackend for WindowsEverythingIpc {
    fn probe(&self) -> EverythingProbe {
        EverythingProbe {
            available: false,
            detail: "Everything IPC is Windows-only (ADR 0002)".to_string(),
        }
    }

    fn query(
        &self,
        _search: &str,
        _max_results: u32,
        _offset: u32,
    ) -> Result<EverythingQueryResult, String> {
        Err("Everything IPC is Windows-only".to_string())
    }
}
