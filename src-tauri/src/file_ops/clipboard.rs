// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows shell clipboard (CF_HDROP + Preferred DropEffect) so copy/cut/paste
//! interoperate with File Explorer.

use serde::Serialize;

const DROPEFFECT_COPY: u32 = 1;
const DROPEFFECT_MOVE: u32 = 2;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellClipboard {
    pub paths: Vec<String>,
    pub mode: Option<String>,
}

pub fn encode_hdrop(paths: &[String]) -> Vec<u8> {
    let mut body = Vec::new();
    for path in paths {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            continue;
        }
        let wide: Vec<u16> = trimmed.encode_utf16().chain(std::iter::once(0)).collect();
        for unit in wide {
            body.extend_from_slice(&unit.to_le_bytes());
        }
    }
    body.extend_from_slice(&0u16.to_le_bytes());

    let mut out = Vec::with_capacity(20 + body.len());
    out.extend_from_slice(&20u32.to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes());
    out.extend_from_slice(&1i32.to_le_bytes());
    out.extend(body);
    out
}

pub fn decode_hdrop(bytes: &[u8]) -> Vec<String> {
    if bytes.len() < 20 {
        return Vec::new();
    }
    let offset = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let wide = i32::from_le_bytes(bytes[16..20].try_into().unwrap()) != 0;
    if offset >= bytes.len() {
        return Vec::new();
    }
    let payload = &bytes[offset..];
    if wide {
        decode_wide_list(payload)
    } else {
        decode_ansi_list(payload)
    }
}

fn decode_wide_list(payload: &[u8]) -> Vec<String> {
    let mut units = Vec::with_capacity(payload.len() / 2);
    let mut i = 0;
    while i + 1 < payload.len() {
        units.push(u16::from_le_bytes([payload[i], payload[i + 1]]));
        i += 2;
    }
    let mut paths = Vec::new();
    let mut start = 0;
    for (idx, unit) in units.iter().enumerate() {
        if *unit != 0 {
            continue;
        }
        if idx == start {
            break;
        }
        if let Ok(text) = String::from_utf16(&units[start..idx]) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                paths.push(trimmed.to_string());
            }
        }
        start = idx + 1;
    }
    paths
}

fn decode_ansi_list(payload: &[u8]) -> Vec<String> {
    let mut paths = Vec::new();
    let mut start = 0;
    for (idx, byte) in payload.iter().enumerate() {
        if *byte != 0 {
            continue;
        }
        if idx == start {
            break;
        }
        let text = String::from_utf8_lossy(&payload[start..idx]);
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            paths.push(trimmed.to_string());
        }
        start = idx + 1;
    }
    paths
}

pub fn mode_to_effect(mode: &str) -> u32 {
    if mode == "cut" {
        DROPEFFECT_MOVE
    } else {
        DROPEFFECT_COPY
    }
}

pub fn effect_to_mode(effect: u32) -> String {
    if effect & DROPEFFECT_MOVE != 0 && effect & DROPEFFECT_COPY == 0 {
        "cut".to_string()
    } else {
        "copy".to_string()
    }
}

#[cfg(windows)]
pub fn set_shell_clipboard(paths: &[String], mode: &str) -> Result<(), String> {
    let cleaned: Vec<String> = paths
        .iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    if cleaned.is_empty() {
        return clear_shell_clipboard();
    }
    let bytes = encode_hdrop(&cleaned);
    let effect = mode_to_effect(mode);
    with_clipboard(|| unsafe { write_formats(&bytes, effect) })
}

#[cfg(windows)]
pub fn clear_shell_clipboard() -> Result<(), String> {
    with_clipboard(|| unsafe {
        use windows::Win32::System::DataExchange::EmptyClipboard;
        EmptyClipboard().map_err(|err| err.to_string())
    })
}

#[cfg(windows)]
pub fn get_shell_clipboard() -> Result<ShellClipboard, String> {
    with_clipboard(|| unsafe { read_formats() })
}

#[cfg(not(windows))]
pub fn set_shell_clipboard(_paths: &[String], _mode: &str) -> Result<(), String> {
    Err("Shell clipboard is only available on Windows".to_string())
}

#[cfg(not(windows))]
pub fn clear_shell_clipboard() -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn get_shell_clipboard() -> Result<ShellClipboard, String> {
    Ok(ShellClipboard {
        paths: Vec::new(),
        mode: None,
    })
}

#[cfg(windows)]
fn with_clipboard<T>(body: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    use windows::Win32::System::DataExchange::{CloseClipboard, OpenClipboard};
    let mut last = "Clipboard is busy".to_string();
    for _ in 0..8 {
        let opened = unsafe { OpenClipboard(None) };
        if opened.is_ok() {
            let result = body();
            unsafe {
                let _ = CloseClipboard();
            }
            return result;
        }
        last = opened.unwrap_err().to_string();
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    Err(last)
}

#[cfg(windows)]
const CF_HDROP: u32 = 15;

#[cfg(windows)]
unsafe fn write_formats(hdrop: &[u8], effect: u32) -> Result<(), String> {
    use windows::Win32::Foundation::{GlobalFree, HANDLE};
    use windows::Win32::System::DataExchange::{
        EmptyClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows::core::w;

    unsafe {
        EmptyClipboard().map_err(|err| err.to_string())?;
        let drop_handle = alloc_global(hdrop)?;
        if SetClipboardData(CF_HDROP, Some(HANDLE(drop_handle.0))).is_err() {
            let _ = GlobalFree(Some(drop_handle));
            return Err("Failed to set CF_HDROP".to_string());
        }
        let effect_bytes = effect.to_le_bytes();
        let effect_handle = alloc_global(&effect_bytes)?;
        let format = RegisterClipboardFormatW(w!("Preferred DropEffect"));
        if format == 0 || SetClipboardData(format, Some(HANDLE(effect_handle.0))).is_err() {
            let _ = GlobalFree(Some(effect_handle));
            return Err("Failed to set Preferred DropEffect".to_string());
        }
        Ok(())
    }
}

#[cfg(windows)]
unsafe fn alloc_global(bytes: &[u8]) -> Result<windows::Win32::Foundation::HGLOBAL, String> {
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len()).map_err(|err| err.to_string())?;
        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            return Err("Failed to lock clipboard memory".to_string());
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
        let _ = GlobalUnlock(handle);
        Ok(handle)
    }
}

#[cfg(windows)]
unsafe fn read_formats() -> Result<ShellClipboard, String> {
    use windows::Win32::System::DataExchange::{GetClipboardData, RegisterClipboardFormatW};
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows::core::w;

    unsafe {
        let handle = match GetClipboardData(CF_HDROP) {
            Ok(handle) if !handle.is_invalid() => handle,
            _ => {
                return Ok(ShellClipboard {
                    paths: Vec::new(),
                    mode: None,
                });
            }
        };
        let hglobal = windows::Win32::Foundation::HGLOBAL(handle.0);
        let size = GlobalSize(hglobal);
        let ptr = GlobalLock(hglobal);
        if ptr.is_null() || size == 0 {
            return Ok(ShellClipboard {
                paths: Vec::new(),
                mode: None,
            });
        }
        let bytes = std::slice::from_raw_parts(ptr as *const u8, size);
        let paths = decode_hdrop(bytes);
        let _ = GlobalUnlock(hglobal);

        let format = RegisterClipboardFormatW(w!("Preferred DropEffect"));
        let mode = if format == 0 || paths.is_empty() {
            None
        } else {
            match GetClipboardData(format) {
                Ok(effect_handle) if !effect_handle.is_invalid() => {
                    let effect_global = windows::Win32::Foundation::HGLOBAL(effect_handle.0);
                    let effect_ptr = GlobalLock(effect_global);
                    let effect = if effect_ptr.is_null() {
                        DROPEFFECT_COPY
                    } else {
                        let raw = std::slice::from_raw_parts(effect_ptr as *const u8, 4.min(GlobalSize(effect_global)));
                        let mut buf = [0u8; 4];
                        let n = raw.len().min(4);
                        buf[..n].copy_from_slice(&raw[..n]);
                        let _ = GlobalUnlock(effect_global);
                        u32::from_le_bytes(buf)
                    };
                    Some(effect_to_mode(effect))
                }
                _ => Some("copy".to_string()),
            }
        };
        Ok(ShellClipboard { paths, mode })
    }
}

#[tauri::command]
pub fn file_ops_clipboard_set(paths: Vec<String>, mode: String) -> Result<(), String> {
    set_shell_clipboard(&paths, &mode)
}

#[tauri::command]
pub fn file_ops_clipboard_get() -> Result<ShellClipboard, String> {
    get_shell_clipboard()
}

#[tauri::command]
pub fn file_ops_clipboard_clear() -> Result<(), String> {
    clear_shell_clipboard()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdrop_roundtrip_preserves_paths() {
        let paths = vec![
            r"C:\Temp\a.txt".to_string(),
            r"D:\文件夹\b.txt".to_string(),
        ];
        let decoded = decode_hdrop(&encode_hdrop(&paths));
        assert_eq!(decoded, paths);
    }

    #[test]
    fn cut_effect_is_move() {
        assert_eq!(mode_to_effect("cut"), DROPEFFECT_MOVE);
        assert_eq!(effect_to_mode(DROPEFFECT_MOVE), "cut");
        assert_eq!(effect_to_mode(DROPEFFECT_COPY), "copy");
    }
}
