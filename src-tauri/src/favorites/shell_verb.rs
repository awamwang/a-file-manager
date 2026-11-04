// SPDX-License-Identifier: GPL-3.0-or-later
//! Explorer HKCU shell verb "Add to AFM favorites" (AIA-67).
//! Static registry command (not an in-proc COM IContextMenu DLL).

use serde::Serialize;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

pub const AFM_ADD_FAVORITE_FLAG: &str = "--afm-add-favorite";
const VERB_KEY: &str = "AFileManager.AddFavorite";
const VERB_LABEL: &str = "Add to AFM favorites";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellVerbStatus {
    pub registered: bool,
    pub note: String,
}

fn to_wide(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn current_exe() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|err| format!("Failed to resolve exe: {err}"))
}

fn command_line(exe: &std::path::Path) -> String {
    format!("\"{}\" {} \"%1\"", exe.display(), AFM_ADD_FAVORITE_FLAG)
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
fn write_verb_tree(root_relative: &str, command: &str) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_SET_VALUE,
        REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    let verb_path = format!(r"Software\Classes\{root_relative}\shell\{VERB_KEY}");
    let cmd_path = format!(r"{verb_path}\command");

    unsafe {
        // Verb key + default label
        {
            let wide = to_wide(&verb_path);
            let mut hkey = windows::Win32::System::Registry::HKEY::default();
            let mut disposition =
                windows::Win32::System::Registry::REG_CREATE_KEY_DISPOSITION::default();
            win32_ok(RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(wide.as_ptr()),
                Some(0),
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut hkey,
                Some(&mut disposition),
            ))?;
            let label = to_wide(VERB_LABEL);
            let bytes = std::slice::from_raw_parts(
                label.as_ptr() as *const u8,
                label.len() * 2,
            );
            win32_ok(RegSetValueExW(
                hkey,
                PCWSTR::null(),
                Some(0),
                REG_SZ,
                Some(bytes),
            ))?;
            let _ = RegCloseKey(hkey);
        }
        // command subkey
        {
            let wide = to_wide(&cmd_path);
            let mut hkey = windows::Win32::System::Registry::HKEY::default();
            let mut disposition =
                windows::Win32::System::Registry::REG_CREATE_KEY_DISPOSITION::default();
            win32_ok(RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(wide.as_ptr()),
                Some(0),
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut hkey,
                Some(&mut disposition),
            ))?;
            let cmd = to_wide(command);
            let bytes = std::slice::from_raw_parts(cmd.as_ptr() as *const u8, cmd.len() * 2);
            win32_ok(RegSetValueExW(
                hkey,
                PCWSTR::null(),
                Some(0),
                REG_SZ,
                Some(bytes),
            ))?;
            let _ = RegCloseKey(hkey);
        }
    }
    Ok(())
}

#[cfg(windows)]
fn delete_verb_tree(root_relative: &str) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows::Win32::System::Registry::{
        RegDeleteTreeW, RegOpenKeyExW, RegCloseKey, HKEY_CURRENT_USER, KEY_ALL_ACCESS,
    };

    let parent = format!(r"Software\Classes\{root_relative}\shell");
    let wide_parent = to_wide(&parent);
    let wide_verb = to_wide(VERB_KEY);
    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        let open = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(wide_parent.as_ptr()),
            Some(0),
            KEY_ALL_ACCESS,
            &mut hkey,
        );
        if open == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        win32_ok(open)?;
        let del = RegDeleteTreeW(hkey, PCWSTR(wide_verb.as_ptr()));
        let _ = RegCloseKey(hkey);
        if del == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        win32_ok(del)?;
    }
    Ok(())
}

#[cfg(windows)]
fn verb_registered() -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, HKEY_CURRENT_USER, KEY_QUERY_VALUE,
    };

    let path = format!(r"Software\Classes\*\shell\{VERB_KEY}\command");
    let wide = to_wide(&path);
    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        let code = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(wide.as_ptr()),
            Some(0),
            KEY_QUERY_VALUE,
            &mut hkey,
        );
        if code == ERROR_SUCCESS {
            let _ = RegCloseKey(hkey);
            true
        } else {
            false
        }
    }
}

pub fn status() -> ShellVerbStatus {
    #[cfg(windows)]
    {
        let registered = verb_registered();
        ShellVerbStatus {
            registered,
            note: if registered {
                "HKCU shell verb registered for files (*) and folders (Directory).".to_string()
            } else {
                "Shell verb not registered. Call favorites_os_hook_set_enabled(true).".to_string()
            },
        }
    }
    #[cfg(not(windows))]
    {
        ShellVerbStatus {
            registered: false,
            note: "Shell verb is Windows-only.".to_string(),
        }
    }
}

pub fn set_enabled(enabled: bool) -> Result<ShellVerbStatus, String> {
    #[cfg(windows)]
    {
        if enabled {
            let exe = current_exe()?;
            let cmd = command_line(&exe);
            write_verb_tree("*", &cmd)?;
            write_verb_tree("Directory", &cmd)?;
        } else {
            delete_verb_tree("*")?;
            delete_verb_tree("Directory")?;
        }
        Ok(status())
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Err("Shell verb is Windows-only".to_string())
    }
}

/// Parse `--afm-add-favorite <path>` from argv.
pub fn take_add_favorite_path(args: &[String]) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == AFM_ADD_FAVORITE_FLAG {
            return iter.next().cloned();
        }
        if let Some(rest) = arg.strip_prefix(&format!("{AFM_ADD_FAVORITE_FLAG}=")) {
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flag_path() {
        let args = vec![
            "afm.exe".into(),
            AFM_ADD_FAVORITE_FLAG.into(),
            r"C:\Docs\a.txt".into(),
        ];
        assert_eq!(
            take_add_favorite_path(&args).as_deref(),
            Some(r"C:\Docs\a.txt")
        );
    }

    #[test]
    fn command_line_quotes_exe() {
        let cmd = command_line(std::path::Path::new(r"C:\Program Files\afm.exe"));
        assert!(cmd.contains(AFM_ADD_FAVORITE_FLAG));
        assert!(cmd.contains("\"%1\""));
    }
}
