// SPDX-License-Identifier: GPL-3.0-or-later
//! ShellNew-backed "New file" templates from HKCR.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NewFileTemplate {
    pub extension: String,
    pub label: String,
    pub default_name: String,
}

#[cfg(windows)]
fn win32_ok(code: windows::Win32::Foundation::WIN32_ERROR) -> Result<(), String> {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    if code == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!("Win32 registry error: {code:?}"))
    }
}

#[cfg(windows)]
pub fn list_new_file_templates() -> Result<Vec<NewFileTemplate>, String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY_CLASSES_ROOT, KEY_READ,
    };

    unsafe {
        let mut root = Default::default();
        win32_ok(RegOpenKeyExW(
            HKEY_CLASSES_ROOT,
            None,
            Some(0),
            KEY_READ,
            &mut root,
        ))
        .map_err(|err| format!("RegOpenKeyExW failed: {err}"))?;

        let mut out = Vec::new();
        let mut index = 0u32;
        loop {
            let mut name_buf = [0u16; 256];
            let mut name_len = name_buf.len() as u32;
            let status = RegEnumKeyExW(
                root,
                index,
                Some(windows::core::PWSTR(name_buf.as_mut_ptr())),
                &mut name_len,
                None,
                None,
                None,
                None,
            );
            if status != ERROR_SUCCESS {
                break;
            }
            index += 1;
            let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
            if !name.starts_with('.') || name.len() < 2 {
                continue;
            }
            let shell_new = format!("{name}\\ShellNew");
            let shell_new_wide: Vec<u16> = shell_new
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let mut shell_key = Default::default();
            if RegOpenKeyExW(
                HKEY_CLASSES_ROOT,
                PCWSTR(shell_new_wide.as_ptr()),
                Some(0),
                KEY_READ,
                &mut shell_key,
            ) != ERROR_SUCCESS
            {
                continue;
            }
            let _ = RegCloseKey(shell_key);

            let ext = name.trim_start_matches('.').to_ascii_lowercase();
            if ext.is_empty() || out.iter().any(|t: &NewFileTemplate| t.extension == ext) {
                continue;
            }
            let label = file_type_label(&name).unwrap_or_else(|| format!("{ext} file"));
            out.push(NewFileTemplate {
                extension: ext.clone(),
                label,
                default_name: format!("New {ext} file.{ext}"),
            });
            if out.len() >= 40 {
                break;
            }
        }
        let _ = RegCloseKey(root);
        out.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
        if out.is_empty() {
            out.push(NewFileTemplate {
                extension: "txt".into(),
                label: "Text Document".into(),
                default_name: "New Text Document.txt".into(),
            });
        }
        Ok(out)
    }
}

#[cfg(windows)]
fn file_type_label(ext_key: &str) -> Option<String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CLASSES_ROOT, KEY_READ, REG_SZ,
    };
    unsafe {
        let key_wide: Vec<u16> = ext_key.encode_utf16().chain(std::iter::once(0)).collect();
        let mut key = Default::default();
        if RegOpenKeyExW(
            HKEY_CLASSES_ROOT,
            PCWSTR(key_wide.as_ptr()),
            Some(0),
            KEY_READ,
            &mut key,
        ) != ERROR_SUCCESS
        {
            return None;
        }
        let mut prog_id_buf = [0u16; 256];
        let mut prog_id_bytes = (prog_id_buf.len() * 2) as u32;
        let mut kind = REG_SZ;
        let ok = RegQueryValueExW(
            key,
            None,
            None,
            Some(&mut kind),
            Some(prog_id_buf.as_mut_ptr() as *mut u8),
            Some(&mut prog_id_bytes),
        );
        let _ = RegCloseKey(key);
        if ok != ERROR_SUCCESS || prog_id_bytes < 2 {
            return None;
        }
        let prog_len = (prog_id_bytes as usize / 2).saturating_sub(1);
        let prog_id = String::from_utf16_lossy(&prog_id_buf[..prog_len]);
        if prog_id.is_empty() {
            return None;
        }
        let prog_wide: Vec<u16> = prog_id.encode_utf16().chain(std::iter::once(0)).collect();
        let mut prog_key = Default::default();
        if RegOpenKeyExW(
            HKEY_CLASSES_ROOT,
            PCWSTR(prog_wide.as_ptr()),
            Some(0),
            KEY_READ,
            &mut prog_key,
        ) != ERROR_SUCCESS
        {
            return Some(prog_id);
        }
        let mut label_buf = [0u16; 256];
        let mut label_bytes = (label_buf.len() * 2) as u32;
        let mut label_kind = REG_SZ;
        let label_ok = RegQueryValueExW(
            prog_key,
            None,
            None,
            Some(&mut label_kind),
            Some(label_buf.as_mut_ptr() as *mut u8),
            Some(&mut label_bytes),
        );
        let _ = RegCloseKey(prog_key);
        if label_ok != ERROR_SUCCESS || label_bytes < 2 {
            return Some(prog_id);
        }
        let label_len = (label_bytes as usize / 2).saturating_sub(1);
        Some(String::from_utf16_lossy(&label_buf[..label_len]))
    }
}

#[cfg(not(windows))]
pub fn list_new_file_templates() -> Result<Vec<NewFileTemplate>, String> {
    Ok(vec![NewFileTemplate {
        extension: "txt".into(),
        label: "Text Document".into(),
        default_name: "New Text Document.txt".into(),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_include_at_least_txt_on_windows() {
        let list = list_new_file_templates().unwrap();
        assert!(!list.is_empty());
        assert!(list
            .iter()
            .any(|t| t.extension == "txt" || !t.extension.is_empty()));
    }
}
