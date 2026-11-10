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
use std::collections::{HashMap, VecDeque};
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

/// Bound process-side Shell results so extreme folders cannot retain every PNG data URL (AIA-279).
const MAX_ICON_CACHE_ENTRIES: usize = 384;
/// Max worker threads inside one `shell_icon_get_many` batch (COM STA per worker).
const SHELL_ICON_PARALLELISM: usize = 4;

struct BoundedIconCache {
    map: HashMap<String, String>,
    order: VecDeque<String>,
}

impl BoundedIconCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get(&self, key: &str) -> Option<&String> {
        self.map.get(key)
    }

    fn insert(&mut self, key: String, value: String) {
        if self.map.contains_key(&key) {
            self.map.insert(key.clone(), value);
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
            self.order.push_back(key);
            return;
        }
        while self.map.len() >= MAX_ICON_CACHE_ENTRIES {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            } else {
                break;
            }
        }
        self.order.push_back(key.clone());
        self.map.insert(key, value);
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.map.len()
    }

    #[cfg(test)]
    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

static ICON_CACHE: Lazy<Mutex<BoundedIconCache>> =
    Lazy::new(|| Mutex::new(BoundedIconCache::new()));

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
    format!(
        "{kind}|{size}|{}",
        crate::paths::normalize_path_for_comparison(path)
    )
}

/// Shared shell-icon key for the same file type (extension / dir), not per-path.
fn type_icon_cache_key(path: &Path, size: u16) -> String {
    if path.is_dir() {
        return format!("icon|{size}|type:dir");
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if !ext.is_empty() => {
            format!("icon|{size}|type:{}", ext.to_ascii_lowercase())
        }
        _ => format!("icon|{size}|type:none"),
    }
}

fn ensure_com_sta() {
    // Ensure COM for this thread; do not CoUninitialize (host may own it).
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
}

fn shell_icon_data_url(path: &Path, size: u16) -> Result<String, String> {
    ensure_com_sta();
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
        ensure_com_sta();

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
    let path_key = cache_key(trimmed, size, kind);
    let type_key = type_icon_cache_key(&pb, size);

    if let Ok(cache) = ICON_CACHE.lock() {
        if let Some(hit) = cache.get(&path_key) {
            return ShellIconResult {
                path: trimmed.to_string(),
                data_url: Some(hit.clone()),
                kind: kind.to_string(),
            };
        }
        // Shell icons (and thumb→icon fallbacks) share one glyph per file type.
        if !want_thumb {
            if let Some(hit) = cache.get(&type_key) {
                return ShellIconResult {
                    path: trimmed.to_string(),
                    data_url: Some(hit.clone()),
                    kind: "icon".to_string(),
                };
            }
        }
    }

    // real_thumb: pixels came from Shell thumbnail handlers (not association icon).
    let (data_url, real_thumb) = if want_thumb {
        // AIA-143: system thumbnail first; shell icon only as fallback.
        match system_thumbnail_data_url(&pb, size) {
            Ok(url) => (Some(url), true),
            Err(_) => {
                if let Ok(cache) = ICON_CACHE.lock() {
                    if let Some(hit) = cache.get(&type_key) {
                        return ShellIconResult {
                            path: trimmed.to_string(),
                            data_url: Some(hit.clone()),
                            kind: "thumbnail".to_string(),
                        };
                    }
                }
                (shell_icon_data_url(&pb, size).ok(), false)
            }
        }
    } else if pb.exists() {
        (shell_icon_data_url(&pb, size).ok(), false)
    } else {
        (None, false)
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
            cache.insert(path_key, url.clone());
            // True thumbnails stay path-scoped; association icons also fill the type key.
            if !real_thumb {
                cache.insert(type_key, url.clone());
            }
        }
    }

    ShellIconResult {
        path: trimmed.to_string(),
        data_url,
        kind,
    }
}

/// Resolve a batch with bounded parallelism; each worker initializes its own COM STA.
fn resolve_icons_parallel(paths: Vec<String>, size: u16, prefer_thumbnail: bool) -> Vec<ShellIconResult> {
    let n = paths.len();
    if n == 0 {
        return Vec::new();
    }
    let workers = SHELL_ICON_PARALLELISM.min(n).max(1);
    if workers == 1 {
        return paths
            .into_iter()
            .map(|p| resolve_icon(&p, size, prefer_thumbnail))
            .collect();
    }

    let mut slots: Vec<Option<ShellIconResult>> = (0..n).map(|_| None).collect();
    let chunk_size = n.div_ceil(workers);

    std::thread::scope(|scope| {
        let mut rest = slots.as_mut_slice();
        let mut offset = 0usize;
        for _ in 0..workers {
            if offset >= n {
                break;
            }
            let end = (offset + chunk_size).min(n);
            let take = end - offset;
            let (this_slots, next) = rest.split_at_mut(take);
            rest = next;
            let chunk_paths: Vec<String> = paths[offset..end].to_vec();
            offset = end;
            scope.spawn(move || {
                ensure_com_sta();
                for (i, p) in chunk_paths.into_iter().enumerate() {
                    this_slots[i] = Some(resolve_icon(&p, size, prefer_thumbnail));
                }
            });
        }
    });

    slots
        .into_iter()
        .map(|slot| {
            slot.unwrap_or(ShellIconResult {
                path: String::new(),
                data_url: None,
                kind: "icon".into(),
            })
        })
        .collect()
}

#[tauri::command]
pub async fn shell_icon_get(
    path: String,
    size: Option<u16>,
    prefer_thumbnail: Option<bool>,
) -> ShellIconResult {
    let size = size.unwrap_or(20);
    let prefer = prefer_thumbnail.unwrap_or(true);
    // Shell thumbnail / icon work is COM + disk; keep it off the async runtime workers.
    let fallback_path = path.clone();
    tauri::async_runtime::spawn_blocking(move || resolve_icon(&path, size, prefer))
        .await
        .unwrap_or(ShellIconResult {
            path: fallback_path,
            data_url: None,
            kind: "icon".into(),
        })
}

#[tauri::command]
pub async fn shell_icon_get_many(
    paths: Vec<String>,
    size: Option<u16>,
    prefer_thumbnail: Option<bool>,
) -> Vec<ShellIconResult> {
    let size = size.unwrap_or(20);
    let prefer = prefer_thumbnail.unwrap_or(true);
    let n = paths.len();
    let t0 = std::time::Instant::now();
    // AIA-278: small frontend batches + spawn_blocking so list IPC / UI stay responsive
    // while Shell generates thumbnails. Inside the blocking task, resolve with
    // SHELL_ICON_PARALLELISM workers (each thread owns its COM STA).
    let results = tauri::async_runtime::spawn_blocking(move || {
        resolve_icons_parallel(paths, size, prefer)
    })
    .await
    .unwrap_or_default();
    crate::perf_log::afm_perf_line(format!(
        "AFM_PERF shell_icon_batch n={} size={} thumb={} {:.1}ms",
        n,
        size,
        if prefer { 1 } else { 0 },
        t0.elapsed().as_secs_f64() * 1000.0
    ));
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn bounded_icon_cache_evicts_oldest() {
        let mut cache = BoundedIconCache::new();
        for i in 0..(MAX_ICON_CACHE_ENTRIES + 10) {
            cache.insert(format!("k{i}"), format!("v{i}"));
        }
        assert_eq!(cache.len(), MAX_ICON_CACHE_ENTRIES);
        assert!(cache.get("k0").is_none(), "oldest entries must be evicted");
        assert!(cache.get(&format!("k{}", MAX_ICON_CACHE_ENTRIES + 9)).is_some());
    }

    #[test]
    fn type_icon_cache_reuses_same_extension() {
        let dir = std::env::temp_dir().join(format!("afm-type-icon-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        fs::write(&a, "a").unwrap();
        fs::write(&b, "b").unwrap();
        if let Ok(mut cache) = ICON_CACHE.lock() {
            cache.clear();
        }
        let first = resolve_icon(&a.to_string_lossy(), 16, false);
        assert!(first.data_url.is_some());
        let type_key = type_icon_cache_key(&a, 16);
        {
            let cache = ICON_CACHE.lock().unwrap();
            assert!(cache.get(&type_key).is_some(), "type cache must fill after shell icon");
        }
        let second = resolve_icon(&b.to_string_lossy(), 16, false);
        assert_eq!(first.data_url, second.data_url);
        let _ = fs::remove_dir_all(&dir);
    }

    /// Manual perf probe: `AFM_PERF_DIR=<dir> cargo test --lib shell_icons::tests::bench_shell_icon_batch -- --nocapture`
    #[test]
    fn bench_shell_icon_batch() {
        let Ok(dir) = std::env::var("AFM_PERF_DIR") else {
            return;
        };
        let paths: Vec<String> = fs::read_dir(&dir)
            .expect("AFM_PERF_DIR")
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_string_lossy().to_string())
            .take(40)
            .collect();
        assert!(paths.len() >= 8, "need at least 8 files in AFM_PERF_DIR");

        let mut cold_icon = Vec::new();
        for _ in 0..5 {
            if let Ok(mut cache) = ICON_CACHE.lock() {
                cache.clear();
            }
            let batch: Vec<String> = paths.iter().take(40).cloned().collect();
            let t0 = std::time::Instant::now();
            let _ = resolve_icons_parallel(batch, 20, false);
            cold_icon.push(t0.elapsed().as_secs_f64() * 1000.0);
        }
        cold_icon.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let warm_t0 = std::time::Instant::now();
        let _ = resolve_icons_parallel(paths.iter().take(40).cloned().collect(), 20, false);
        let warm_icon_ms = warm_t0.elapsed().as_secs_f64() * 1000.0;
        eprintln!(
            "AFM_PERF shell_icon_batch n=40 thumb=0 cold_samples_ms={cold_icon:?} cold_median_ms={:.1} warm_ms={warm_icon_ms:.1}",
            cold_icon[cold_icon.len() / 2]
        );

        let thumb_paths: Vec<String> = paths.iter().take(8).cloned().collect();
        let mut cold_thumb = Vec::new();
        for _ in 0..5 {
            if let Ok(mut cache) = ICON_CACHE.lock() {
                cache.clear();
            }
            let t0 = std::time::Instant::now();
            let _ = resolve_icons_parallel(thumb_paths.clone(), 96, true);
            cold_thumb.push(t0.elapsed().as_secs_f64() * 1000.0);
        }
        cold_thumb.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let warm_t0 = std::time::Instant::now();
        let _ = resolve_icons_parallel(thumb_paths, 96, true);
        let warm_thumb_ms = warm_t0.elapsed().as_secs_f64() * 1000.0;
        eprintln!(
            "AFM_PERF shell_icon_batch n=8 thumb=1 cold_samples_ms={cold_thumb:?} cold_median_ms={:.1} warm_ms={warm_thumb_ms:.1}",
            cold_thumb[cold_thumb.len() / 2]
        );
    }

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
