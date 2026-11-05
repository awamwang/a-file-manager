// SPDX-License-Identifier: GPL-3.0-or-later
//! Native Windows "Open with" dialog (AIA-58). Day-1: OS dialog only (no app list submenu).

use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenWithResult {
    pub success: bool,
    pub error: Option<String>,
}

/// Open a file/folder with the OS default association (ShellExecute "open").
/// Prefer this over `plugin-opener` for arbitrary FS paths (opener path ACL).
#[tauri::command]
pub fn open_path_default(path: String) -> OpenWithResult {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return OpenWithResult {
            success: false,
            error: Some("Path is empty".to_string()),
        };
    }
    let p = Path::new(trimmed);
    if !p.exists() {
        return OpenWithResult {
            success: false,
            error: Some(format!("Path not found: {trimmed}")),
        };
    }
    #[cfg(windows)]
    {
        windows_impl::open_default(trimmed)
    }
    #[cfg(not(windows))]
    {
        OpenWithResult {
            success: false,
            error: Some("open_path_default is only supported on Windows".to_string()),
        }
    }
}

#[tauri::command]
pub fn open_native_open_with_dialog(path: String) -> OpenWithResult {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return OpenWithResult {
            success: false,
            error: Some("Path is empty".to_string()),
        };
    }
    let p = Path::new(trimmed);
    if !p.exists() {
        return OpenWithResult {
            success: false,
            error: Some(format!("Path not found: {trimmed}")),
        };
    }
    // Files use the native Open With dialog; folders are accepted so callers can
    // fall back to IDE launchers / Explorer when the OS dialog is unsuitable.
    #[cfg(windows)]
    {
        if p.is_dir() {
            return windows_impl::open_folder_in_explorer(trimmed);
        }
        windows_impl::open_as(trimmed)
    }
    #[cfg(not(windows))]
    {
        OpenWithResult {
            success: false,
            error: Some("Native Open With dialog is only supported on Windows".to_string()),
        }
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::OpenWithResult;
    use crate::win_process;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_ASYNCOK, SEE_MASK_INVOKEIDLIST, SHELLEXECUTEINFOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    pub fn open_folder_in_explorer(path: &str) -> OpenWithResult {
        let absolute = path.replace('/', "\\");
        match win_process::command("explorer.exe").arg(&absolute).spawn() {
            Ok(_) => OpenWithResult {
                success: true,
                error: None,
            },
            Err(err) => OpenWithResult {
                success: false,
                error: Some(format!("Failed to open folder: {err}")),
            },
        }
    }

    pub fn open_default(path: &str) -> OpenWithResult {
        let absolute = path.replace('/', "\\");
        let file_wide: Vec<u16> = OsStr::new(&absolute)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let verb_wide: Vec<u16> = OsStr::new("open")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // Tauri sync commands run off the UI apartment (no message pump).
        // ShellExecuteEx without SEE_MASK_ASYNCOK waits for DDE / async
        // activation to finish — MKV player associations commonly DDE, and a
        // pump-less wait times out around ~30s before returning. Ask the shell
        // to return immediately; Explorer-like open latency.
        unsafe {
            let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
            sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            sei.fMask = SEE_MASK_ASYNCOK;
            sei.lpVerb = PCWSTR(verb_wide.as_ptr());
            sei.lpFile = PCWSTR(file_wide.as_ptr());
            sei.nShow = SW_SHOWNORMAL.0;

            if ShellExecuteExW(&mut sei).is_ok() {
                return OpenWithResult {
                    success: true,
                    error: None,
                };
            }
        }

        // Fallback: cmd start "" "path"
        match win_process::command("cmd.exe")
            .args(["/C", "start", "", &absolute])
            .spawn()
        {
            Ok(_) => OpenWithResult {
                success: true,
                error: None,
            },
            Err(err) => OpenWithResult {
                success: false,
                error: Some(format!("Failed to open path: {err}")),
            },
        }
    }

    pub fn open_as(path: &str) -> OpenWithResult {
        let absolute = path.replace('/', "\\");
        let verb_wide: Vec<u16> = OsStr::new("openas")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let file_wide: Vec<u16> = OsStr::new(&absolute)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
            sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            sei.lpVerb = PCWSTR(verb_wide.as_ptr());
            sei.lpFile = PCWSTR(file_wide.as_ptr());
            sei.nShow = SW_SHOWNORMAL.0;
            sei.fMask = SEE_MASK_INVOKEIDLIST;

            if ShellExecuteExW(&mut sei).is_ok() {
                return OpenWithResult {
                    success: true,
                    error: None,
                };
            }
        }

        match win_process::command("rundll32.exe")
            .args(["shell32.dll,OpenAs_RunDLL", &absolute])
            .spawn()
        {
            Ok(_) => OpenWithResult {
                success: true,
                error: None,
            },
            Err(err) => OpenWithResult {
                success: false,
                error: Some(format!("Failed to open Open With dialog: {err}")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_path_fails() {
        let result = open_native_open_with_dialog(String::new());
        assert!(!result.success);
    }

    #[test]
    fn open_path_default_empty_fails() {
        let result = open_path_default(String::new());
        assert!(!result.success);
    }

    #[test]
    fn open_path_default_missing_fails() {
        let result = open_path_default("Z:/missing-afm-open-default-xyz.bin".into());
        assert!(!result.success);
    }

    #[test]
    fn missing_path_fails() {
        let result = open_native_open_with_dialog(
            "Z:/missing-afm-open-with-xyz.bin".to_string(),
        );
        assert!(!result.success);
    }

    #[test]
    fn directory_accepted_on_windows() {
        let dir = std::env::temp_dir();
        let result = open_native_open_with_dialog(dir.to_string_lossy().into_owned());
        #[cfg(windows)]
        assert!(result.success);
        #[cfg(not(windows))]
        assert!(!result.success);
    }
}
