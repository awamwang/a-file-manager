// SPDX-License-Identifier: GPL-3.0-or-later
//! Plain-text system clipboard (CF_UNICODETEXT) for vault copy (ADR 0016).
//!
//! 明文只经由这里直写系统剪贴板，不进前端状态、不进日志。
//! Windows 走 Win32 原生；其它平台回退 pbcopy / wl-copy / xclip。

/// Write plain text to the system clipboard.
pub fn set_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Err("refusing to write empty text to clipboard".to_string());
    }
    #[cfg(windows)]
    {
        set_text_windows(text)
    }
    #[cfg(not(windows))]
    {
        set_text_unix(text)
    }
}

#[cfg(windows)]
const CF_UNICODETEXT: u32 = 13;

#[cfg(windows)]
fn set_text_windows(text: &str) -> Result<(), String> {
    use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData};
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::Foundation::HANDLE;

    // NUL-terminated UTF-16, matching CF_UNICODETEXT contract.
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes: Vec<u8> = wide.iter().flat_map(|u| u.to_le_bytes()).collect();

    // Clipboard may be held by another process; retry like the shell path.
    let mut last = "Clipboard is busy".to_string();
    for _ in 0..8 {
        let opened = unsafe { OpenClipboard(None) };
        if opened.is_ok() {
            let result = unsafe {
                EmptyClipboard().map_err(|err| err.to_string())?;
                let handle =
                    GlobalAlloc(GMEM_MOVEABLE, bytes.len()).map_err(|err| err.to_string())?;
                let ptr = GlobalLock(handle);
                if ptr.is_null() {
                    return Err("Failed to lock clipboard memory".to_string());
                }
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
                let _ = GlobalUnlock(handle);
                if SetClipboardData(CF_UNICODETEXT, Some(HANDLE(handle.0))).is_err() {
                    return Err("Failed to set CF_UNICODETEXT".to_string());
                }
                Ok(())
            };
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

#[cfg(not(windows))]
fn set_text_unix(text: &str) -> Result<(), String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let candidates: [&str; 3] = ["pbcopy", "wl-copy", "xclip"];
    let mut last = "no clipboard tool found (pbcopy/wl-copy/xclip)".to_string();
    for tool in candidates {
        let mut child = Command::new(tool)
            .arg("-selection")
            .arg("clipboard")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| err.to_string())?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(text.as_bytes()).map_err(|err| err.to_string())?;
        }
        match child.wait() {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => last = format!("{tool} exited with {status}"),
            Err(err) => last = err.to_string(),
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_text() {
        assert!(set_text("").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_roundtrip_write() {
        // 本机冒烟：写一段文本并读回校验（GetClipboardData CF_UNICODETEXT）。
        use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
        use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
        use windows::Win32::Foundation::HGLOBAL;

        let text = "afm-clipboard-test-✓";
        assert!(set_text(text).is_ok());
        // 剪贴板可能被其它进程（剪贴板管理器等）短暂持有；与 set_text 一致地重试开板，避免偶发占用导致测试抖动。
        let mut opened = Err("Clipboard is busy".to_string());
        for _ in 0..8 {
            if let Ok(()) = unsafe { OpenClipboard(None) } {
                opened = Ok(());
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        assert!(opened.is_ok());
        let result = unsafe {
            let read = (|| {
                let handle = GetClipboardData(CF_UNICODETEXT).map_err(|e| e.to_string())?;
                let hglobal = HGLOBAL(handle.0);
                let size = GlobalSize(hglobal);
                let ptr = GlobalLock(hglobal);
                if ptr.is_null() || size < 2 {
                    return Err("lock/size failed".to_string());
                }
                let units = std::slice::from_raw_parts(ptr as *const u16, size / 2);
                let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
                let text = String::from_utf16(&units[..end]).map_err(|e| e.to_string())?;
                let _ = GlobalUnlock(hglobal);
                Ok(text)
            })();
            let _ = CloseClipboard();
            read
        };
        assert_eq!(result.unwrap(), text);
    }
}
