// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows login autostart via HKCU Run key (AIA-60). No Tauri plugin dependency.

use serde::Serialize;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

pub const AFM_AUTOSTART_CLI_FLAG: &str = "--afm-autostart";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "AFileManager";

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutostartStatus {
    pub enabled: bool,
    pub command: Option<String>,
    pub error: Option<String>,
}

fn to_wide(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn current_exe_path() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|err| format!("Failed to resolve current exe: {err}"))
}

fn build_run_command(exe: &std::path::Path) -> String {
    format!("\"{}\" {}", exe.display(), AFM_AUTOSTART_CLI_FLAG)
}

#[cfg(windows)]
fn win32_ok(code: windows::Win32::Foundation::WIN32_ERROR) -> Result<(), String> {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    if code == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!("Win32 error {}", code.0))
    }
}

#[cfg(windows)]
fn open_run_key(write: bool) -> Result<windows::Win32::System::Registry::HKEY, String> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCreateKeyExW, RegOpenKeyExW, HKEY_CURRENT_USER, KEY_SET_VALUE, KEY_QUERY_VALUE,
        REG_OPTION_NON_VOLATILE,
    };

    let wide = to_wide(RUN_KEY);
    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        if write {
            let mut disposition =
                windows::Win32::System::Registry::REG_CREATE_KEY_DISPOSITION::default();
            let access = KEY_QUERY_VALUE | KEY_SET_VALUE;
            win32_ok(RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(wide.as_ptr()),
                Some(0),
                None,
                REG_OPTION_NON_VOLATILE,
                access,
                None,
                &mut hkey,
                Some(&mut disposition),
            ))?;
        } else {
            win32_ok(RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(wide.as_ptr()),
                Some(0),
                KEY_QUERY_VALUE,
                &mut hkey,
            ))?;
        }
        Ok(hkey)
    }
}

#[cfg(windows)]
fn read_run_value() -> Result<Option<String>, String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows::Win32::System::Registry::{RegCloseKey, RegQueryValueExW, REG_SZ, REG_VALUE_TYPE};

    let hkey = match open_run_key(false) {
        Ok(key) => key,
        Err(_) => return Ok(None),
    };
    let name = to_wide(VALUE_NAME);
    unsafe {
        let mut value_type = REG_VALUE_TYPE::default();
        let mut size = 0u32;
        let first = RegQueryValueExW(
            hkey,
            PCWSTR(name.as_ptr()),
            None,
            Some(&mut value_type),
            None,
            Some(&mut size),
        );
        if first == ERROR_FILE_NOT_FOUND {
            let _ = RegCloseKey(hkey);
            return Ok(None);
        }
        if let Err(err) = win32_ok(first) {
            let _ = RegCloseKey(hkey);
            return Err(format!("RegQueryValueExW size failed: {err}"));
        }

        let mut buffer = vec![0u8; size as usize];
        let second = RegQueryValueExW(
            hkey,
            PCWSTR(name.as_ptr()),
            None,
            Some(&mut value_type),
            Some(buffer.as_mut_ptr()),
            Some(&mut size),
        );
        let _ = RegCloseKey(hkey);
        win32_ok(second).map_err(|err| format!("RegQueryValueExW data failed: {err}"))?;

        if value_type != REG_SZ {
            return Ok(None);
        }
        let u16_len = (size as usize) / 2;
        let words = std::slice::from_raw_parts(buffer.as_ptr() as *const u16, u16_len);
        let trimmed: Vec<u16> = words.iter().copied().take_while(|c| *c != 0).collect();
        Ok(Some(String::from_utf16_lossy(&trimmed)))
    }
}

#[cfg(windows)]
fn write_run_value(command: &str) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{RegCloseKey, RegSetValueExW, REG_SZ};

    let hkey = open_run_key(true)?;
    let name = to_wide(VALUE_NAME);
    let data = to_wide(command);
    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2) };
    unsafe {
        let result = RegSetValueExW(hkey, PCWSTR(name.as_ptr()), Some(0), REG_SZ, Some(bytes));
        let _ = RegCloseKey(hkey);
        win32_ok(result).map_err(|err| format!("RegSetValueExW failed: {err}"))?;
    }
    Ok(())
}

#[cfg(windows)]
fn delete_run_value() -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows::Win32::System::Registry::{RegCloseKey, RegDeleteValueW};

    let hkey = open_run_key(true)?;
    let name = to_wide(VALUE_NAME);
    unsafe {
        let result = RegDeleteValueW(hkey, PCWSTR(name.as_ptr()));
        let _ = RegCloseKey(hkey);
        if result == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        win32_ok(result).map_err(|err| format!("RegDeleteValueW failed: {err}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn autostart_status() -> AutostartStatus {
    #[cfg(windows)]
    {
        match read_run_value() {
            Ok(Some(command)) => AutostartStatus {
                enabled: true,
                command: Some(command),
                error: None,
            },
            Ok(None) => AutostartStatus {
                enabled: false,
                command: None,
                error: None,
            },
            Err(error) => AutostartStatus {
                enabled: false,
                command: None,
                error: Some(error),
            },
        }
    }
    #[cfg(not(windows))]
    {
        AutostartStatus {
            enabled: false,
            command: None,
            error: Some("Autostart is only supported on Windows".to_string()),
        }
    }
}

#[tauri::command]
pub fn autostart_set_enabled(enabled: bool) -> Result<AutostartStatus, String> {
    #[cfg(windows)]
    {
        if enabled {
            let exe = current_exe_path()?;
            let command = build_run_command(&exe);
            write_run_value(&command)?;
        } else {
            delete_run_value()?;
        }
        Ok(autostart_status())
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Err("Autostart is only supported on Windows".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_run_command_includes_flag() {
        let command = build_run_command(std::path::Path::new(r"C:\Apps\a-file-manager.exe"));
        assert!(command.contains(AFM_AUTOSTART_CLI_FLAG));
        assert!(command.contains(r"C:\Apps\a-file-manager.exe"));
    }

    #[test]
    fn status_returns_without_panic() {
        let status = autostart_status();
        assert!(status.error.is_none() || !status.error.as_ref().unwrap().is_empty());
    }
}
