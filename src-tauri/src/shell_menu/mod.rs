// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows shell / system context menu for a filesystem path (AIA-55).

use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellMenuResult {
    pub success: bool,
    pub cancelled: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub fn shell_show_context_menu(path: String, x: i32, y: i32) -> ShellMenuResult {
    #[cfg(windows)]
    {
        windows_impl::show_context_menu(&path, x, y)
    }
    #[cfg(not(windows))]
    {
        let _ = (path, x, y);
        ShellMenuResult {
            success: false,
            cancelled: false,
            error: Some("System context menu is only supported on Windows".to_string()),
        }
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::ShellMenuResult;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, POINT};
    use windows::Win32::System::Com::{
        CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows::Win32::UI::Shell::{
        IContextMenu, ILCreateFromPathW, IShellFolder, IShellItem, SHBindToParent,
        SHCreateItemFromParsingName, SHGetIDListFromObject, CMF_EXPLORE, CMF_NORMAL,
        CMINVOKECOMMANDINFOEX,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreatePopupMenu, DestroyMenu, GetCursorPos, GetForegroundWindow, SetForegroundWindow,
        TrackPopupMenuEx, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, SW_SHOWNORMAL,
    };

    struct OwnedPidl(*mut ITEMIDLIST);

    impl Drop for OwnedPidl {
        fn drop(&mut self) {
            unsafe {
                if !self.0.is_null() {
                    CoTaskMemFree(Some(self.0 as *const _));
                }
            }
        }
    }

    pub fn show_context_menu(path: &str, x: i32, y: i32) -> ShellMenuResult {
        let trimmed = path.trim();
        if trimmed.is_empty() || !Path::new(trimmed).exists() {
            return ShellMenuResult {
                success: false,
                cancelled: false,
                error: Some(format!("Path not found: {trimmed}")),
            };
        }

        unsafe { show_context_menu_unsafe(trimmed, x, y) }
    }

    unsafe fn show_context_menu_unsafe(path: &str, x: i32, y: i32) -> ShellMenuResult {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let needs_uninit = hr.is_ok();

        let result = match create_context_menu(path) {
            Ok((menu, _pidl)) => popup_and_invoke(&menu, x, y),
            Err(error) => ShellMenuResult {
                success: false,
                cancelled: false,
                error: Some(error),
            },
        };

        if needs_uninit {
            CoUninitialize();
        }
        result
    }

    unsafe fn create_context_menu(path: &str) -> Result<(IContextMenu, OwnedPidl), String> {
        let absolute_pidl = create_absolute_pidl(path)?;
        let owned = OwnedPidl(absolute_pidl);

        let mut child_pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        let parent_folder: IShellFolder =
            SHBindToParent(absolute_pidl, Some(&mut child_pidl)).map_err(|err| {
                format!("Failed to bind to parent folder: {err}")
            })?;

        if child_pidl.is_null() {
            return Err("Failed to resolve selected shell item".to_string());
        }

        let child_items = [child_pidl as *const ITEMIDLIST];
        let menu = parent_folder
            .GetUIObjectOf::<IContextMenu>(HWND::default(), &child_items, None)
            .map_err(|err| format!("Failed to get IContextMenu: {err}"))?;

        Ok((menu, owned))
    }

    unsafe fn create_absolute_pidl(path: &str) -> Result<*mut ITEMIDLIST, String> {
        let normalized = path.replace('/', "\\");
        let wide: Vec<u16> = OsStr::new(&normalized)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let wide_ptr = PCWSTR::from_raw(wide.as_ptr());

        if let Ok(shell_item) =
            SHCreateItemFromParsingName::<_, _, IShellItem>(wide_ptr, None)
        {
            if let Ok(pidl) = SHGetIDListFromObject(&shell_item) {
                if !pidl.is_null() {
                    return Ok(pidl);
                }
            }
        }

        let legacy = ILCreateFromPathW(wide_ptr);
        if !legacy.is_null() {
            return Ok(legacy);
        }

        Err(format!("Failed to resolve shell path '{path}'"))
    }

    unsafe fn popup_and_invoke(menu: &IContextMenu, x: i32, y: i32) -> ShellMenuResult {
        let hmenu = match CreatePopupMenu() {
            Ok(h) => h,
            Err(err) => {
                return ShellMenuResult {
                    success: false,
                    cancelled: false,
                    error: Some(format!("Failed to create popup menu: {err}")),
                };
            }
        };

        let query = menu.QueryContextMenu(hmenu, 0, 1, 0x7FFF, CMF_NORMAL | CMF_EXPLORE);
        if query.is_err() {
            let _ = DestroyMenu(hmenu);
            return ShellMenuResult {
                success: false,
                cancelled: false,
                error: Some(format!("QueryContextMenu failed: {query:?}")),
            };
        }

        let hwnd = GetForegroundWindow();
        let _ = SetForegroundWindow(hwnd);

        // Prefer live cursor; fall back to caller coords (CSS client px ≈ screen for Day-1).
        let mut point = POINT { x, y };
        let _ = GetCursorPos(&mut point);

        let cmd = TrackPopupMenuEx(
            hmenu,
            (TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_LEFTALIGN).0,
            point.x,
            point.y,
            hwnd,
            None,
        );
        let selected = cmd.0 as u32;

        if selected == 0 {
            let _ = DestroyMenu(hmenu);
            return ShellMenuResult {
                success: true,
                cancelled: true,
                error: None,
            };
        }

        // QueryContextMenu idCmdFirst = 1 → MAKEINTRESOURCE(id - 1).
        let mut invoke_info: CMINVOKECOMMANDINFOEX = std::mem::zeroed();
        invoke_info.cbSize = std::mem::size_of::<CMINVOKECOMMANDINFOEX>() as u32;
        invoke_info.hwnd = hwnd;
        invoke_info.lpVerb = windows::core::PCSTR((selected as usize - 1) as *const u8);
        invoke_info.nShow = SW_SHOWNORMAL.0;

        let invoke = menu.InvokeCommand(&invoke_info as *const _ as *const _);
        let _ = DestroyMenu(hmenu);

        match invoke {
            Ok(()) => ShellMenuResult {
                success: true,
                cancelled: false,
                error: None,
            },
            Err(err) => ShellMenuResult {
                success: false,
                cancelled: false,
                error: Some(format!("InvokeCommand failed: {err}")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_path_fails_cleanly() {
        let result = shell_show_context_menu(
            "Z:/definitely-missing-a-file-manager-path-xyz".to_string(),
            0,
            0,
        );
        assert!(!result.success);
        assert!(!result.cancelled);
        assert!(result.error.is_some());
    }

    #[test]
    fn empty_path_fails_cleanly() {
        let result = shell_show_context_menu(String::new(), 10, 10);
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
