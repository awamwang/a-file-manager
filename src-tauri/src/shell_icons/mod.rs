// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows shell icons + light image thumbnails (AIA-65).

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use file_icon_provider::get_file_icon;
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{ColorType, ImageEncoder};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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

fn is_image_ext(path: &Path) -> bool {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico") => true,
        _ => false,
    }
}

fn image_thumbnail_data_url(path: &Path, max_edge: u32) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|err| format!("read image failed: {err}"))?;
    let img = image::load_from_memory(&bytes).map_err(|err| format!("decode image failed: {err}"))?;
    let thumb = img.resize(max_edge, max_edge, FilterType::Triangle).into_rgba8();
    encode_rgba_png_data_url(thumb.width(), thumb.height(), thumb.as_raw())
}

pub fn resolve_icon(path: &str, size: u16, prefer_thumbnail: bool) -> ShellIconResult {
    let trimmed = path.trim();
    let pb = PathBuf::from(trimmed.replace('/', "\\"));
    let size = size.clamp(8, 256);
    let want_thumb = prefer_thumbnail && pb.is_file() && is_image_ext(&pb);
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
        image_thumbnail_data_url(&pb, u32::from(size))
            .or_else(|_| shell_icon_data_url(&pb, size))
            .ok()
    } else if pb.exists() {
        shell_icon_data_url(&pb, size).ok()
    } else {
        None
    };

    if let Some(ref url) = data_url {
        if let Ok(mut cache) = ICON_CACHE.lock() {
            cache.insert(key, url.clone());
        }
    }

    ShellIconResult {
        path: trimmed.to_string(),
        data_url,
        kind: kind.to_string(),
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
    fn thumbnail_for_png() {
        let dir = std::env::temp_dir().join(format!("afm-thumb-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("dot.png");
        // 1x1 red PNG
        let png = BASE64
            .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==")
            .unwrap();
        fs::write(&file, png).unwrap();
        let result = resolve_icon(&file.to_string_lossy(), 32, true);
        assert_eq!(result.kind, "thumbnail");
        assert!(result.data_url.is_some());
        let _ = fs::remove_dir_all(&dir);
    }
}
