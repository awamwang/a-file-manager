// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows shell context menu: native `TrackPopupMenu` cascade (FIL-53 / AIA-229).
//!
//! Product UI opens [`shell_show_context_menu`] beside the AFM row. Enumerate /
//! invoke helpers remain for diagnostics and tests (AIA-239 path retired in UI).

use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellMenuResult {
    pub success: bool,
    pub cancelled: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellContextMenuItem {
    pub id: u32,
    pub name: String,
    pub verb: Option<String>,
    pub children: Option<Vec<ShellContextMenuItem>>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetShellContextMenuResult {
    pub success: bool,
    pub items: Vec<ShellContextMenuItem>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InvokeShellMenuResult {
    pub success: bool,
    pub error: Option<String>,
}

/// 原生 Win32 弹出菜单（FIL-53 产品路径）。
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

#[tauri::command]
pub fn shell_get_context_menu(path: String) -> GetShellContextMenuResult {
    #[cfg(windows)]
    {
        windows_impl::get_context_menu(&path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        GetShellContextMenuResult {
            success: false,
            items: vec![],
            error: Some("System context menu is only supported on Windows".to_string()),
        }
    }
}

#[tauri::command]
pub fn shell_invoke_context_menu_item(
    path: String,
    command_id: u32,
    command_verb: Option<String>,
) -> InvokeShellMenuResult {
    #[cfg(windows)]
    {
        windows_impl::invoke_context_menu_item(&path, command_id, command_verb.as_deref())
    }
    #[cfg(not(windows))]
    {
        let _ = (path, command_id, command_verb);
        InvokeShellMenuResult {
            success: false,
            error: Some("System context menu is only supported on Windows".to_string()),
        }
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{
        GetShellContextMenuResult, InvokeShellMenuResult, ShellContextMenuItem, ShellMenuResult,
    };
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows::core::{PCWSTR, PSTR};
    use windows::Win32::Foundation::{HWND, POINT};
    use windows::Win32::System::Com::{
        CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows::Win32::UI::Shell::{
        IContextMenu, ILCreateFromPathW, IShellFolder, IShellItem, SHBindToParent,
        SHCreateItemFromParsingName, SHGetIDListFromObject, CMF_EXPLORE, CMF_NORMAL,
        CMINVOKECOMMANDINFOEX, GCS_VERBA,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreatePopupMenu, DestroyMenu, GetCursorPos, GetForegroundWindow, GetMenuItemCount,
        GetMenuItemInfoW, SetForegroundWindow, TrackPopupMenuEx, MENUITEMINFOW, MIIM_ID,
        MIIM_STRING, MIIM_SUBMENU, SW_SHOWNORMAL, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    };

    /// Verbs AFM already covers in its own menu — omit from the embedded shell list.
    const EXCLUDED_VERBS: &[&str] = &[
        "cut",
        "copy",
        "paste",
        "delete",
        "rename",
        "link",
        "properties",
    ];

    const UNSUPPORTED_VERBS: &[&str] = &["pintostartscreen"];

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

    pub fn get_context_menu(path: &str) -> GetShellContextMenuResult {
        let trimmed = path.trim();
        if trimmed.is_empty() || !Path::new(trimmed).exists() {
            return GetShellContextMenuResult {
                success: false,
                items: vec![],
                error: Some(format!("Path not found: {trimmed}")),
            };
        }

        unsafe {
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let needs_uninit = hr.is_ok();
            let result = get_context_menu_items(trimmed);
            if needs_uninit {
                CoUninitialize();
            }
            result
        }
    }

    pub fn invoke_context_menu_item(
        path: &str,
        command_id: u32,
        command_verb: Option<&str>,
    ) -> InvokeShellMenuResult {
        let trimmed = path.trim();
        if trimmed.is_empty() || !Path::new(trimmed).exists() {
            return InvokeShellMenuResult {
                success: false,
                error: Some(format!("Path not found: {trimmed}")),
            };
        }

        unsafe {
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let needs_uninit = hr.is_ok();
            let result = invoke_command(trimmed, command_id, command_verb);
            if needs_uninit {
                CoUninitialize();
            }
            result
        }
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

    unsafe fn get_context_menu_items(path: &str) -> GetShellContextMenuResult {
        let (menu, _pidl) = match create_context_menu(path) {
            Ok(pair) => pair,
            Err(error) => {
                return GetShellContextMenuResult {
                    success: false,
                    items: vec![],
                    error: Some(error),
                };
            }
        };

        let hmenu = match CreatePopupMenu() {
            Ok(h) => h,
            Err(err) => {
                return GetShellContextMenuResult {
                    success: false,
                    items: vec![],
                    error: Some(format!("Failed to create popup menu: {err}")),
                };
            }
        };

        let query = menu.QueryContextMenu(hmenu, 0, 1, 0x7FFF, CMF_NORMAL | CMF_EXPLORE);
        if query.is_err() {
            let _ = DestroyMenu(hmenu);
            return GetShellContextMenuResult {
                success: false,
                items: vec![],
                error: Some(format!("QueryContextMenu failed: {query:?}")),
            };
        }

        let items = extract_menu_items(&menu, hmenu);
        let _ = DestroyMenu(hmenu);

        GetShellContextMenuResult {
            success: true,
            items,
            error: None,
        }
    }

    unsafe fn invoke_command(
        path: &str,
        command_id: u32,
        command_verb: Option<&str>,
    ) -> InvokeShellMenuResult {
        let (menu, _pidl) = match create_context_menu(path) {
            Ok(pair) => pair,
            Err(error) => {
                return InvokeShellMenuResult {
                    success: false,
                    error: Some(error),
                };
            }
        };

        // QueryContextMenu must run before InvokeCommand so the verb table is populated.
        let hmenu = match CreatePopupMenu() {
            Ok(h) => h,
            Err(err) => {
                return InvokeShellMenuResult {
                    success: false,
                    error: Some(format!("Failed to create popup menu: {err}")),
                };
            }
        };

        let query = menu.QueryContextMenu(hmenu, 0, 1, 0x7FFF, CMF_NORMAL | CMF_EXPLORE);
        if query.is_err() {
            let _ = DestroyMenu(hmenu);
            return InvokeShellMenuResult {
                success: false,
                error: Some(format!("QueryContextMenu failed: {query:?}")),
            };
        }

        let invoke = if command_id == 0 {
            if let Some(verb) = command_verb {
                invoke_by_verb(&menu, verb)
            } else {
                Err(windows::core::Error::new(
                    windows::core::HRESULT(0x80070057u32 as i32),
                    "Invalid command id",
                ))
            }
        } else {
            invoke_by_id(&menu, command_id).or_else(|err| {
                if let Some(verb) = command_verb {
                    invoke_by_verb(&menu, verb).map_err(|_| err)
                } else {
                    Err(err)
                }
            })
        };

        let _ = DestroyMenu(hmenu);

        match invoke {
            Ok(()) => InvokeShellMenuResult {
                success: true,
                error: None,
            },
            Err(err) => InvokeShellMenuResult {
                success: false,
                error: Some(format!("InvokeCommand failed: {err}")),
            },
        }
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

    unsafe fn extract_menu_items(
        context_menu: &IContextMenu,
        hmenu: windows::Win32::UI::WindowsAndMessaging::HMENU,
    ) -> Vec<ShellContextMenuItem> {
        let mut items: Vec<ShellContextMenuItem> = Vec::new();
        let menu_count = GetMenuItemCount(Some(hmenu));

        for menu_index in 0..menu_count {
            let mut text_buffer: [u16; 256] = [0; 256];
            let mut mii: MENUITEMINFOW = std::mem::zeroed();
            mii.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
            mii.fMask = MIIM_ID | MIIM_STRING | MIIM_SUBMENU;
            mii.dwTypeData = windows::core::PWSTR(text_buffer.as_mut_ptr());
            mii.cch = text_buffer.len() as u32;

            if GetMenuItemInfoW(hmenu, menu_index as u32, true, &mut mii).is_err() {
                continue;
            }

            let text_len = text_buffer.iter().position(|&c| c == 0).unwrap_or(0);
            if text_len == 0 {
                continue;
            }

            let menu_text = String::from_utf16_lossy(&text_buffer[..text_len]).replace('&', "");
            if menu_text.is_empty() {
                continue;
            }

            if is_excluded_menu_title(&menu_text) {
                continue;
            }

            if !mii.hSubMenu.is_invalid() {
                let children = extract_menu_items(context_menu, mii.hSubMenu);
                if !children.is_empty() {
                    items.push(ShellContextMenuItem {
                        id: 0,
                        name: menu_text,
                        verb: None,
                        children: Some(children),
                    });
                }
                continue;
            }

            let menu_id = mii.wID;
            let verb = read_verb(context_menu, menu_id);

            if let Some(ref verb_str) = verb {
                let verb_lower = verb_str.to_lowercase();
                if EXCLUDED_VERBS
                    .iter()
                    .any(|&excluded| verb_lower == excluded)
                {
                    continue;
                }
                if UNSUPPORTED_VERBS
                    .iter()
                    .any(|&unsupported| verb_lower == unsupported)
                {
                    continue;
                }
            }

            items.push(ShellContextMenuItem {
                id: menu_id,
                name: menu_text,
                verb,
                children: None,
            });
        }

        items
    }

    fn is_excluded_menu_title(menu_text: &str) -> bool {
        matches!(
            menu_text.to_lowercase().as_str(),
            "pin to quick access" | "unpin from quick access"
        )
    }

    unsafe fn read_verb(context_menu: &IContextMenu, menu_id: u32) -> Option<String> {
        let mut verb_buffer: [u8; 256] = [0; 256];
        if context_menu
            .GetCommandString(
                (menu_id.saturating_sub(1)) as usize,
                GCS_VERBA,
                None,
                PSTR::from_raw(verb_buffer.as_mut_ptr()),
                verb_buffer.len() as u32,
            )
            .is_err()
        {
            return None;
        }
        let verb_len = verb_buffer.iter().position(|&c| c == 0).unwrap_or(0);
        if verb_len == 0 {
            return None;
        }
        Some(String::from_utf8_lossy(&verb_buffer[..verb_len]).to_string())
    }

    unsafe fn invoke_by_id(menu: &IContextMenu, command_id: u32) -> windows::core::Result<()> {
        let command_index = command_id.checked_sub(1).ok_or_else(|| {
            windows::core::Error::new(
                windows::core::HRESULT(0x80070057u32 as i32),
                "Invalid command id",
            )
        })?;

        let hwnd = GetForegroundWindow();
        let mut invoke_info: CMINVOKECOMMANDINFOEX = std::mem::zeroed();
        invoke_info.cbSize = std::mem::size_of::<CMINVOKECOMMANDINFOEX>() as u32;
        invoke_info.hwnd = hwnd;
        invoke_info.lpVerb = windows::core::PCSTR(command_index as usize as *const u8);
        invoke_info.nShow = SW_SHOWNORMAL.0;

        menu.InvokeCommand(&invoke_info as *const _ as *const _)
    }

    unsafe fn invoke_by_verb(menu: &IContextMenu, command_verb: &str) -> windows::core::Result<()> {
        let hwnd = GetForegroundWindow();
        let verb_ansi = std::ffi::CString::new(command_verb).map_err(|_| {
            windows::core::Error::new(
                windows::core::HRESULT(0x80070057u32 as i32),
                "Invalid command verb",
            )
        })?;

        let mut invoke_info: CMINVOKECOMMANDINFOEX = std::mem::zeroed();
        invoke_info.cbSize = std::mem::size_of::<CMINVOKECOMMANDINFOEX>() as u32;
        invoke_info.hwnd = hwnd;
        invoke_info.lpVerb = windows::core::PCSTR(verb_ansi.as_ptr() as *const u8);
        invoke_info.nShow = SW_SHOWNORMAL.0;

        menu.InvokeCommand(&invoke_info as *const _ as *const _)
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

        // WebView 传入的是视口坐标；悬停级联时用光标屏幕坐标更稳。
        let _ = (x, y);
        let mut point = POINT { x: 0, y: 0 };
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

    #[test]
    fn get_missing_path_fails_cleanly() {
        let result =
            shell_get_context_menu("Z:/definitely-missing-a-file-manager-path-xyz".to_string());
        assert!(!result.success);
        assert!(result.items.is_empty());
        assert!(result.error.is_some());
    }

    #[test]
    fn get_empty_path_fails_cleanly() {
        let result = shell_get_context_menu(String::new());
        assert!(!result.success);
        assert!(result.items.is_empty());
        assert!(result.error.is_some());
    }

    #[test]
    fn invoke_missing_path_fails_cleanly() {
        let result = shell_invoke_context_menu_item(
            "Z:/definitely-missing-a-file-manager-path-xyz".to_string(),
            1,
            None,
        );
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[cfg(windows)]
    #[test]
    fn get_existing_temp_file_returns_items() {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("afm-shell-menu-{stamp}.txt"));
        fs::write(&path, b"afm shell menu probe").expect("write temp file");
        let path_str = path.to_string_lossy().to_string();

        let result = shell_get_context_menu(path_str);
        let _ = fs::remove_file(&path);

        assert!(
            result.success,
            "expected success, got error={:?}",
            result.error
        );
        assert!(
            !result.items.is_empty(),
            "expected at least one shell menu item"
        );
    }
}
