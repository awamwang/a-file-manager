// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows shell clipboard (CF_HDROP + Preferred DropEffect) so copy/cut/paste
//! interoperate with File Explorer.

// 7.24: the payload DTO is neutral and lives in `platform/`; this module is
// the Windows CF_HDROP backend behind `ClipboardProvider`.
pub use crate::platform::ShellClipboard;

const DROPEFFECT_COPY: u32 = 1;
const DROPEFFECT_MOVE: u32 = 2;

// Cross-platform HDROP byte contract (used by tests + Windows clipboard paths);
// only dead in non-test lib builds on non-Windows targets.
#[cfg_attr(not(test), allow(dead_code))]
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

#[cfg_attr(not(test), allow(dead_code))]
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

#[cfg_attr(not(test), allow(dead_code))]
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

#[cfg_attr(not(test), allow(dead_code))]
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

#[cfg_attr(not(test), allow(dead_code))]
pub fn mode_to_effect(mode: &str) -> u32 {
    if mode == "cut" {
        DROPEFFECT_MOVE
    } else {
        DROPEFFECT_COPY
    }
}

#[cfg_attr(not(test), allow(dead_code))]
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
    let effect = mode_to_effect(mode);
    // Remote-desktop clipboard brokers (RustDesk / MS-RDPECLIP Cliprdr) read the
    // *contents* of copied files through CFSTR_FILEDESCRIPTOR + CFSTR_FILECONTENTS
    // (lazy, per-index), not the CF_HDROP path list — a path is meaningless on the
    // far machine. The Shell `BHID_DataObject` already serves CF_HDROP *and* the
    // file-content formats, so publishing it as the clipboard owner is what makes
    // AFM's copy usable as a remote source. Same object Explorer relies on (FIL-38).
    if publish_shell_data_object(&cleaned, effect).is_ok() {
        return Ok(());
    }
    // Fallback: keep the legacy CF_HDROP + Preferred DropEffect hand-written
    // clipboard so same-machine paste with File Explorer never regresses if the
    // Shell object could not be built (e.g. an item the shell cannot parse).
    let bytes = encode_hdrop(&cleaned);
    with_clipboard(|| unsafe { write_formats(&bytes, effect) })
}

/// Initialize COM + OLE on the current apartment. The clipboard entry points
/// run on Tauri worker threads, and `OleSetClipboard` / `OleGetClipboard`
/// require `OleInitialize` (plain `CoInitializeEx` is not enough — OLE clipboard
/// calls then fail and the publish silently degrades to a bare CF_HDROP). Both
/// calls are reference-counted and idempotent; we never uninitialize (host owns).
#[cfg(windows)]
fn ensure_ole() {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
    use windows::Win32::System::Ole::OleInitialize;
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let _ = OleInitialize(None);
    }
}

/// Build a Shell file data object over `paths` and hand it to the clipboard via
/// `OleSetClipboard`, tagging the preferred drop effect. On success OLE owns the
/// object and external brokers can pull `CFSTR_FILECONTENTS` lazily.
#[cfg(windows)]
fn publish_shell_data_object(paths: &[String], effect: u32) -> Result<(), String> {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::GlobalFree;
    use windows::Win32::System::Com::{
        IDataObject, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL, DVASPECT_CONTENT, FORMATETC,
    };
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::System::Ole::OleSetClipboard;
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows::Win32::UI::Shell::{
        BHID_DataObject, ILCreateFromPathW, ILFree, SHCreateShellItemArrayFromIDLists,
    };

    unsafe {
        // OleSetClipboard needs OLE initialized on this apartment.
        ensure_ole();

        let mut idlists: Vec<*const ITEMIDLIST> = Vec::with_capacity(paths.len());
        let mut wide_paths: Vec<Vec<u16>> = Vec::with_capacity(paths.len());
        for path in paths {
            let mut wide: Vec<u16> = path.encode_utf16().collect();
            wide.push(0);
            let item = ILCreateFromPathW(PCWSTR(wide.as_ptr()));
            if item.is_null() {
                for released in idlists.iter() {
                    ILFree(Some(*released));
                }
                return Err("Shell could not resolve a copied path".to_string());
            }
            wide_paths.push(wide);
            idlists.push(item.cast_const());
        }

        let array = SHCreateShellItemArrayFromIDLists(&idlists);
        for released in idlists.iter() {
            ILFree(Some(*released));
        }
        let array = array.map_err(|err| err.to_string())?;
        let data_object: IDataObject = array
            .BindToHandler(None, &BHID_DataObject)
            .map_err(|err| err.to_string())?;

        let drop_format = RegisterClipboardFormatW(w!("Preferred DropEffect")) as u16;
        if drop_format != 0 {
            let format_etc = FORMATETC {
                cfFormat: drop_format,
                ptd: std::ptr::null_mut(),
                dwAspect: DVASPECT_CONTENT.0,
                lindex: -1,
                tymed: TYMED_HGLOBAL.0 as u32,
            };
            let handle = GlobalAlloc(GMEM_MOVEABLE, std::mem::size_of::<u32>())
                .map_err(|err| err.to_string())?;
            let ptr = GlobalLock(handle);
            if ptr.is_null() {
                let _ = GlobalFree(Some(handle));
                return Err("Failed to lock clipboard memory".to_string());
            }
            std::ptr::write(ptr as *mut u32, effect);
            let _ = GlobalUnlock(handle);
            let medium = STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                u: STGMEDIUM_0 { hGlobal: handle },
                pUnkForRelease: std::mem::ManuallyDrop::new(None),
            };
            // fRelease = true: the Shell object takes ownership of the HGLOBAL.
            if data_object.SetData(&format_etc, &medium, true).is_err() {
                let _ = GlobalFree(Some(handle));
                return Err("Failed to set Preferred DropEffect".to_string());
            }
        }

        // Hand the object to OLE; on success OLE keeps it alive for external readers.
        OleSetClipboard(&data_object).map_err(|err| err.to_string())?;
    }
    Ok(())
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
    let mut clip = with_clipboard(|| unsafe { read_formats() })?;
    // A remote-desktop broker (RustDesk / MS-RDPECLIP Cliprdr) puts a CF_HDROP on
    // the local clipboard whose paths point at the *far* machine — they do not
    // resolve here, so a path-based paste would silently copy nothing. Re-read
    // through the data object's CFSTR_FILEDESCRIPTOR + CFSTR_FILECONTENTS and
    // materialize the real bytes locally so paste has files to work with. This
    // only triggers when the path list is empty or holds entries absent from this
    // host, so same-machine File Explorer paste keeps its original fast path.
    if filecontents_fallback_wanted(&clip.paths) {
        if let Ok(Some(materialized)) = materialize_filecontents_via_ole() {
            if !materialized.is_empty() {
                clip.paths = materialized;
                clip.mode = Some("copy".to_string());
            }
        }
    }
    Ok(clip)
}

/// True when the CF_HDROP list cannot be pasted by path: either empty (broker
/// exposed only file contents) or containing a path that does not exist locally
/// (a far-machine path). Existing local paths are left to the path fast path.
#[cfg(windows)]
fn filecontents_fallback_wanted(paths: &[String]) -> bool {
    paths.is_empty() || paths.iter().any(|p| !std::path::Path::new(p).exists())
}

/// Pull the copied-file bytes out of the clipboard data object and write them to
/// a fresh temp directory, returning the local paths. `Ok(None)` means there is
/// no Shell file-descriptor on the clipboard, so the caller keeps its CF_HDROP
/// result untouched.
#[cfg(windows)]
fn materialize_filecontents_via_ole() -> Result<Option<Vec<String>>, String> {
    use windows::core::w;
    use windows::Win32::System::Com::{
        IDataObject, FORMATETC, STGMEDIUM, DVASPECT_CONTENT, TYMED_HGLOBAL, TYMED_ISTREAM,
    };
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows::Win32::System::Ole::{OleGetClipboard, ReleaseStgMedium};

    unsafe {
        ensure_ole();
        let descriptor_format = RegisterClipboardFormatW(w!("FileDescriptor"));
        let contents_format = RegisterClipboardFormatW(w!("FileContents"));
        if descriptor_format == 0 || contents_format == 0 {
            return Ok(None);
        }

        let object: IDataObject = match OleGetClipboard() {
            Ok(object) => object,
            Err(_) => return Ok(None),
        };

        let descriptor_etc = FORMATETC {
            cfFormat: descriptor_format as u16,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };
        let mut medium: STGMEDIUM = match object.GetData(&descriptor_etc) {
            Ok(medium) => medium,
            Err(_) => return Ok(None),
        };
        if medium.tymed & TYMED_HGLOBAL.0 as u32 == 0 {
            ReleaseStgMedium(&mut medium);
            return Ok(None);
        }
        let hglobal = medium.u.hGlobal;
        let size = GlobalSize(hglobal);
        let base = GlobalLock(hglobal) as *const u8;
        let descriptors = if base.is_null() || size < std::mem::size_of::<windows::Win32::UI::Shell::FILEGROUPDESCRIPTORW>() {
            let _ = GlobalUnlock(hglobal);
            ReleaseStgMedium(&mut medium);
            return Ok(None);
        } else {
            let bytes = std::slice::from_raw_parts(base, size);
            let parsed = read_filegroup_descriptors(bytes);
            let _ = GlobalUnlock(hglobal);
            parsed
        };
        ReleaseStgMedium(&mut medium);
        if descriptors.is_empty() {
            return Ok(None);
        }

        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_millis())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("afm-clip-{}-{}", std::process::id(), stamp));
        std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;

        let mut materialized = Vec::new();
        for (index, (name, is_directory)) in descriptors.into_iter().enumerate() {
            if is_directory || name.is_empty() {
                continue;
            }
            let contents_etc = FORMATETC {
                cfFormat: contents_format as u16,
                ptd: std::ptr::null_mut(),
                dwAspect: DVASPECT_CONTENT.0,
                lindex: index as i32,
                tymed: (TYMED_HGLOBAL.0 | TYMED_ISTREAM.0) as u32,
            };
            let mut file_medium = match object.GetData(&contents_etc) {
                Ok(medium) => medium,
                Err(_) => continue,
            };
            let destination = dir.join(&name);
            let wrote = if file_medium.tymed & TYMED_ISTREAM.0 as u32 != 0 {
                let stream = file_medium.u.pstm.as_ref().cloned();
                match stream {
                    Some(stream) => write_stream_to_file(&stream, &destination),
                    None => Err("empty file contents stream".to_string()),
                }
            } else if file_medium.tymed & TYMED_HGLOBAL.0 as u32 != 0 {
                write_hglobal_to_file(file_medium.u.hGlobal, &destination)
            } else {
                Err("unsupported file contents medium".to_string())
            };
            ReleaseStgMedium(&mut file_medium);
            if wrote.is_ok() {
                materialized.push(destination.to_string_lossy().to_string());
            }
        }
        Ok(Some(materialized))
    }
}


// No non-Windows stubs: the clipboard provider is only registered on Windows,
// and `platform::clipboard_*` handles the absent-provider case centrally
// (7.24). Keeping stubs here would be dead code with a second degradation path.

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
    use windows::core::w;
    use windows::Win32::Foundation::{GlobalFree, HANDLE};
    use windows::Win32::System::DataExchange::{
        EmptyClipboard, RegisterClipboardFormatW, SetClipboardData,
    };

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
    use windows::core::w;
    use windows::Win32::System::DataExchange::{GetClipboardData, RegisterClipboardFormatW};
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

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
                        let raw = std::slice::from_raw_parts(
                            effect_ptr as *const u8,
                            4.min(GlobalSize(effect_global)),
                        );
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

/// Parse a `FILEGROUPDESCRIPTORW` buffer (a `cItems` count followed by that many
/// contiguous `FILEDESCRIPTORW` records) into `(name, is_directory)` pairs.
/// `FILEDESCRIPTORW` is `#[repr(packed)]`, so fields are read as unaligned bytes
/// via `offset_of!` rather than borrowed (borrowing a packed field is UB).
#[cfg(windows)]
fn read_filegroup_descriptors(bytes: &[u8]) -> Vec<(String, bool)> {
    use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY;
    use windows::Win32::UI::Shell::FILEDESCRIPTORW;

    if bytes.len() < 4 {
        return Vec::new();
    }
    let c_items = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let element = std::mem::size_of::<FILEDESCRIPTORW>();
    if element == 0 {
        return Vec::new();
    }
    let base = filegroup_array_offset();
    let name_offset = std::mem::offset_of!(FILEDESCRIPTORW, cFileName);
    let attribute_offset = std::mem::offset_of!(FILEDESCRIPTORW, dwFileAttributes);
    let mut out = Vec::new();
    for index in 0..c_items {
        let multiplied = match index.checked_mul(element) {
            Some(value) => value,
            None => break,
        };
        let record = match base.checked_add(multiplied) {
            Some(value) => value,
            None => break,
        };
        let end = match record.checked_add(element) {
            Some(value) => value,
            None => break,
        };
        if end > bytes.len() {
            break;
        }
        let attribute_start = record + attribute_offset;
        let is_directory = attribute_start + 4 <= bytes.len()
            && (u32::from_le_bytes(
                bytes[attribute_start..attribute_start + 4]
                    .try_into()
                    .unwrap_or([0u8; 4]),
            ) & FILE_ATTRIBUTE_DIRECTORY.0)
                != 0;
        let name_start = record + name_offset;
        let mut units = Vec::with_capacity(260);
        let mut position = 0usize;
        while position < 260 {
            let at = name_start + position * 2;
            if at + 2 > bytes.len() {
                break;
            }
            let unit = u16::from_le_bytes([bytes[at], bytes[at + 1]]);
            if unit == 0 {
                break;
            }
            units.push(unit);
            position += 1;
        }
        out.push((String::from_utf16_lossy(&units), is_directory));
    }
    out
}

/// Byte offset of the first `FILEDESCRIPTORW` inside `FILEGROUPDESCRIPTORW`,
/// derived from the real type layout (never hardcoded across Rust versions).
#[cfg(windows)]
fn filegroup_array_offset() -> usize {
    use windows::Win32::UI::Shell::FILEGROUPDESCRIPTORW;
    std::mem::offset_of!(FILEGROUPDESCRIPTORW, fgd)
}

#[cfg(windows)]
fn write_hglobal_to_file(
    hglobal: windows::Win32::Foundation::HGLOBAL,
    destination: &std::path::Path,
) -> Result<(), String> {
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    unsafe {
        let size = GlobalSize(hglobal);
        let ptr = GlobalLock(hglobal) as *const u8;
        if ptr.is_null() {
            return Err("failed to lock file contents".to_string());
        }
        let bytes = std::slice::from_raw_parts(ptr, size);
        let result = std::fs::write(destination, bytes).map_err(|err| err.to_string());
        let _ = GlobalUnlock(hglobal);
        result
    }
}

#[cfg(windows)]
fn write_stream_to_file(
    stream: &windows::Win32::System::Com::IStream,
    destination: &std::path::Path,
) -> Result<(), String> {
    use std::ffi::c_void;
    use std::io::Write;
    let mut file = std::fs::File::create(destination).map_err(|err| err.to_string())?;
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let mut read = 0u32;
        let hr = unsafe {
            stream.Read(buffer.as_mut_ptr() as *mut c_void, buffer.len() as u32, Some(&mut read))
        };
        hr.ok().map_err(|err| err.to_string())?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read as usize])
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

// Tauri commands are neutral entry points: they call the registered provider
// through `platform/`, never the OS backend functions above (7.24).
#[tauri::command]
pub fn file_ops_clipboard_set(paths: Vec<String>, mode: String) -> Result<(), String> {
    crate::platform::clipboard_set(&paths, &mode)
}

#[tauri::command]
pub fn file_ops_clipboard_get() -> Result<ShellClipboard, String> {
    crate::platform::clipboard_get()
}

#[tauri::command]
pub fn file_ops_clipboard_clear() -> Result<(), String> {
    crate::platform::clipboard_clear()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    use windows::{
        core::{implement, HRESULT},
        Win32::Foundation::{DV_E_FORMATETC, E_NOTIMPL, E_OUTOFMEMORY, HGLOBAL, S_OK},
        Win32::System::Com::{
            FORMATETC, IAdviseSink, IDataObject, IDataObject_Impl, IEnumFORMATETC, IEnumSTATDATA,
            STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL, TYMED_ISTREAM, DVASPECT_CONTENT,
        },
        Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
    };

    #[test]
    fn hdrop_roundtrip_preserves_paths() {
        let paths = vec![r"C:\Temp\a.txt".to_string(), r"D:\文件夹\b.txt".to_string()];
        let decoded = decode_hdrop(&encode_hdrop(&paths));
        assert_eq!(decoded, paths);
    }

    #[test]
    fn cut_effect_is_move() {
        assert_eq!(mode_to_effect("cut"), DROPEFFECT_MOVE);
        assert_eq!(effect_to_mode(DROPEFFECT_MOVE), "cut");
        assert_eq!(effect_to_mode(DROPEFFECT_COPY), "copy");
    }

    #[cfg(windows)]
    fn encode_descriptor_group(files: &[(&str, u32)]) -> Vec<u8> {
        use windows::Win32::UI::Shell::FILEDESCRIPTORW;
        let element = std::mem::size_of::<FILEDESCRIPTORW>();
        let base = filegroup_array_offset();
        let name_offset = std::mem::offset_of!(FILEDESCRIPTORW, cFileName);
        let attribute_offset = std::mem::offset_of!(FILEDESCRIPTORW, dwFileAttributes);
        let mut out = vec![0u8; base + element * files.len()];
        out[0..4].copy_from_slice(&(files.len() as u32).to_le_bytes());
        for (index, (name, attributes)) in files.iter().enumerate() {
            let record = base + index * element;
            out[record + attribute_offset..record + attribute_offset + 4]
                .copy_from_slice(&attributes.to_le_bytes());
            let mut units: Vec<u16> = name.encode_utf16().collect();
            units.push(0);
            for (position, unit) in units.iter().take(260).enumerate() {
                let at = record + name_offset + position * 2;
                out[at..at + 2].copy_from_slice(&unit.to_le_bytes());
            }
        }
        out
    }

    #[cfg(windows)]
    #[test]
    fn filedescriptor_group_parses_names_and_directories() {
        let bytes = encode_descriptor_group(&[("alpha.txt", 0), ("\u{3b2} folder", 16)]);
        let parsed = read_filegroup_descriptors(&bytes);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "alpha.txt");
        assert!(!parsed[0].1);
        assert_eq!(parsed[1].0, "\u{3b2} folder");
        assert!(parsed[1].1);
    }

    #[cfg(windows)]
    #[test]
    fn filecontents_fallback_only_for_non_local_paths() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("present.txt");
        std::fs::write(&existing, b"x").unwrap();
        assert!(!filecontents_fallback_wanted(&[existing
            .to_string_lossy()
            .to_string()]));
        assert!(filecontents_fallback_wanted(&[]));
        assert!(filecontents_fallback_wanted(
            &[dir.path().join("missing.txt").to_string_lossy().to_string()]
        ));
    }

    // ------------------------------------------------------------------
    // Gated round-trip tests. They drive the REAL system clipboard, so they
    // are `#[ignore]`d by default and never clobber the user's clipboard
    // during `npm test`. `ClipboardSnapshot` restores the prior text and
    // empties the clipboard on drop. Run explicitly with:
    //   cargo test --lib file_ops::clipboard -- --ignored
    // ------------------------------------------------------------------

    /// Serializes the gated tests that drive the shared system clipboard, so a
    /// default (multi-threaded) `cargo test -- --ignored` run cannot let them
    /// clobber each other. Poisoned lock is recovered (a panicking peer test
    /// still leaves a restorable clipboard via each guard's Drop).
    #[cfg(windows)]
    static CLIPBOARD_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Best-effort clipboard restore: re-install the previously copied text (the
    /// only format we can faithfully re-serialize) and empty the rest, so a
    /// round-trip test never leaves synthetic file payloads on the clipboard.
    #[cfg(windows)]
    struct ClipboardSnapshot {
        text: Option<String>,
    }

    #[cfg(windows)]
    impl ClipboardSnapshot {
        fn capture() -> Self {
            Self { text: read_clipboard_text() }
        }
    }

    #[cfg(windows)]
    impl Drop for ClipboardSnapshot {
        fn drop(&mut self) {
            // Release our ownership first (flushes whatever the test installed),
            // then empty, then restore the prior text if there was any.
            unsafe {
                let _ = windows::Win32::System::Ole::OleFlushClipboard();
            }
            let _ = clear_shell_clipboard();
            if let Some(text) = &self.text {
                let _ = crate::clipboard::set_text(text);
            }
        }
    }

    #[cfg(windows)]
    fn read_clipboard_text() -> Option<String> {
        use windows::Win32::Foundation::HGLOBAL;
        use windows::Win32::System::DataExchange::{
            CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
        };
        use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
        const CF_UNICODETEXT: u32 = 13;
        unsafe {
            if OpenClipboard(None).is_err() {
                return None;
            }
            let result = (|| {
                if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() {
                    return None;
                }
                let handle = GetClipboardData(CF_UNICODETEXT).ok()?;
                let hglobal = HGLOBAL(handle.0);
                let size = GlobalSize(hglobal);
                let ptr = GlobalLock(hglobal) as *const u16;
                if ptr.is_null() || size == 0 {
                    let _ = GlobalUnlock(hglobal);
                    return None;
                }
                let units = std::slice::from_raw_parts(ptr, size / 2);
                let end = units.iter().position(|unit| *unit == 0).unwrap_or(units.len());
                let text = String::from_utf16_lossy(&units[..end]);
                let _ = GlobalUnlock(hglobal);
                Some(text)
            })();
            let _ = CloseClipboard();
            result
        }
    }

    /// Read `CFSTR_FILECONTENTS` for `index` off the current clipboard data
    /// object, returning the raw bytes (HGLOBAL or IStream).
    #[cfg(windows)]
    fn read_filecontents_bytes(index: i32) -> Option<Vec<u8>> {
        use windows::core::w;
        use windows::Win32::System::Com::{
            FORMATETC, IDataObject, STGMEDIUM, TYMED_HGLOBAL, TYMED_ISTREAM, DVASPECT_CONTENT,
        };
        use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
        use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
        use windows::Win32::System::Ole::{OleGetClipboard, ReleaseStgMedium};

        ensure_ole();
        let contents_format = unsafe { RegisterClipboardFormatW(w!("FileContents")) } as u16;
        if contents_format == 0 {
            return None;
        }
        // The clipboard may be briefly owned/busy right after OleSetClipboard;
        // retry like `with_clipboard` so the gated round-trip is not flaky.
        let mut object: Option<IDataObject> = None;
        for _ in 0..8 {
            if let Ok(candidate) = unsafe { OleGetClipboard() } {
                object = Some(candidate);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        let object: IDataObject = object?;
        let etc = FORMATETC {
            cfFormat: contents_format,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: index,
            tymed: (TYMED_HGLOBAL.0 | TYMED_ISTREAM.0) as u32,
        };
        let mut medium: STGMEDIUM = unsafe { object.GetData(&etc) }.ok()?;
        let bytes = if medium.tymed & TYMED_HGLOBAL.0 as u32 != 0 {
            unsafe {
                let hglobal = medium.u.hGlobal;
                let size = GlobalSize(hglobal);
                let ptr = GlobalLock(hglobal) as *const u8;
                let data = if ptr.is_null() || size == 0 {
                    None
                } else {
                    Some(std::slice::from_raw_parts(ptr, size).to_vec())
                };
                let _ = GlobalUnlock(hglobal);
                data
            }
        } else if medium.tymed & TYMED_ISTREAM.0 as u32 != 0 {
            let stream = unsafe { medium.u.pstm.as_ref() }.cloned();
            match stream {
                Some(stream) => {
                    use std::ffi::c_void;
                    let mut out = Vec::new();
                    let mut buffer = vec![0u8; 16 * 1024];
                    loop {
                        let mut read = 0u32;
                        let hr = unsafe {
                            stream
                                .Read(buffer.as_mut_ptr() as *mut c_void, buffer.len() as u32, Some(&mut read))
                        };
                        if hr.is_err() || read == 0 {
                            break;
                        }
                        out.extend_from_slice(&buffer[..read as usize]);
                    }
                    Some(out)
                }
                None => None,
            }
        } else {
            None
        };
        unsafe { ReleaseStgMedium(&mut medium) };
        bytes
    }

    /// Wrap bytes in a movable `HGLOBAL` for a returned `STGMEDIUM`.
    #[cfg(windows)]
    fn hglobal_with(bytes: &[u8]) -> windows::core::Result<HGLOBAL> {
        unsafe {
            let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1))?;
            let ptr = GlobalLock(handle) as *mut u8;
            if ptr.is_null() {
                return Err(E_OUTOFMEMORY.into());
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            let _ = GlobalUnlock(handle);
            Ok(handle)
        }
    }

    /// Synthetic clipboard data object serving ONLY `CFSTR_FILEDESCRIPTOR` +
    /// `CFSTR_FILECONTENTS` (no `CF_HDROP`), mimicking a remote-desktop broker
    /// that copied a file whose path does not resolve locally.
    #[cfg(windows)]
    #[implement(IDataObject)]
    struct RemoteContentsStub {
        descriptor_format: u16,
        contents_format: u16,
        descriptor_bytes: Vec<u8>,
        contents_bytes: Vec<u8>,
    }

    #[cfg(windows)]
    impl IDataObject_Impl for RemoteContentsStub_Impl {
        fn GetData(&self, pformatetcin: *const FORMATETC) -> windows::core::Result<STGMEDIUM> {
            let fe = unsafe { &*pformatetcin };
            if fe.tymed & TYMED_HGLOBAL.0 as u32 == 0 {
                return Err(DV_E_FORMATETC.into());
            }
            let bytes = if fe.cfFormat == self.descriptor_format {
                &self.descriptor_bytes
            } else if fe.cfFormat == self.contents_format && fe.lindex == 0 {
                &self.contents_bytes
            } else {
                return Err(DV_E_FORMATETC.into());
            };
            let handle = hglobal_with(bytes)?;
            Ok(STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                u: STGMEDIUM_0 { hGlobal: handle },
                pUnkForRelease: std::mem::ManuallyDrop::new(None),
            })
        }

        fn QueryGetData(&self, pformatetc: *const FORMATETC) -> HRESULT {
            let fe = unsafe { &*pformatetc };
            let matches = (fe.cfFormat == self.descriptor_format
                || (fe.cfFormat == self.contents_format && fe.lindex == 0))
                && fe.tymed & TYMED_HGLOBAL.0 as u32 != 0;
            if matches {
                S_OK
            } else {
                DV_E_FORMATETC
            }
        }

        fn GetDataHere(
            &self,
            _pformatetc: *const FORMATETC,
            _pmedium: *mut STGMEDIUM,
        ) -> windows::core::Result<()> {
            Err(E_NOTIMPL.into())
        }

        fn GetCanonicalFormatEtc(
            &self,
            pformatectin: *const FORMATETC,
            pformatetcout: *mut FORMATETC,
        ) -> HRESULT {
            unsafe {
                if !pformatetcout.is_null() {
                    let mut out = *pformatectin;
                    out.ptd = std::ptr::null_mut();
                    *pformatetcout = out;
                }
            }
            S_OK
        }

        fn SetData(
            &self,
            _pformatetc: *const FORMATETC,
            _pmedium: *const STGMEDIUM,
            _frelease: windows::core::BOOL,
        ) -> windows::core::Result<()> {
            Err(E_NOTIMPL.into())
        }

        fn EnumFormatEtc(&self, _dwdirection: u32) -> windows::core::Result<IEnumFORMATETC> {
            // OleSetClipboard enumerates the owner's formats; an honest list
            // (descriptor + per-file contents) is required for OLE to accept
            // the object. Everything else stays E_NOTIMPL.
            let formats = [
                FORMATETC {
                    cfFormat: self.descriptor_format,
                    ptd: std::ptr::null_mut(),
                    dwAspect: DVASPECT_CONTENT.0,
                    lindex: -1,
                    tymed: TYMED_HGLOBAL.0 as u32,
                },
                FORMATETC {
                    cfFormat: self.contents_format,
                    ptd: std::ptr::null_mut(),
                    dwAspect: DVASPECT_CONTENT.0,
                    lindex: 0,
                    tymed: (TYMED_HGLOBAL.0 | TYMED_ISTREAM.0) as u32,
                },
            ];
            unsafe { windows::Win32::UI::Shell::SHCreateStdEnumFmtEtc(&formats) }
        }

        fn DAdvise(
            &self,
            _pformatetc: *const FORMATETC,
            _advf: u32,
            _padvsink: windows::core::Ref<IAdviseSink>,
        ) -> windows::core::Result<u32> {
            Err(E_NOTIMPL.into())
        }

        fn DUnadvise(&self, _dwconnection: u32) -> windows::core::Result<()> {
            Err(E_NOTIMPL.into())
        }

        fn EnumDAdvise(&self) -> windows::core::Result<IEnumSTATDATA> {
            Err(E_NOTIMPL.into())
        }
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "drives the real system clipboard; run with --ignored"]
    fn gated_read_side_materializes_remote_file_contents() {
        use windows::core::w;
        use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
        use windows::Win32::System::Ole::OleSetClipboard;

        let _serial = CLIPBOARD_TEST_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let _guard = ClipboardSnapshot::capture();
        ensure_ole();

        let descriptor_format = unsafe { RegisterClipboardFormatW(w!("FileDescriptor")) } as u16;
        let contents_format = unsafe { RegisterClipboardFormatW(w!("FileContents")) } as u16;
        assert_ne!(descriptor_format, 0, "FileDescriptor format should register");
        assert_ne!(contents_format, 0, "FileContents format should register");

        // UTF-8 payload + a file name the far machine "copied"; no CF_HDROP, so
        // the read side cannot resolve a path and must materialize the contents.
        let payload: Vec<u8> = "远程剪贴板-bytes-\u{4e00}\u{4e8c}".as_bytes().to_vec();
        let descriptor = encode_descriptor_group(&[("afm-gated-rt.txt", 0u32)]);
        let stub = RemoteContentsStub {
            descriptor_format,
            contents_format,
            descriptor_bytes: descriptor,
            contents_bytes: payload.clone(),
        };
        let object: IDataObject = stub.into();
        unsafe { OleSetClipboard(&object) }.expect("OleSetClipboard");

        let clip = get_shell_clipboard().expect("get_shell_clipboard");
        assert_eq!(clip.paths.len(), 1, "expected one materialized file");
        assert_eq!(clip.mode.as_deref(), Some("copy"));
        let path = std::path::Path::new(&clip.paths[0]);
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("afm-gated-rt.txt")
        );
        assert_eq!(std::fs::read(path).expect("materialized file readable"), payload);
        let temp_parent = path.parent().map(|value| value.to_path_buf());

        drop(object);
        if let Some(parent) = temp_parent {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "drives the real system clipboard; run with --ignored"]
    fn gated_write_side_publishes_readable_file_contents() {
        let _serial = CLIPBOARD_TEST_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let _guard = ClipboardSnapshot::capture();
        ensure_ole();

        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("publish-me.txt");
        let payload = b"published-by-afm".to_vec();
        std::fs::write(&source, &payload).expect("write source");

        set_shell_clipboard(&[source.to_string_lossy().to_string()], "copy")
            .expect("set_shell_clipboard");

        // The exact contract a remote broker depends on: CFSTR_FILECONTENTS for
        // file 0 carries the real bytes, not just a path string.
        let contents = read_filecontents_bytes(0).expect("CFSTR_FILECONTENTS readable");
        assert_eq!(contents, payload);
    }
}
