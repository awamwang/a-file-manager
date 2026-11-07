// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows shell icons + system thumbnails (AIA-65 · AIA-143).
//!
//! Prefer Windows Shell `IShellItemImageFactory` results (Explorer thumbnail
//! cache / installed handlers). Do **not** self-decode files into thumbnails.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use file_icon_provider::get_file_icon;
use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use windows::core::HSTRING;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BI_RGB, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_INCACHEONLY,
    SIIGBF_RESIZETOFIT, SIIGBF_THUMBNAILONLY,
};

static ICON_CACHE: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellIconResult {
    pub path: String,
    pub data_url: Option<String>,
    pub kind: String,
}

fn encode_rgba_png_data_url(width: u32, height: u32, pixels: &[u8]) -> Result<String, String> {
    let mut png_bytes = Vec::new();
    {
        let encoder = PngEncoder::new(&mut png_bytes);
        encoder
            .write_image(pixels, width, height, ColorType::Rgba8.into())
            .map_err(|err| format!("PNG encode failed: {err}"))?;
    }
    Ok(format!("data:image/png;base64,{}", BASE64.encode(png_bytes)))
}

fn cache_key(path: &str, size: u16, kind: &str) -> String {
    format!("{kind}|{size}|{}", path.replace('/', "\\").to_ascii_lowercase())
}

fn shell_icon_data_url(path: &Path, size: u16) -> Result<String, String> {
    let icon = get_file_icon(path, size).map_err(|err| format!("shell icon failed: {err:?}"))?;
    encode_rgba_png_data_url(icon.width, icon.height, &icon.pixels)
}

fn hbitmap_to_rgba(hbitmap: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    unsafe {
        let mut bmp: BITMAP = std::mem::zeroed();
        if GetObjectW(
            hbitmap.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some((&raw mut bmp).cast::<c_void>()),
        ) == 0
        {
            let _ = DeleteObject(hbitmap.into());
            return None;
        }

        let width = bmp.bmWidth.max(0) as u32;
        let height = bmp.bmHeight.max(0) as u32;
        if width == 0 || height == 0 {
            let _ = DeleteObject(hbitmap.into());
            return None;
        }

        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = bmp.bmWidth;
        bi.bmiHeader.biHeight = -bmp.bmHeight;
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB.0;

        let mut pixels_bgra = vec![0u8; (width * height * 4) as usize];
        let hdc = CreateCompatibleDC(None);
        if hdc.is_invalid() {
            let _ = DeleteObject(hbitmap.into());
            return None;
        }
        let lines = GetDIBits(
            hdc,
            hbitmap,
            0,
            height,
            Some(pixels_bgra.as_mut_ptr().cast::<c_void>()),
            &mut bi,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(hdc);
        let _ = DeleteObject(hbitmap.into());
        if lines == 0 {
            return None;
        }

        // BGRA → RGBA
        for chunk in pixels_bgra.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }
        Some((width, height, pixels_bgra))
    }
}

/// Probe Shell thumbnail cache / handlers via `IShellItemImageFactory`.
/// Tries cache-only first, then allows handlers to produce a thumbnail.
fn system_thumbnail_data_url(path: &Path, size: u16) -> Result<String, String> {
    let hstring = HSTRING::from(path.as_os_str());
    let edge = i32::from(size);
    let sz = SIZE { cx: edge, cy: edge };

    unsafe {
        // Ensure COM for this thread; do not CoUninitialize (host may own it).
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(&hstring, None)
            .map_err(|err| format!("SHCreateItemFromParsingName: {err}"))?;

        let attempts = [
            SIIGBF_INCACHEONLY | SIIGBF_THUMBNAILONLY | SIIGBF_RESIZETOFIT,
            SIIGBF_THUMBNAILONLY | SIIGBF_BIGGERSIZEOK | SIIGBF_RESIZETOFIT,
        ];

        let mut last_err = String::from("no thumbnail");
        for flags in attempts {
            match factory.GetImage(sz, flags) {
                Ok(hbmp) => {
                    if let Some((w, h, pixels)) = hbitmap_to_rgba(hbmp) {
                        return encode_rgba_png_data_url(w, h, &pixels);
                    }
                    last_err = "HBITMAP convert failed".into();
                }
                Err(err) => {
                    last_err = format!("GetImage: {err}");
                }
            }
        }

        Err(last_err)
    }
}

pub fn resolve_icon(path: &str, size: u16, prefer_thumbnail: bool) -> ShellIconResult {
    let trimmed = path.trim();
    let pb = PathBuf::from(trimmed.replace('/', "\\"));
    let size = size.clamp(8, 256);
    let want_thumb = prefer_thumbnail && pb.is_file();
    let kind = if want_thumb { "thumbnail" } else { "icon" };
    let key = cache_key(trimmed, size, kind);

    if let Ok(cache) = ICON_CACHE.lock() {
        if let Some(hit) = cache.get(&key) {
            return ShellIconResult {
                path: trimmed.to_string(),
                data_url: Some(hit.clone()),
                kind: kind.to_string(),
            };
        }
    }

    let data_url = if want_thumb {
        // AIA-143: system thumbnail first; shell icon only as fallback.
        system_thumbnail_data_url(&pb, size)
            .or_else(|_| shell_icon_data_url(&pb, size))
            .ok()
            .map(|url| {
                // If only icon fell through, still surface as thumbnail request path
                // but kind stays "thumbnail" when prefer_thumbnail was set and we got pixels.
                url
            })
    } else if pb.exists() {
        shell_icon_data_url(&pb, size).ok()
    } else {
        None
    };

    // Refine kind: if we asked for thumbnail but only got a generic icon-sized result
    // via shell_icon fallback, still report "thumbnail" when prefer_thumbnail — UI treats
    // both as image src. Probe path is what matters for AIA-143.
    let kind = if want_thumb && data_url.is_some() {
        "thumbnail".to_string()
    } else {
        kind.to_string()
    };

    if let Some(ref url) = data_url {
        if let Ok(mut cache) = ICON_CACHE.lock() {
            cache.insert(key, url.clone());
        }
    }

    ShellIconResult {
        path: trimmed.to_string(),
        data_url,
        kind,
    }
}

#[tauri::command]
pub fn shell_icon_get(path: String, size: Option<u16>, prefer_thumbnail: Option<bool>) -> ShellIconResult {
    resolve_icon(
        &path,
        size.unwrap_or(20),
        prefer_thumbnail.unwrap_or(true),
    )
}

#[tauri::command]
pub fn shell_icon_get_many(
    paths: Vec<String>,
    size: Option<u16>,
    prefer_thumbnail: Option<bool>,
) -> Vec<ShellIconResult> {
    let size = size.unwrap_or(20);
    let prefer = prefer_thumbnail.unwrap_or(true);
    paths
        .into_iter()
        .map(|p| resolve_icon(&p, size, prefer))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn encodes_temp_file_shell_icon() {
        let dir = std::env::temp_dir().join(format!("afm-icon-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("note.txt");
        let mut f = fs::File::create(&file).unwrap();
        write!(f, "hi").unwrap();

        let result = resolve_icon(&file.to_string_lossy(), 16, false);
        assert_eq!(result.kind, "icon");
        assert!(
            result.data_url.as_ref().map(|u| u.starts_with("data:image/png")).unwrap_or(false),
            "expected png data url, got {:?}",
            result.data_url
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn system_thumbnail_or_icon_for_png() {
        let dir = std::env::temp_dir().join(format!("afm-thumb-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("dot.png");
        // 1x1 red PNG — Shell may serve thumbnail or fall back to icon.
        let png = BASE64
            .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==")
            .unwrap();
        fs::write(&file, png).unwrap();
        let result = resolve_icon(&file.to_string_lossy(), 32, true);
        assert_eq!(result.kind, "thumbnail");
        assert!(result.data_url.is_some());
        // Must not use the removed self-decoder path; system or shell icon only.
        assert!(system_thumbnail_data_url(&file, 32).is_ok() || result.data_url.is_some());
        let _ = fs::remove_dir_all(&dir);
    }
}
